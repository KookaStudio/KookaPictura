//! Classic TIFF and BigTIFF IFD reading for the DNG reader: typed tag values,
//! either byte order, every offset and count bounded by the file.

use crate::error::PsdError;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Order {
    Little,
    Big,
}

impl Order {
    pub(super) fn u16(self, b: [u8; 2]) -> u16 {
        match self {
            Order::Little => u16::from_le_bytes(b),
            Order::Big => u16::from_be_bytes(b),
        }
    }

    fn u32(self, b: [u8; 4]) -> u32 {
        match self {
            Order::Little => u32::from_le_bytes(b),
            Order::Big => u32::from_be_bytes(b),
        }
    }

    fn u64(self, b: [u8; 8]) -> u64 {
        match self {
            Order::Little => u64::from_le_bytes(b),
            Order::Big => u64::from_be_bytes(b),
        }
    }
}

pub(super) struct Entry {
    pub tag: u16,
    typ: u16,
    count: usize,
    /// Absolute position of the value bytes, inline or not.
    pub at: usize,
}

pub(super) struct Ifd {
    pub entries: Vec<Entry>,
    pub next: u64,
}

impl Ifd {
    pub fn get(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|e| e.tag == tag)
    }
}

pub(super) struct Tiff<'a> {
    bytes: &'a [u8],
    pub order: Order,
    big: bool,
}

fn type_size(typ: u16) -> Option<usize> {
    Some(match typ {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => return None,
    })
}

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("DNG: {what}"))
}

impl<'a> Tiff<'a> {
    /// Parses the header; returns the reader and the first IFD's offset.
    pub fn parse(bytes: &'a [u8]) -> Result<(Self, u64), PsdError> {
        if bytes.len() < 8 {
            return Err(PsdError::Truncated);
        }
        let order = match &bytes[..2] {
            b"II" => Order::Little,
            b"MM" => Order::Big,
            _ => return Err(invalid("not a TIFF container")),
        };
        match order.u16([bytes[2], bytes[3]]) {
            42 => {
                let tiff = Self {
                    bytes,
                    order,
                    big: false,
                };
                let first = tiff.u32_at(4)? as u64;
                Ok((tiff, first))
            }
            43 => {
                if bytes.len() < 16 {
                    return Err(PsdError::Truncated);
                }
                if order.u16([bytes[4], bytes[5]]) != 8 || order.u16([bytes[6], bytes[7]]) != 0 {
                    return Err(invalid("BigTIFF offset size"));
                }
                let tiff = Self {
                    bytes,
                    order,
                    big: true,
                };
                let first = tiff.u64_at(8)?;
                Ok((tiff, first))
            }
            _ => Err(invalid("TIFF magic")),
        }
    }

    /// A classic TIFF reader over bytes without a standard header: a vendor
    /// magic, or a maker note whose offsets count from its own start.
    pub fn view(bytes: &'a [u8], order: Order) -> Self {
        Self {
            bytes,
            order,
            big: false,
        }
    }

    /// The first IFD's offset, from the classic header's last four bytes.
    pub fn first_ifd(&self) -> Result<u64, PsdError> {
        self.u32_at(4).map(u64::from)
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    fn slice(&self, at: usize, len: usize) -> Result<&'a [u8], PsdError> {
        at.checked_add(len)
            .and_then(|end| self.bytes.get(at..end))
            .ok_or(PsdError::Truncated)
    }

    fn u16_at(&self, at: usize) -> Result<u16, PsdError> {
        let s = self.slice(at, 2)?;
        Ok(self.order.u16([s[0], s[1]]))
    }

    fn u32_at(&self, at: usize) -> Result<u32, PsdError> {
        let s = self.slice(at, 4)?;
        Ok(self.order.u32([s[0], s[1], s[2], s[3]]))
    }

    fn u64_at(&self, at: usize) -> Result<u64, PsdError> {
        let s = self.slice(at, 8)?;
        Ok(self
            .order
            .u64([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
    }

    /// The position of `tag`'s entry in the IFD at `offset`, reading only its
    /// entry table, so a file prefix answers even when the values lie beyond it.
    fn find_entry(&self, offset: u64, tag: u16) -> Option<usize> {
        let at = usize::try_from(offset).ok()?;
        let (count, first, entry_len) = if self.big {
            let n = usize::try_from(self.u64_at(at).ok()?).ok()?;
            (n, at + 8, 20)
        } else {
            (self.u16_at(at).ok()? as usize, at + 2, 12)
        };
        (0..count)
            .map(|i| first + i * entry_len)
            .map_while(|p| self.u16_at(p).ok().map(|t| (p, t)))
            .find(|&(_, t)| t == tag)
            .map(|(p, _)| p)
    }

    /// Whether the IFD at `offset` lists `tag`, reading only its entry table.
    pub fn lists_tag(&self, offset: u64, tag: u16) -> bool {
        self.find_entry(offset, tag).is_some()
    }

    /// The ASCII value of `tag` in the IFD at `offset`, reading only that
    /// entry and its value; `None` when either lies beyond the bytes.
    pub fn ascii_in(&self, offset: u64, tag: u16) -> Option<String> {
        let p = self.find_entry(offset, tag)?;
        if self.u16_at(p + 2).ok()? != 2 {
            return None;
        }
        let (count, value_at, inline) = if self.big {
            (usize::try_from(self.u64_at(p + 4).ok()?).ok()?, p + 12, 8)
        } else {
            (self.u32_at(p + 4).ok()? as usize, p + 8, 4)
        };
        let at = if count <= inline {
            value_at
        } else if self.big {
            usize::try_from(self.u64_at(value_at).ok()?).ok()?
        } else {
            self.u32_at(value_at).ok()? as usize
        };
        let raw = self.slice(at, count).ok()?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        Some(String::from_utf8_lossy(&raw[..end]).trim().to_string())
    }

    pub fn ifd(&self, offset: u64) -> Result<Ifd, PsdError> {
        self.directory(offset, false)
    }

    /// A maker note's directory, skipping entries whose values lie outside
    /// the file: some bodies (E-P1, E-PL1) record such junk entries.
    pub fn ifd_skipping_strays(&self, offset: u64) -> Result<Ifd, PsdError> {
        self.directory(offset, true)
    }

    fn directory(&self, offset: u64, skip_strays: bool) -> Result<Ifd, PsdError> {
        let at = usize::try_from(offset).map_err(|_| PsdError::Truncated)?;
        let (count, first, entry_len, inline) = if self.big {
            let n = usize::try_from(self.u64_at(at)?).map_err(|_| PsdError::Truncated)?;
            (n, at + 8, 20, 8)
        } else {
            (self.u16_at(at)? as usize, at + 2, 12, 4)
        };
        let table = count.checked_mul(entry_len).ok_or(PsdError::Truncated)?;
        self.slice(first, table + if self.big { 8 } else { 4 })?;
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let p = first + i * entry_len;
            let tag = self.u16_at(p)?;
            let typ = self.u16_at(p + 2)?;
            let (count, value_at) = if self.big {
                (self.u64_at(p + 4)?, p + 12)
            } else {
                (self.u32_at(p + 4)? as u64, p + 8)
            };
            let Some(size) = type_size(typ) else {
                continue;
            };
            let count = usize::try_from(count).map_err(|_| PsdError::Truncated)?;
            let len = count.checked_mul(size).ok_or(PsdError::Truncated)?;
            let at = if len <= inline {
                value_at
            } else if self.big {
                usize::try_from(self.u64_at(value_at)?).map_err(|_| PsdError::Truncated)?
            } else {
                self.u32_at(value_at)? as usize
            };
            match self.slice(at, len) {
                Err(_) if skip_strays => continue,
                checked => checked?,
            };
            entries.push(Entry {
                tag,
                typ,
                count,
                at,
            });
        }
        let next_at = first + table;
        let next = if self.big {
            self.u64_at(next_at)?
        } else {
            self.u32_at(next_at)? as u64
        };
        Ok(Ifd { entries, next })
    }

    /// Unsigned integer values (BYTE, SHORT, LONG, LONG8, IFD, IFD8).
    pub fn uints(&self, e: &Entry) -> Result<Vec<u64>, PsdError> {
        (0..e.count)
            .map(|i| match e.typ {
                1 | 7 => Ok(self.bytes[e.at + i] as u64),
                3 => self.u16_at(e.at + 2 * i).map(u64::from),
                4 | 13 => self.u32_at(e.at + 4 * i).map(u64::from),
                16 | 18 => self.u64_at(e.at + 8 * i),
                _ => Err(invalid(&format!(
                    "tag {} is not an unsigned integer",
                    e.tag
                ))),
            })
            .collect()
    }

    /// 16-bit words: SHORT values, or BYTE and UNDEFINED bytes taken in pairs
    /// in the file's byte order (some Canon bodies store their colour data
    /// so). An odd byte count is refused, not truncated.
    pub fn words(&self, e: &Entry) -> Result<Vec<u16>, PsdError> {
        match e.typ {
            3 => (0..e.count).map(|i| self.u16_at(e.at + 2 * i)).collect(),
            1 | 7 if !e.count.is_multiple_of(2) => Err(invalid(&format!(
                "tag {} has an odd byte count for 16-bit words",
                e.tag
            ))),
            1 | 7 => Ok(self
                .raw(e)
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&b| self.order.u16(b))
                .collect()),
            _ => Err(invalid(&format!("tag {} is not 16-bit words", e.tag))),
        }
    }

    pub fn uint(&self, e: &Entry) -> Result<u64, PsdError> {
        self.uints(e)?
            .first()
            .copied()
            .ok_or_else(|| invalid(&format!("tag {} is empty", e.tag)))
    }

    /// Any numeric type as `f64`.
    pub fn reals(&self, e: &Entry) -> Result<Vec<f64>, PsdError> {
        (0..e.count)
            .map(|i| {
                Ok(match e.typ {
                    1 | 7 => self.bytes[e.at + i] as f64,
                    6 => self.bytes[e.at + i] as i8 as f64,
                    3 => self.u16_at(e.at + 2 * i)? as f64,
                    8 => self.u16_at(e.at + 2 * i)? as i16 as f64,
                    4 | 13 => self.u32_at(e.at + 4 * i)? as f64,
                    9 => self.u32_at(e.at + 4 * i)? as i32 as f64,
                    11 => f32::from_bits(self.u32_at(e.at + 4 * i)?) as f64,
                    12 => f64::from_bits(self.u64_at(e.at + 8 * i)?),
                    16 | 18 => self.u64_at(e.at + 8 * i)? as f64,
                    17 => self.u64_at(e.at + 8 * i)? as i64 as f64,
                    5 | 10 => {
                        let n = self.u32_at(e.at + 8 * i)?;
                        let d = self.u32_at(e.at + 8 * i + 4)?;
                        let (n, d) = if e.typ == 10 {
                            (n as i32 as f64, d as i32 as f64)
                        } else {
                            (n as f64, d as f64)
                        };
                        if d == 0.0 {
                            0.0
                        } else {
                            n / d
                        }
                    }
                    _ => return Err(invalid(&format!("tag {} is not numeric", e.tag))),
                })
            })
            .collect()
    }

    pub fn raw(&self, e: &Entry) -> &'a [u8] {
        let len = e.count * type_size(e.typ).unwrap_or(1);
        &self.bytes[e.at..e.at + len]
    }

    pub fn ascii(&self, e: &Entry) -> Option<String> {
        let raw = self.raw(e);
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let text = String::from_utf8_lossy(&raw[..end]).trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}
