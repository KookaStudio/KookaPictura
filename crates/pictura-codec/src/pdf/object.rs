//! PDF object model and serializer (ISO 32000-1 §7.3): the direct objects, a
//! numbered object table, and the per-object encryption hook strings and
//! streams pass through.

use std::fmt::Write as _;

/// A direct PDF object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Obj {
    Bool(bool),
    Int(i64),
    Real(f64),
    Name(String),
    /// A string's raw bytes; text goes through [`Obj::text`].
    Str(Vec<u8>),
    Array(Vec<Obj>),
    Dict(Dict),
    Ref(u32),
}

impl Obj {
    pub(crate) fn name(name: &str) -> Obj {
        Obj::Name(name.to_string())
    }

    /// A text string: PDFDocEncoding-compatible ASCII as is, anything else as
    /// UTF-16BE with a byte-order mark.
    pub(crate) fn text(text: &str) -> Obj {
        if text.bytes().all(|b| (0x20..0x7f).contains(&b)) {
            Obj::Str(text.as_bytes().to_vec())
        } else {
            let mut bytes = vec![0xfe, 0xff];
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            Obj::Str(bytes)
        }
    }

    pub(crate) fn reals(values: &[f64]) -> Obj {
        Obj::Array(values.iter().map(|&v| Obj::Real(v)).collect())
    }
}

/// An ordered dictionary (key order is kept so output is deterministic).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Dict(pub(crate) Vec<(String, Obj)>);

impl Dict {
    pub(crate) fn new() -> Dict {
        Dict(Vec::new())
    }

    pub(crate) fn typed(kind: &str) -> Dict {
        Dict::new().with("Type", Obj::name(kind))
    }

    pub(crate) fn with(mut self, key: &str, value: Obj) -> Dict {
        self.set(key, value);
        self
    }

    pub(crate) fn set(&mut self, key: &str, value: Obj) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key.to_string(), value)),
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Obj> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

impl From<Dict> for Obj {
    fn from(dict: Dict) -> Obj {
        Obj::Dict(dict)
    }
}

/// An indirect object's body.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Body {
    Obj(Obj),
    /// A stream: its dictionary (`Length` is added on output) and its
    /// already-filtered data.
    Stream(Dict, Vec<u8>),
}

/// Encrypts one object's strings and stream data; `None` writes them plain.
pub(crate) type Cipher<'a> = &'a dyn Fn(u32, &[u8]) -> Vec<u8>;

/// The numbered indirect objects of one file (object `n` is `slots[n - 1]`).
#[derive(Debug, Default)]
pub(crate) struct Objects {
    slots: Vec<Option<Body>>,
    /// Objects never encrypted: the `/Encrypt` dictionary, and the metadata
    /// stream when the metadata stays plain.
    plain: Vec<u32>,
}

impl Objects {
    /// Reserve the next object number, to be filled with [`Objects::set`].
    pub(crate) fn reserve(&mut self) -> u32 {
        self.slots.push(None);
        self.slots.len() as u32
    }

    pub(crate) fn add(&mut self, body: Body) -> u32 {
        self.slots.push(Some(body));
        self.slots.len() as u32
    }

    pub(crate) fn set(&mut self, num: u32, body: Body) {
        self.slots[num as usize - 1] = Some(body);
    }

    pub(crate) fn mark_plain(&mut self, num: u32) {
        self.plain.push(num);
    }

    pub(crate) fn is_plain(&self, num: u32) -> bool {
        self.plain.contains(&num)
    }

    pub(crate) fn count(&self) -> u32 {
        self.slots.len() as u32
    }

    pub(crate) fn body(&self, num: u32) -> &Body {
        self.slots[num as usize - 1]
            .as_ref()
            .expect("every reserved object is filled before output")
    }

    /// `n 0 obj … endobj` for object `num`, encrypted through `cipher` unless
    /// it is marked plain.
    pub(crate) fn serialize(&self, num: u32, cipher: Option<Cipher>) -> Vec<u8> {
        let cipher = if self.plain.contains(&num) {
            None
        } else {
            cipher
        };
        let mut out = format!("{num} 0 obj\n").into_bytes();
        match self.body(num) {
            Body::Obj(obj) => write_obj(&mut out, obj, num, cipher),
            Body::Stream(dict, data) => {
                let data = match cipher {
                    Some(encrypt) => encrypt(num, data),
                    None => data.clone(),
                };
                let dict = dict.clone().with("Length", Obj::Int(data.len() as i64));
                write_obj(&mut out, &Obj::Dict(dict), num, cipher);
                out.extend_from_slice(b"\nstream\n");
                out.extend_from_slice(&data);
                out.extend_from_slice(b"\nendstream");
            }
        }
        out.extend_from_slice(b"\nendobj\n");
        out
    }
}

/// Serialize `obj` (belonging to object `num`) into `out`.
pub(crate) fn write_obj(out: &mut Vec<u8>, obj: &Obj, num: u32, cipher: Option<Cipher>) {
    match obj {
        Obj::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Obj::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
        Obj::Real(r) => out.extend_from_slice(real(*r).as_bytes()),
        Obj::Name(name) => write_name(out, name),
        Obj::Str(bytes) => match cipher {
            Some(encrypt) => write_hex(out, &encrypt(num, bytes)),
            None => write_literal(out, bytes),
        },
        Obj::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                write_obj(out, item, num, cipher);
            }
            out.push(b']');
        }
        Obj::Dict(dict) => {
            out.extend_from_slice(b"<<");
            for (key, value) in &dict.0 {
                write_name(out, key);
                out.push(b' ');
                write_obj(out, value, num, cipher);
            }
            out.extend_from_slice(b">>");
        }
        Obj::Ref(target) => out.extend_from_slice(format!("{target} 0 R").as_bytes()),
    }
}

/// A real with at most four decimals and no exponent (PDF has none).
pub(crate) fn real(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" || text.is_empty() {
        "0".into()
    } else {
        text.to_string()
    }
}

fn write_name(out: &mut Vec<u8>, name: &str) {
    out.push(b'/');
    for &b in name.as_bytes() {
        let regular = (0x21..=0x7e).contains(&b) && !b"#()<>[]{}/%".contains(&b);
        if regular {
            out.push(b);
        } else {
            out.extend_from_slice(format!("#{b:02X}").as_bytes());
        }
    }
}

fn write_literal(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'(');
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            0x20..=0x7e => out.push(b),
            _ => out.extend_from_slice(format!("\\{b:03o}").as_bytes()),
        }
    }
    out.push(b')');
}

pub(crate) fn write_hex(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'<');
    let mut text = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(text, "{b:02X}");
    }
    out.extend_from_slice(text.as_bytes());
    out.push(b'>');
}

/// A classic cross-reference section for objects `first..first + offsets.len()`
/// (plus the free head entry when `first` is 1).
pub(crate) fn xref_section(first: u32, offsets: &[usize]) -> Vec<u8> {
    let mut out = Vec::new();
    if first == 1 {
        out.extend_from_slice(format!("xref\n0 {}\n", offsets.len() + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f\r\n");
    } else {
        out.extend_from_slice(format!("xref\n{first} {}\n", offsets.len()).as_bytes());
    }
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(obj: &Obj) -> String {
        let mut out = Vec::new();
        write_obj(&mut out, obj, 1, None);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn objects_serialize_with_escapes() {
        assert_eq!(text(&Obj::name("A B#")), "/A#20B#23");
        assert_eq!(text(&Obj::Str(b"a(b)\\\n".to_vec())), "(a\\(b\\)\\\\\\012)");
        assert_eq!(text(&Obj::Real(1.5)), "1.5");
        assert_eq!(text(&Obj::Real(72.0)), "72");
        assert_eq!(text(&Obj::Real(-0.00001)), "0");
        let dict = Dict::typed("Page").with("Count", Obj::Int(1));
        assert_eq!(text(&dict.into()), "<</Type /Page/Count 1>>");
        assert_eq!(
            Obj::text("é"),
            Obj::Str(vec![0xfe, 0xff, 0x00, 0xe9]),
            "non-ASCII text is UTF-16BE"
        );
    }

    #[test]
    fn encrypted_strings_are_hex_and_plain_objects_skip_the_cipher() {
        let mut objects = Objects::default();
        let a = objects.add(Body::Obj(Obj::Str(b"hi".to_vec())));
        let b = objects.add(Body::Stream(Dict::new(), b"data".to_vec()));
        objects.mark_plain(b);
        let flip = |_: u32, bytes: &[u8]| bytes.iter().map(|b| b ^ 0xff).collect();
        let out = String::from_utf8_lossy(&objects.serialize(a, Some(&flip))).into_owned();
        assert_eq!(out, "1 0 obj\n<9796>\nendobj\n");
        let out = objects.serialize(b, Some(&flip));
        assert!(
            out.windows(4).any(|w| w == b"data"),
            "a plain stream is written as is"
        );
    }

    #[test]
    fn xref_entries_are_twenty_bytes() {
        let section = xref_section(1, &[15, 1234]);
        let text = String::from_utf8(section).unwrap();
        assert!(text.starts_with("xref\n0 3\n0000000000 65535 f\r\n"));
        assert!(text.ends_with("0000001234 00000 n\r\n"));
        assert_eq!(text.len(), "xref\n0 3\n".len() + 3 * 20);
    }
}
