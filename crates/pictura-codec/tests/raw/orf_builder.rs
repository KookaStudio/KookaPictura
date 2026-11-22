//! Lays out synthetic ORFs: the sensor strips, then IFD0, Exif and a maker
//! note in each layout the cameras write.

/// The maker note's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// `OLYMPUS\0II`: offsets from the note, ImageProcessing by pointer.
    Olympus,
    /// `OM SYSTEM`: the same, with a longer header.
    OmSystem,
    /// `OLYMP\0`: offsets from the file, ImageProcessing as the value.
    OlympValue,
    /// `OLYMP\0` with the levels in RawInfo.
    OlympRawInfo,
    /// `OLYMP\0` with only the main note's tags.
    OlympMain,
}

/// How the samples are stored.
#[derive(Debug, Clone)]
pub enum Data {
    /// Left-aligned by the shift, in file order.
    Words(u32),
    PaddedLsb,
    MsbWords,
    Msb,
    /// The strips as given.
    Strips(Vec<Vec<u8>>),
}

#[derive(Debug, Clone)]
pub struct Orf {
    pub magic: &'static [u8; 4],
    pub width: usize,
    pub height: usize,
    pub samples: Vec<u16>,
    pub data: Data,
    pub rows_per_strip: usize,
    pub make: &'static str,
    pub model: &'static str,
    pub orientation: u16,
    pub cfa: [u8; 4],
    pub note: Note,
    pub bits: u16,
    pub black: [u16; 4],
    pub white: Option<i16>,
    pub crop: Option<[u16; 4]>,
    pub as_shot: Option<[u16; 2]>,
    /// The 6600 K balance, or the fine-weather one in RawInfo.
    pub daylight: Option<[u16; 2]>,
    pub matrix: [i16; 9],
}

impl Orf {
    pub fn new(width: usize, height: usize, samples: Vec<u16>, data: Data) -> Self {
        let magic = match data {
            Data::Msb => b"IIRS",
            _ => b"IIRO",
        };
        Self {
            magic,
            width,
            height,
            samples,
            data,
            rows_per_strip: height,
            make: "OLYMPUS CORPORATION",
            model: "E-300",
            orientation: 1,
            cfa: [0, 1, 1, 2],
            note: Note::Olympus,
            bits: 12,
            black: [60, 62, 64, 66],
            white: None,
            crop: None,
            as_shot: Some([512, 384]),
            daylight: Some([554, 464]),
            matrix: [404, -148, 0, -64, 388, -68, -2, -96, 354],
        }
    }
}

enum Value {
    Short(Vec<u16>),
    SShort(Vec<i16>),
    Long(Vec<u32>),
    Ascii(&'static str),
    Bytes(Vec<u8>),
    /// A directory named by an IFD pointer.
    Pointer(Vec<(u16, Value)>),
    /// A directory stored as the value.
    Embedded(Vec<(u16, Value)>),
    Note(Note, Vec<(u16, Value)>),
}

struct Writer {
    out: Vec<u8>,
    be: bool,
}

impl Writer {
    fn u16(&self, v: u16) -> [u8; 2] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }

    fn u32(&self, v: u32) -> [u8; 4] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }

    /// Writes a directory and then its values, whose offsets count from
    /// `base`; returns the directory's position.
    fn directory(&mut self, mut tags: Vec<(u16, Value)>, base: usize) -> usize {
        tags.sort_by_key(|(tag, _)| *tag);
        let at = self.out.len();
        let count = self.u16(tags.len() as u16);
        self.out.extend(count);
        self.out.resize(at + 2 + 12 * tags.len() + 4, 0);
        for (i, (tag, value)) in tags.into_iter().enumerate() {
            let start = self.out.len();
            let (typ, count, inline): (u16, usize, Option<Vec<u8>>) = match value {
                Value::Short(v) => (
                    3,
                    v.len(),
                    Some(v.iter().flat_map(|&x| self.u16(x)).collect()),
                ),
                Value::SShort(v) => {
                    let bytes = v.iter().flat_map(|&x| self.u16(x as u16)).collect();
                    (8, v.len(), Some(bytes))
                }
                Value::Long(v) => (
                    4,
                    v.len(),
                    Some(v.iter().flat_map(|&x| self.u32(x)).collect()),
                ),
                Value::Ascii(s) => (2, s.len() + 1, Some([s.as_bytes(), &[0]].concat())),
                Value::Bytes(v) => (7, v.len(), Some(v)),
                Value::Pointer(tags) => {
                    let child = self.directory(tags, base);
                    let typ = if base == 0 { 4 } else { 13 };
                    (typ, 1, Some(self.u32((child - base) as u32).to_vec()))
                }
                Value::Embedded(tags) => {
                    self.directory(tags, base);
                    (7, self.out.len() - start, None)
                }
                Value::Note(note, tags) => {
                    let header: &[u8] = match note {
                        Note::Olympus => b"OLYMPUS\0II\x03\0",
                        Note::OmSystem => b"OM SYSTEM\0\0\0II\x04\0",
                        _ => b"OLYMP\0\x02\0",
                    };
                    self.out.extend(header);
                    let relative = matches!(note, Note::Olympus | Note::OmSystem);
                    self.directory(tags, if relative { start } else { 0 });
                    (7, self.out.len() - start, None)
                }
            };
            let field = match inline {
                Some(mut bytes) if bytes.len() <= 4 => {
                    bytes.resize(4, 0);
                    bytes
                }
                Some(bytes) => {
                    let value_at = self.out.len();
                    self.out.extend(bytes);
                    self.u32((value_at - base) as u32).to_vec()
                }
                None => self.u32((start - base) as u32).to_vec(),
            };
            let (tag, typ, count) = (self.u16(tag), self.u16(typ), self.u32(count as u32));
            let entry = [&tag[..], &typ[..], &count[..], &field[..]].concat();
            self.out[at + 2 + 12 * i..][..12].copy_from_slice(&entry);
        }
        at
    }
}

fn strips(o: &Orf, be: bool) -> Vec<Vec<u8>> {
    if let Data::Strips(strips) = &o.data {
        return strips.clone();
    }
    let rows: Vec<&[u16]> = o.samples.chunks(o.width).collect();
    let fields: Vec<Vec<&[u16]>> = match o.data {
        // The even rows, then the odd, each field in strips of its own.
        Data::Msb => [0, 1]
            .map(|field| rows.iter().skip(field).step_by(2).copied().collect())
            .to_vec(),
        _ => vec![rows],
    };
    fields
        .iter()
        .flat_map(|rows| rows.chunks(o.rows_per_strip))
        .map(|rows| {
            rows.iter()
                .flat_map(|row| encode_row(&o.data, row, be))
                .collect()
        })
        .collect()
}

fn encode_row(data: &Data, row: &[u16], be: bool) -> Vec<u8> {
    let msb = || -> Vec<u8> {
        row.chunks(2)
            .flat_map(|p| {
                [
                    (p[0] >> 4) as u8,
                    ((p[0] & 0xF) << 4 | p[1] >> 8) as u8,
                    p[1] as u8,
                ]
            })
            .collect()
    };
    match *data {
        Data::Words(shift) => row
            .iter()
            .flat_map(|&v| {
                let v = v << shift;
                if be {
                    v.to_be_bytes()
                } else {
                    v.to_le_bytes()
                }
            })
            .collect(),
        Data::Msb => msb(),
        Data::MsbWords => msb()
            .chunks(4)
            .flat_map(|w| [w[3], w[2], w[1], w[0]])
            .collect(),
        Data::PaddedLsb => row
            .chunks(10)
            .flat_map(|group| {
                let mut out: Vec<u8> = group
                    .chunks(2)
                    .flat_map(|p| {
                        [
                            p[0] as u8,
                            (p[0] >> 8 | (p[1] & 0xF) << 4) as u8,
                            (p[1] >> 4) as u8,
                        ]
                    })
                    .collect();
                out.push(0);
                out
            })
            .collect(),
        Data::Strips(_) => unreachable!("written as given"),
    }
}

fn maker_note(o: &Orf) -> Vec<(u16, Value)> {
    let matrix = || Value::Short(o.matrix.iter().map(|&v| v as u16).collect());
    if o.note == Note::OlympMain {
        let mut main = vec![
            (0x1011, matrix()),
            (0x1012, Value::Short(o.black.to_vec())),
            (0x102C, Value::Short(vec![o.bits, 0])),
        ];
        if let Some([r, b]) = o.as_shot {
            main.push((0x1017, Value::Short(vec![r, 64])));
            main.push((0x1018, Value::Short(vec![b, 64])));
        }
        return main;
    }
    let mut dir = vec![
        (0x0200, matrix()),
        (0x0600, Value::Short(o.black.to_vec())),
        (0x0611, Value::Short(vec![o.bits, 0])),
    ];
    let raw_info = o.note == Note::OlympRawInfo;
    if let Some([x, y, w, h]) = o.crop {
        let pair = |v| Value::Short(if raw_info { vec![v] } else { vec![v, 0] });
        dir.push((0x0612, pair(x)));
        dir.push((0x0613, pair(y)));
        dir.push((0x0614, Value::Long(vec![w.into()])));
        dir.push((0x0615, Value::Long(vec![h.into()])));
    }
    if let Some([r, b]) = o.as_shot {
        let levels = if raw_info {
            vec![r, b]
        } else {
            vec![r, b, 256, 256]
        };
        dir.push((0x0100, Value::Short(levels)));
    }
    if let Some([r, b]) = o.daylight {
        dir.push((
            if raw_info { 0x0122 } else { 0x010C },
            Value::Short(vec![r, b]),
        ));
    }
    match o.note {
        Note::OlympRawInfo => vec![(0x3000, Value::Embedded(dir))],
        Note::OlympValue => vec![(0x2040, Value::Embedded(dir))],
        _ => {
            if let Some(white) = o.white {
                dir.push((0x0805, Value::SShort(vec![white, 640])));
            }
            vec![(0x2040, Value::Pointer(dir))]
        }
    }
}

pub fn orf(o: &Orf) -> Vec<u8> {
    let be = o.magic.starts_with(b"MM");
    let mut w = Writer {
        out: [&o.magic[..], &[0; 4]].concat(),
        be,
    };
    let (mut offsets, mut counts) = (Vec::new(), Vec::new());
    for strip in strips(o, be) {
        offsets.push(w.out.len() as u32);
        counts.push(strip.len() as u32);
        w.out.extend(strip);
    }
    let cfa = [&w.u16(2)[..], &w.u16(2)[..], &o.cfa[..]].concat();
    let exif = vec![
        (0x927C, Value::Note(o.note, maker_note(o))),
        (0xA302, Value::Bytes(cfa)),
    ];
    let ifd0 = vec![
        (256, Value::Long(vec![o.width as u32])),
        (257, Value::Long(vec![o.height as u32])),
        // The cameras write these whatever the coding.
        (258, Value::Short(vec![16])),
        (259, Value::Short(vec![1])),
        (271, Value::Ascii(o.make)),
        (272, Value::Ascii(o.model)),
        (273, Value::Long(offsets)),
        (274, Value::Short(vec![o.orientation])),
        (278, Value::Long(vec![o.rows_per_strip as u32])),
        (279, Value::Long(counts)),
        (34665, Value::Pointer(exif)),
    ];
    let at = w.directory(ifd0, 0);
    let first = w.u32(at as u32);
    w.out[4..8].copy_from_slice(&first);
    w.out
}
