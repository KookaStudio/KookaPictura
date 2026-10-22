//! A minimal PDF object reader for Images mode (design D4).
//!
//! Objects are found by scanning the file for `N G obj` headers in file order,
//! so a damaged or missing cross-reference table is no obstacle; a later
//! definition replaces an earlier one, as incremental updates append. Object
//! streams are expanded. Stream data is kept as stored (still filtered): the
//! images are copied, not decoded. An encrypted file stays encrypted until
//! `decrypt` opens it with a password (#627).

use std::cell::Cell;
use std::collections::HashMap;

use crate::budget::read_bounded;
use crate::probe::ImageBudget;

/// The deepest array/dictionary nesting the lexer follows.
pub(crate) const MAX_NESTING: u32 = 256;
/// The widest PNG-predictor row (`/Columns`, in bytes here) a stream may declare.
const MAX_PREDICTOR_COLUMNS: usize = 1 << 24;
/// The most components (`/Colors`) a predictor row may declare (PDF caps 32).
const MAX_PREDICTOR_COLORS: usize = 32;

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
    /// Each object's generation number, which its decryption key includes.
    generations: HashMap<u32, u16>,
    pub trailer: Dict,
    /// The most bytes one stream may inflate to.
    pub(crate) stream_limit: u64,
    /// Inflated bytes the whole file may still produce.
    pub(crate) inflate_budget: Cell<u64>,
}

/// What `PdfObjects::decrypt` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unlock {
    /// Not encrypted, or now decrypted.
    Open,
    WrongPassword,
    /// A security handler other than the standard one.
    Unsupported,
}

impl PdfObjects {
    pub(crate) fn parse(bytes: &[u8]) -> Option<PdfObjects> {
        let mut objects = HashMap::new();
        let mut generations = HashMap::new();
        let mut trailer = Dict::default();
        let mut at = 0;
        while at < bytes.len() {
            if let Some((num, generation, end)) = object_header(bytes, at) {
                let mut lexer = Lexer::new(bytes, end);
                if let Some(object) = lexer.object_body() {
                    if let Object::Stream(dict, _) = &object {
                        if dict.get("Type").is_some_and(|t| t.is_name("XRef")) {
                            merge_trailer(&mut trailer, dict);
                        }
                    }
                    objects.insert(num, object);
                    generations.insert(num, generation);
                    at = lexer.at;
                    continue;
                }
            }
            if bytes[at..].starts_with(b"trailer") {
                let mut lexer = Lexer::new(bytes, at + b"trailer".len());
                if let Some(Object::Dict(dict)) = lexer.value() {
                    merge_trailer(&mut trailer, &dict);
                    at = lexer.at;
                    continue;
                }
            }
            at += 1;
        }
        let budget = ImageBudget::default();
        let mut file = PdfObjects {
            objects,
            generations,
            trailer,
            stream_limit: budget.max_alloc_bytes,
            inflate_budget: Cell::new(budget.max_document_bytes),
        };
        file.expand_object_streams();
        (!file.objects.is_empty()).then_some(file)
    }

    pub(crate) fn encrypted(&self) -> bool {
        self.trailer.get("Encrypt").is_some()
    }

    /// Decrypt every string and stream with `password` through the standard
    /// security handler, then expand the object streams the encryption hid.
    pub(crate) fn decrypt(&mut self, password: &str) -> Unlock {
        use super::super::crypt::{Decryptor, Method, Standard};
        let Some(entry) = self.trailer.get("Encrypt").cloned() else {
            return Unlock::Open;
        };
        let Some(dict) = self.resolve_dict(Some(&entry)).cloned() else {
            return Unlock::Unsupported;
        };
        if !dict.get("Filter").is_some_and(|f| f.is_name("Standard")) {
            return Unlock::Unsupported;
        }
        let int = |key: &str| {
            dict.get(key)
                .map(|v| self.resolve(v))
                .and_then(Object::as_int)
        };
        let bytes = |key: &str| match dict.get(key).map(|v| self.resolve(v)) {
            Some(Object::Str(s)) => s.clone(),
            _ => Vec::new(),
        };
        let v = int("V").unwrap_or(0);
        // Version 4+ names a crypt filter per kind of data; earlier ones are RC4.
        let method = |key: &str| {
            if v < 4 {
                return Method::Rc4;
            }
            let Some(Object::Name(name)) = dict.get(key) else {
                return Method::Identity;
            };
            let filter = std::str::from_utf8(name)
                .ok()
                .and_then(|name| self.resolve_dict(dict.get("CF"))?.get(name))
                .and_then(|f| self.resolve_dict(Some(f)));
            match filter.and_then(|f| f.get("CFM")) {
                Some(cfm) if cfm.is_name("V2") => Method::Rc4,
                Some(cfm) if cfm.is_name("AESV2") || cfm.is_name("AESV3") => Method::Aes,
                _ => Method::Identity,
            }
        };
        let (streams, strings) = (method("StmF"), method("StrF"));
        let length = int("Length").map_or(if v >= 4 { 16 } else { 5 }, |bits| {
            usize::try_from(bits / 8).unwrap_or(0)
        });
        let r = int("R").unwrap_or(0);
        // AESV2 is AES-128 only: a shorter /Length would make a key AES cannot
        // take. Revisions 5 and 6 always use their own 32-byte key.
        if r < 5 && (streams == Method::Aes || strings == Method::Aes) && length != 16 {
            return Unlock::Unsupported;
        }
        let id = match self.trailer.get("ID").map(|i| self.resolve(i)) {
            Some(Object::Array(items)) => match items.first().map(|i| self.resolve(i)) {
                Some(Object::Str(s)) => s.clone(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        let standard = Standard {
            r,
            length,
            o: bytes("O"),
            u: bytes("U"),
            oe: bytes("OE"),
            ue: bytes("UE"),
            // A writer may store /P unsigned; the low 32 bits are the flags.
            p: int("P").unwrap_or(0) as i32,
            encrypt_metadata: !matches!(dict.get("EncryptMetadata"), Some(Object::Bool(false))),
            id,
        };
        let Some(decryptor) = Decryptor::open(&standard, password) else {
            return Unlock::WrongPassword;
        };
        let skip = match entry {
            Object::Ref(num) => Some(num),
            _ => None,
        };
        for (&num, object) in &mut self.objects {
            if Some(num) == skip {
                continue;
            }
            let generation = self.generations.get(&num).copied().unwrap_or(0);
            let mut string = |data: &[u8]| decryptor.decrypt(num, generation, strings, data);
            match object {
                // Cross-reference streams are never encrypted.
                Object::Stream(dict, _) if dict.get("Type").is_some_and(|t| t.is_name("XRef")) => {}
                Object::Stream(dict, data) => {
                    let plain_metadata = !standard.encrypt_metadata
                        && dict.get("Type").is_some_and(|t| t.is_name("Metadata"));
                    if !plain_metadata {
                        *data = decryptor.decrypt(num, generation, streams, data);
                    }
                    for (_, value) in &mut dict.0 {
                        decrypt_strings(value, &mut string);
                    }
                }
                other => decrypt_strings(other, &mut string),
            }
        }
        self.trailer.0.retain(|(key, _)| key != b"Encrypt");
        self.expand_object_streams();
        Unlock::Open
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
            // Every entry is at least `0 0 ` in the stream, so /N beyond a
            // quarter of its length is a lie; it never sizes an allocation.
            let count = dict.get("N").and_then(Object::as_int).unwrap_or(0).max(0);
            let count = usize::try_from(count)
                .unwrap_or(usize::MAX)
                .min(data.len() / 4);
            let first = dict
                .get("First")
                .and_then(Object::as_int)
                .and_then(|f| usize::try_from(f).ok())
                .unwrap_or(0);
            let mut header = Lexer::new(&data, 0);
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                match (header.value(), header.value()) {
                    (Some(Object::Int(num)), Some(Object::Int(offset))) => {
                        let (Ok(num), Ok(offset)) = (u32::try_from(num), usize::try_from(offset))
                        else {
                            continue;
                        };
                        entries.push((num, offset))
                    }
                    _ => break,
                }
            }
            for (num, offset) in entries {
                let Some(at) = first.checked_add(offset).filter(|&at| at <= data.len()) else {
                    continue;
                };
                let mut lexer = Lexer::new(&data, at);
                if let Some(object) = lexer.value() {
                    // A direct definition elsewhere (an incremental update)
                    // takes precedence over the compressed one.
                    self.objects.entry(num).or_insert(object);
                }
            }
        }
    }
}

/// Replace every string inside `object` with `decrypt` of it.
fn decrypt_strings(object: &mut Object, decrypt: &mut impl FnMut(&[u8]) -> Vec<u8>) {
    match object {
        Object::Str(s) => *s = decrypt(s),
        Object::Array(items) => items.iter_mut().for_each(|i| decrypt_strings(i, decrypt)),
        Object::Dict(dict) => dict
            .0
            .iter_mut()
            .for_each(|(_, value)| decrypt_strings(value, decrypt)),
        _ => {}
    }
}

fn merge_trailer(trailer: &mut Dict, from: &Dict) {
    for key in ["Root", "Encrypt", "Info", "ID"] {
        if let Some(value) = from.get(key) {
            trailer.set(key, value.clone());
        }
    }
}

/// Unfilter a stream: no filter or `FlateDecode` (with a PNG predictor);
/// anything else is `None`. A stream that inflates past the file's per-stream
/// limit or its remaining inflate budget is `None` too (#844).
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
    // Truncated streams are common; keep what inflated.
    let limit = file.stream_limit.min(file.inflate_budget.get());
    let out = read_bounded(flate2::read::ZlibDecoder::new(data), limit)?;
    file.inflate_budget
        .set(file.inflate_budget.get() - out.len() as u64);
    let parms = file.resolve_dict(dict.get("DecodeParms"));
    let predictor = parms
        .and_then(|p| p.get("Predictor"))
        .and_then(Object::as_int)
        .unwrap_or(1);
    let parm = |key, default| {
        parms
            .and_then(|p| p.get(key))
            .and_then(Object::as_int)
            .unwrap_or(default)
            .max(1) as usize
    };
    let columns = parm("Columns", 1);
    if columns > MAX_PREDICTOR_COLUMNS {
        return None;
    }
    if predictor == 2 {
        let colors = parm("Colors", 1);
        if colors > MAX_PREDICTOR_COLORS {
            return None;
        }
        return undo_tiff_predictor(out, columns, colors, parm("BitsPerComponent", 8));
    }
    if predictor < 10 {
        return Some(out);
    }
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

/// Undo TIFF predictor 2 (horizontal differencing per component) on 8- or
/// 16-bit samples; other depths are returned as they are. `None` when the row
/// size overflows or is empty.
pub(crate) fn undo_tiff_predictor(
    mut data: Vec<u8>,
    columns: usize,
    colors: usize,
    bits: usize,
) -> Option<Vec<u8>> {
    let bytes = match bits {
        8 => 1,
        16 => 2,
        _ => return Some(data),
    };
    let row = columns
        .checked_mul(colors)?
        .checked_mul(bytes)
        .filter(|&row| row > 0)?;
    for line in data.chunks_mut(row) {
        if line.len() < row {
            break;
        }
        for i in colors..columns * colors {
            if bytes == 1 {
                line[i] = line[i].wrapping_add(line[i - colors]);
            } else {
                let at = i * 2;
                let prev = u16::from_be_bytes([line[at - colors * 2], line[at - colors * 2 + 1]]);
                let cur = u16::from_be_bytes([line[at], line[at + 1]]).wrapping_add(prev);
                line[at..at + 2].copy_from_slice(&cur.to_be_bytes());
            }
        }
    }
    Some(data)
}

/// `N G obj` starting exactly at `at` (after a delimiter): the object and
/// generation numbers and the offset past `obj`.
fn object_header(bytes: &[u8], at: usize) -> Option<(u32, u16, usize)> {
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
    let generation: u16 = std::str::from_utf8(&bytes[i..i + g]).ok()?.parse().ok()?;
    i += g;
    let s = spaces(i);
    if s == 0 {
        return None;
    }
    i += s;
    if !bytes[i..].starts_with(b"obj") {
        return None;
    }
    Some((num, generation, i + 3))
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
    /// Open arrays and dictionaries around the value being read.
    depth: u32,
}

impl<'a> Lexer<'a> {
    fn new(bytes: &'a [u8], at: usize) -> Self {
        Lexer {
            bytes,
            at,
            depth: 0,
        }
    }

    /// Enter one array or dictionary, or `None` past [`MAX_NESTING`].
    fn descend(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth <= MAX_NESTING).then_some(())
    }

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
                let mut probe = Lexer::new(self.bytes, end);
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
                self.descend()?;
                let mut dict = Dict::default();
                loop {
                    self.skip_space();
                    if self.bytes[self.at..].starts_with(b">>") {
                        self.at += 2;
                        self.depth -= 1;
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
                self.descend()?;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    if self.bytes.get(self.at) == Some(&b']') {
                        self.at += 1;
                        self.depth -= 1;
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
