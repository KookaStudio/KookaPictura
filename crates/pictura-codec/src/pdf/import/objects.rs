//! A minimal PDF object reader for Images mode (design D4).
//!
//! Objects are found by scanning the file for `N G obj` headers in file order,
//! so a damaged or missing cross-reference table is no obstacle; a later
//! definition replaces an earlier one, as incremental updates append. Object
//! streams are expanded. Stream data is kept as stored (still filtered): the
//! images are copied, not decoded.

use std::collections::HashMap;
use std::io::Read as _;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Object {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    Name(Vec<u8>),
    Str(Vec<u8>),
    Array(Vec<Object>),
    Dict(Dict),
    Ref(u32),
    Stream(Dict, Vec<u8>),
}

/// A dictionary in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Dict(pub Vec<(Vec<u8>, Object)>);

impl Dict {
    pub(crate) fn get(&self, key: &str) -> Option<&Object> {
        self.0
            .iter()
            .find(|(k, _)| k == key.as_bytes())
            .map(|(_, v)| v)
    }

    pub(crate) fn set(&mut self, key: &str, value: Object) {
        match self.0.iter_mut().find(|(k, _)| k == key.as_bytes()) {
            Some(entry) => entry.1 = value,
            None => self.0.push((key.as_bytes().to_vec(), value)),
        }
    }
}

impl Object {
    pub(crate) fn dict(&self) -> Option<&Dict> {
        match self {
            Object::Dict(d) | Object::Stream(d, _) => Some(d),
            _ => None,
        }
    }

    pub(crate) fn is_name(&self, name: &str) -> bool {
        matches!(self, Object::Name(n) if n == name.as_bytes())
    }

    pub(crate) fn as_int(&self) -> Option<i64> {
        match self {
            Object::Int(i) => Some(*i),
            Object::Real(r) => Some(*r as i64),
            _ => None,
        }
    }
}

/// Every object of one PDF file and its trailer.
pub(crate) struct PdfObjects {
    objects: HashMap<u32, Object>,
    pub trailer: Dict,
}

impl PdfObjects {
    pub(crate) fn parse(bytes: &[u8]) -> Option<PdfObjects> {
        let mut objects = HashMap::new();
        let mut trailer = Dict::default();
        let mut at = 0;
        while at < bytes.len() {
            if let Some((num, end)) = object_header(bytes, at) {
                let mut lexer = Lexer { bytes, at: end };
                if let Some(object) = lexer.object_body() {
                    if let Object::Stream(dict, _) = &object {
                        if dict.get("Type").is_some_and(|t| t.is_name("XRef")) {
                            merge_trailer(&mut trailer, dict);
                        }
                    }
                    objects.insert(num, object);
                    at = lexer.at;
                    continue;
                }
            }
            if bytes[at..].starts_with(b"trailer") {
                let mut lexer = Lexer {
                    bytes,
                    at: at + b"trailer".len(),
                };
                if let Some(Object::Dict(dict)) = lexer.value() {
                    merge_trailer(&mut trailer, &dict);
                    at = lexer.at;
                    continue;
                }
            }
            at += 1;
        }
        let mut file = PdfObjects { objects, trailer };
        file.expand_object_streams();
        (!file.objects.is_empty()).then_some(file)
    }

    pub(crate) fn encrypted(&self) -> bool {
        self.trailer.get("Encrypt").is_some()
    }

    pub(crate) fn get(&self, num: u32) -> Option<&Object> {
        self.objects.get(&num)
    }

    /// `object` with one level of indirection followed.
    pub(crate) fn resolve<'a>(&'a self, object: &'a Object) -> &'a Object {
        match object {
            Object::Ref(num) => self.get(*num).unwrap_or(&Object::Null),
            other => other,
        }
    }

    pub(crate) fn resolve_dict<'a>(&'a self, object: Option<&'a Object>) -> Option<&'a Dict> {
        object.map(|o| self.resolve(o)).and_then(Object::dict)
    }

    fn expand_object_streams(&mut self) {
        if self.encrypted() {
            return;
        }
        let streams: Vec<(Dict, Vec<u8>)> = self
            .objects
            .values()
            .filter_map(|object| match object {
                Object::Stream(dict, data)
                    if dict.get("Type").is_some_and(|t| t.is_name("ObjStm")) =>
                {
                    Some((dict.clone(), data.clone()))
                }
                _ => None,
            })
            .collect();
        for (dict, data) in streams {
            let Some(data) = decode_stream(self, &dict, &data) else {
                continue;
            };
            let count = dict.get("N").and_then(Object::as_int).unwrap_or(0).max(0) as usize;
            let first = dict
                .get("First")
                .and_then(Object::as_int)
                .unwrap_or(0)
                .max(0) as usize;
            let mut header = Lexer {
                bytes: &data,
                at: 0,
            };
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                match (header.value(), header.value()) {
                    (Some(Object::Int(num)), Some(Object::Int(offset))) => {
                        entries.push((num as u32, offset as usize))
                    }
                    _ => break,
                }
            }
            for (num, offset) in entries {
                let mut lexer = Lexer {
                    bytes: &data,
                    at: first + offset,
                };
                if let Some(object) = lexer.value() {
                    // A direct definition elsewhere (an incremental update)
                    // takes precedence over the compressed one.
                    self.objects.entry(num).or_insert(object);
                }
            }
        }
    }
}

fn merge_trailer(trailer: &mut Dict, from: &Dict) {
    for key in ["Root", "Encrypt", "Info"] {
        if let Some(value) = from.get(key) {
            trailer.set(key, value.clone());
        }
    }
}

/// Unfilter a stream: no filter or `FlateDecode` (with a PNG predictor);
/// anything else is `None`.
pub(crate) fn decode_stream(file: &PdfObjects, dict: &Dict, data: &[u8]) -> Option<Vec<u8>> {
    let filter = dict.get("Filter").map(|f| file.resolve(f));
    let flate = match filter {
        None => false,
        Some(Object::Name(n)) if n == b"FlateDecode" => true,
        Some(Object::Array(a)) if a.is_empty() => false,
        Some(Object::Array(a)) if a.len() == 1 && a[0].is_name("FlateDecode") => true,
        _ => return None,
    };
    if !flate {
        return Some(data.to_vec());
    }
    let mut out = Vec::new();
    // Truncated streams are common; keep what inflated.
    let _ = flate2::read::ZlibDecoder::new(data).read_to_end(&mut out);
    let parms = file.resolve_dict(dict.get("DecodeParms"));
    let predictor = parms
        .and_then(|p| p.get("Predictor"))
        .and_then(Object::as_int)
        .unwrap_or(1);
    if predictor < 10 {
        return Some(out);
    }
    let columns = parms
        .and_then(|p| p.get("Columns"))
        .and_then(Object::as_int)
        .unwrap_or(1)
        .max(1) as usize;
    let mut rows = Vec::with_capacity(out.len());
    let mut prior = vec![0u8; columns];
    for line in out.chunks(columns + 1) {
        if line.len() < columns + 1 {
            break;
        }
        let mut row = line[1..].to_vec();
        for x in 0..columns {
            let a = if x > 0 { row[x - 1] } else { 0 };
            row[x] = row[x].wrapping_add(match line[0] {
                1 => a,
                2 => prior[x],
                3 => ((u16::from(a) + u16::from(prior[x])) / 2) as u8,
                4 => {
                    let c = if x > 0 { prior[x - 1] } else { 0 };
                    let p = i16::from(a) + i16::from(prior[x]) - i16::from(c);
                    let (pa, pb, pc) = (
                        (p - i16::from(a)).abs(),
                        (p - i16::from(prior[x])).abs(),
                        (p - i16::from(c)).abs(),
                    );
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        prior[x]
                    } else {
                        c
                    }
                }
                _ => 0,
            });
        }
        rows.extend_from_slice(&row);
        prior = row;
    }
    Some(rows)
}

/// `N G obj` starting exactly at `at` (after a delimiter): the object number
/// and the offset past `obj`.
fn object_header(bytes: &[u8], at: usize) -> Option<(u32, usize)> {
    if at > 0 && !is_space(bytes[at - 1]) && !is_delimiter(bytes[at - 1]) {
        return None;
    }
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let n = digits(at);
    if n == 0 || n > 10 {
        return None;
    }
    let num: u32 = std::str::from_utf8(&bytes[at..at + n]).ok()?.parse().ok()?;
    let mut i = at + n;
    let spaces = |from: usize| bytes[from..].iter().take_while(|b| is_space(**b)).count();
    let s = spaces(i);
    if s == 0 {
        return None;
    }
    i += s;
    let g = digits(i);
    if g == 0 || g > 5 {
        return None;
    }
    i += g;
    let s = spaces(i);
    if s == 0 {
        return None;
    }
    i += s;
    if !bytes[i..].starts_with(b"obj") {
        return None;
    }
    Some((num, i + 3))
}

fn is_space(b: u8) -> bool {
    matches!(b, b'\0' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

struct Lexer<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Lexer<'_> {
    fn skip_space(&mut self) {
        while let Some(&b) = self.bytes.get(self.at) {
            if is_space(b) {
                self.at += 1;
            } else if b == b'%' {
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| *b != b'\n' && *b != b'\r')
                {
                    self.at += 1;
                }
            } else {
                break;
            }
        }
    }

    fn keyword(&mut self) -> &[u8] {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| !is_space(*b) && !is_delimiter(*b))
        {
            self.at += 1;
        }
        &self.bytes[start..self.at]
    }

    /// The value after `obj`, with its stream data when it has one.
    fn object_body(&mut self) -> Option<Object> {
        let value = self.value()?;
        let save = self.at;
        self.skip_space();
        if !self.bytes[self.at..].starts_with(b"stream") {
            self.at = save;
            return Some(value);
        }
        let Object::Dict(dict) = value else {
            return None;
        };
        self.at += b"stream".len();
        if self.bytes[self.at..].starts_with(b"\r\n") {
            self.at += 2;
        } else if self.bytes.get(self.at) == Some(&b'\n') || self.bytes.get(self.at) == Some(&b'\r')
        {
            self.at += 1;
        }
        let start = self.at;
        let declared = match dict.get("Length") {
            Some(Object::Int(n)) if *n >= 0 => Some(*n as usize),
            _ => None,
        };
        let end = declared
            .filter(|n| {
                let end = start + n;
                let mut probe = Lexer {
                    bytes: self.bytes,
                    at: end,
                };
                probe.skip_space();
                end <= self.bytes.len() && self.bytes[probe.at..].starts_with(b"endstream")
            })
            .map(|n| start + n)
            .or_else(|| {
                let found = self.bytes[start..]
                    .windows(9)
                    .position(|w| w == b"endstream")?;
                let mut end = start + found;
                if end > start && self.bytes[end - 1] == b'\n' {
                    end -= 1;
                }
                if end > start && self.bytes[end - 1] == b'\r' {
                    end -= 1;
                }
                Some(end)
            })?;
        let data = self.bytes[start..end].to_vec();
        self.at = end;
        self.skip_space();
        if self.bytes[self.at..].starts_with(b"endstream") {
            self.at += b"endstream".len();
        }
        Some(Object::Stream(dict, data))
    }

    fn value(&mut self) -> Option<Object> {
        self.skip_space();
        let b = *self.bytes.get(self.at)?;
        match b {
            b'/' => {
                self.at += 1;
                let raw = self.keyword().to_vec();
                Some(Object::Name(unescape_name(&raw)))
            }
            b'(' => self.literal_string(),
            b'<' if self.bytes.get(self.at + 1) == Some(&b'<') => {
                self.at += 2;
                let mut dict = Dict::default();
                loop {
                    self.skip_space();
                    if self.bytes[self.at..].starts_with(b">>") {
                        self.at += 2;
                        return Some(Object::Dict(dict));
                    }
                    let Object::Name(key) = self.value()? else {
                        return None;
                    };
                    let value = self.value()?;
                    dict.0.push((key, value));
                }
            }
            b'<' => {
                self.at += 1;
                let mut digits = Vec::new();
                loop {
                    let c = *self.bytes.get(self.at)?;
                    self.at += 1;
                    if c == b'>' {
                        break;
                    }
                    if c.is_ascii_hexdigit() {
                        digits.push(c);
                    }
                }
                if digits.len() % 2 == 1 {
                    digits.push(b'0');
                }
                let bytes = digits
                    .chunks(2)
                    .map(|pair| {
                        u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16)
                            .unwrap_or(0)
                    })
                    .collect();
                Some(Object::Str(bytes))
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    if self.bytes.get(self.at) == Some(&b']') {
                        self.at += 1;
                        return Some(Object::Array(items));
                    }
                    items.push(self.value()?);
                }
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.number_or_ref(),
            _ => {
                let word = self.keyword();
                match word {
                    b"true" => Some(Object::Bool(true)),
                    b"false" => Some(Object::Bool(false)),
                    b"null" => Some(Object::Null),
                    _ => None,
                }
            }
        }
    }

    fn number_or_ref(&mut self) -> Option<Object> {
        let word = std::str::from_utf8(self.keyword()).ok()?.to_string();
        let Ok(int) = word.parse::<i64>() else {
            return word.parse::<f64>().ok().map(Object::Real);
        };
        // `N G R` is a reference; otherwise rewind to the integer.
        let save = self.at;
        self.skip_space();
        let generation = std::str::from_utf8(self.keyword())
            .ok()
            .and_then(|w| w.parse::<u32>().ok());
        if generation.is_some() {
            self.skip_space();
            if self.keyword() == b"R" && int >= 0 {
                return Some(Object::Ref(int as u32));
            }
        }
        self.at = save;
        Some(Object::Int(int))
    }

    fn literal_string(&mut self) -> Option<Object> {
        self.at += 1;
        let mut out = Vec::new();
        let mut depth = 1;
        loop {
            let c = *self.bytes.get(self.at)?;
            self.at += 1;
            match c {
                b'(' => {
                    depth += 1;
                    out.push(c);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Object::Str(out));
                    }
                    out.push(c);
                }
                b'\\' => {
                    let e = *self.bytes.get(self.at)?;
                    self.at += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'\r' => {
                            if self.bytes.get(self.at) == Some(&b'\n') {
                                self.at += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut value = u32::from(e - b'0');
                            for _ in 0..2 {
                                match self.bytes.get(self.at) {
                                    Some(d @ b'0'..=b'7') => {
                                        value = value * 8 + u32::from(d - b'0');
                                        self.at += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(value as u8);
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
    }
}

fn unescape_name(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'#' && i + 2 < raw.len() {
            let hex = std::str::from_utf8(&raw[i + 1..i + 3]).unwrap_or("");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    out
}

/// Serialize `object` (the writer half used for the one-image wrapper PDFs).
pub(crate) fn write_object(out: &mut Vec<u8>, object: &Object) {
    match object {
        Object::Null => out.extend_from_slice(b"null"),
        Object::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Object::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
        Object::Real(r) => out.extend_from_slice(super::super::object::real(*r).as_bytes()),
        Object::Name(n) => {
            out.push(b'/');
            for &b in n {
                if b.is_ascii_graphic() && !is_delimiter(b) && b != b'#' {
                    out.push(b);
                } else {
                    out.extend_from_slice(format!("#{b:02X}").as_bytes());
                }
            }
        }
        Object::Str(s) => super::super::object::write_hex(out, s),
        Object::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                write_object(out, item);
            }
            out.push(b']');
        }
        Object::Dict(dict) => write_dict(out, dict),
        Object::Ref(num) => out.extend_from_slice(format!("{num} 0 R").as_bytes()),
        Object::Stream(dict, data) => {
            let mut dict = dict.clone();
            dict.set("Length", Object::Int(data.len() as i64));
            write_dict(out, &dict);
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(data);
            out.extend_from_slice(b"\nendstream");
        }
    }
}

fn write_dict(out: &mut Vec<u8>, dict: &Dict) {
    out.extend_from_slice(b"<<");
    for (key, value) in &dict.0 {
        write_object(out, &Object::Name(key.clone()));
        out.push(b' ');
        write_object(out, value);
        out.push(b' ');
    }
    out.extend_from_slice(b">>");
}
