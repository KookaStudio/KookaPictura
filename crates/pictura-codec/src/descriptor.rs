//! Tolerant PSD descriptor reader/writer.
//!
//! A descriptor is a sequence of `key -> (ostype, value)` pairs. The preserved
//! descriptor bytes remain the source of truth for re-emission; this module
//! derives a typed view so callers can read short keys (for example the Camera
//! Raw Filter's `Fltr` settings). Any ostype this reader does not model is kept
//! verbatim as [`DescValue::Raw`], which carries its 4-byte ostype so it can be
//! re-emitted unchanged.

use crate::budget::DecodeBudget;
use crate::common::Reader;
use crate::error::PsdError;

/// Nesting cap. Real descriptors are a handful of levels deep; the cap keeps a
/// malformed file from exhausting the stack.
const MAX_DEPTH: u32 = 64;

enum DescriptorFailure {
    Malformed(PsdError),
    Resource(PsdError),
}

impl From<PsdError> for DescriptorFailure {
    fn from(error: PsdError) -> Self {
        Self::Malformed(error)
    }
}

impl DescriptorFailure {
    fn into_error(self) -> PsdError {
        match self {
            Self::Malformed(error) | Self::Resource(error) => error,
        }
    }
}

type DescriptorResult<T> = Result<T, DescriptorFailure>;

fn admitted_vec<T>(
    count: usize,
    budget: &mut Option<&mut DecodeBudget>,
    what: &str,
) -> DescriptorResult<Vec<T>> {
    if let Some(budget) = budget.as_deref_mut() {
        return crate::layer_content_budget::vec_with_capacity(count, budget, what)
            .map_err(DescriptorFailure::Resource);
    }
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| {
        DescriptorFailure::Resource(PsdError::Unsupported(format!(
            "{what} allocation refused after decode budget admission"
        )))
    })?;
    Ok(values)
}

/// A parsed descriptor value.
///
/// `Raw` holds the 4-byte ostype followed by the exact value bytes of an
/// unmodeled value, so writing it back reproduces the input verbatim.
#[derive(Debug, Clone, PartialEq)]
pub enum DescValue {
    Long(i32),
    Double(f64),
    UnitFloat {
        unit: [u8; 4],
        value: f64,
    },
    Text(String),
    Bool(bool),
    Enum {
        kind: Vec<u8>,
        value: Vec<u8>,
    },
    Object {
        name: String,
        class_id: Vec<u8>,
        items: Vec<(Vec<u8>, DescValue)>,
    },
    List(Vec<DescValue>),
    Raw(Vec<u8>),
}

/// Read a version-16 `DescriptorBlock` (a descriptor body preceded by its
/// `u32` version) into a [`DescValue::Object`].
pub(crate) fn read_descriptor(r: &mut Reader) -> Result<DescValue, PsdError> {
    read_descriptor_block(r, &mut None).map_err(DescriptorFailure::into_error)
}

pub(crate) fn read_descriptor_with_budget(
    r: &mut Reader,
    budget: &mut DecodeBudget,
) -> Result<Option<DescValue>, PsdError> {
    match read_descriptor_block(r, &mut Some(budget)) {
        Ok(value) => Ok(Some(value)),
        Err(DescriptorFailure::Malformed(_)) => Ok(None),
        Err(DescriptorFailure::Resource(error)) => Err(error),
    }
}

fn read_descriptor_block(
    r: &mut Reader,
    budget: &mut Option<&mut DecodeBudget>,
) -> DescriptorResult<DescValue> {
    let version = r.u32()?;
    if version != 16 {
        return Err(PsdError::Unsupported(format!("descriptor block version {version}")).into());
    }
    read_descriptor_body(r, 0, budget)
}

/// Serialize a descriptor as a version-16 `DescriptorBlock`. Only an
/// [`DescValue::Object`] is meaningful at the top level.
pub fn write_descriptor(value: &DescValue) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&16u32.to_be_bytes());
    if let DescValue::Object {
        name,
        class_id,
        items,
    } = value
    {
        write_object_body(name, class_id, items, &mut out);
    }
    out
}

/// Find an item by key in a parsed [`DescValue::Object`] item list.
pub(crate) fn get_object_item<'a>(
    obj: &'a [(Vec<u8>, DescValue)],
    key: &[u8],
) -> Option<&'a DescValue> {
    obj.iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v)
}

/// Find an item by key mutably in a parsed [`DescValue::Object`] item list.
pub(crate) fn get_object_item_mut<'a>(
    obj: &'a mut [(Vec<u8>, DescValue)],
    key: &[u8],
) -> Option<&'a mut DescValue> {
    obj.iter_mut()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v)
}

/// Replace the item with `key` if present, else append it.
pub(crate) fn set_object_item(obj: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: DescValue) {
    match obj.iter_mut().find(|(k, _)| k.as_slice() == key) {
        Some((_, slot)) => *slot = value,
        None => obj.push((key.to_vec(), value)),
    }
}

fn read_descriptor_body(
    r: &mut Reader,
    depth: u32,
    budget: &mut Option<&mut DecodeBudget>,
) -> DescriptorResult<DescValue> {
    if depth > MAX_DEPTH {
        return Err(too_deep().into());
    }
    let name = read_unicode_string_admitted(r, budget)?;
    let class_id = read_length_and_key_admitted(r, budget)?;
    let count = r.u32()? as usize;
    if count > r.remaining() / 4 {
        return Err(PsdError::Truncated.into());
    }
    let mut items = admitted_vec(count, budget, "descriptor container")?;
    for _ in 0..count {
        let key = read_length_and_key_admitted(r, budget)?;
        let value = read_descriptor_value(r, depth + 1, budget)?;
        items.push((key, value));
    }
    Ok(DescValue::Object {
        name,
        class_id,
        items,
    })
}

fn read_descriptor_value(
    r: &mut Reader,
    depth: u32,
    budget: &mut Option<&mut DecodeBudget>,
) -> DescriptorResult<DescValue> {
    if depth > MAX_DEPTH {
        return Err(too_deep().into());
    }
    let start = r.pos;
    let ostype = arr4(r.take(4)?);
    Ok(match ostype.as_slice() {
        b"Objc" | b"GlbO" => read_descriptor_body(r, depth, budget)?,
        b"VlLs" | b"obj " => read_list(r, depth, budget)?,
        b"long" => DescValue::Long(r.i32()?),
        b"doub" => DescValue::Double(f64::from_bits(r.u64()?)),
        b"UntF" => DescValue::UnitFloat {
            unit: arr4(r.take(4)?),
            value: f64::from_bits(r.u64()?),
        },
        b"TEXT" => DescValue::Text(read_unicode_string_admitted(r, budget)?),
        b"bool" => DescValue::Bool(r.u8()? != 0),
        b"enum" => DescValue::Enum {
            kind: read_length_and_key_admitted(r, budget)?,
            value: read_length_and_key_admitted(r, budget)?,
        },
        _ => {
            skip_descriptor_value(r, ostype, depth)?;
            let bytes = &r.data[start..r.pos];
            let mut raw = admitted_vec(bytes.len(), budget, "descriptor raw data")?;
            raw.extend_from_slice(bytes);
            DescValue::Raw(raw)
        }
    })
}

fn read_list(
    r: &mut Reader,
    depth: u32,
    budget: &mut Option<&mut DecodeBudget>,
) -> DescriptorResult<DescValue> {
    if depth > MAX_DEPTH {
        return Err(too_deep().into());
    }
    let count = r.u32()? as usize;
    if count > r.remaining() / 4 {
        return Err(PsdError::Truncated.into());
    }
    let mut items = admitted_vec(count, budget, "descriptor container")?;
    for _ in 0..count {
        items.push(read_descriptor_value(r, depth + 1, budget)?);
    }
    Ok(DescValue::List(items))
}

fn write_object_body(
    name: &str,
    class_id: &[u8],
    items: &[(Vec<u8>, DescValue)],
    out: &mut Vec<u8>,
) {
    write_unicode_string(name, out);
    write_length_and_key(class_id, out);
    out.extend_from_slice(&(items.len() as u32).to_be_bytes());
    for (key, value) in items {
        write_length_and_key(key, out);
        write_descriptor_value(value, out);
    }
}

fn write_descriptor_value(value: &DescValue, out: &mut Vec<u8>) {
    match value {
        DescValue::Object {
            name,
            class_id,
            items,
        } => {
            out.extend_from_slice(b"Objc");
            write_object_body(name, class_id, items, out);
        }
        DescValue::List(values) => {
            out.extend_from_slice(b"VlLs");
            out.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                write_descriptor_value(value, out);
            }
        }
        DescValue::Long(n) => {
            out.extend_from_slice(b"long");
            out.extend_from_slice(&n.to_be_bytes());
        }
        DescValue::Double(d) => {
            out.extend_from_slice(b"doub");
            out.extend_from_slice(&d.to_bits().to_be_bytes());
        }
        DescValue::UnitFloat { unit, value } => {
            out.extend_from_slice(b"UntF");
            out.extend_from_slice(unit);
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        DescValue::Text(s) => {
            out.extend_from_slice(b"TEXT");
            write_unicode_string(s, out);
        }
        DescValue::Bool(b) => {
            out.extend_from_slice(b"bool");
            out.push(u8::from(*b));
        }
        DescValue::Enum { kind, value } => {
            out.extend_from_slice(b"enum");
            write_length_and_key(kind, out);
            write_length_and_key(value, out);
        }
        DescValue::Raw(bytes) => out.extend_from_slice(bytes),
    }
}

fn write_unicode_string(s: &str, out: &mut Vec<u8>) {
    let units: Vec<u16> = s.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
}

/// Length-and-key: a `u32` length; a zero length means the following 4 bytes
/// are an interned term key.
fn write_length_and_key(key: &[u8], out: &mut Vec<u8>) {
    if key.len() == 4 {
        out.extend_from_slice(&0u32.to_be_bytes());
    } else {
        out.extend_from_slice(&(key.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(key);
}

fn read_length_and_key_admitted(
    r: &mut Reader,
    budget: &mut Option<&mut DecodeBudget>,
) -> DescriptorResult<Vec<u8>> {
    let bytes = read_key_bytes(r)?;
    let mut key = admitted_vec(bytes.len(), budget, "descriptor key")?;
    key.extend_from_slice(bytes);
    Ok(key)
}

/// UTF-16BE string with a `u32` code-unit count.
pub(crate) fn read_unicode_string(r: &mut Reader) -> Result<String, PsdError> {
    read_unicode_string_admitted(r, &mut None).map_err(DescriptorFailure::into_error)
}

fn read_unicode_string_admitted(
    r: &mut Reader,
    budget: &mut Option<&mut DecodeBudget>,
) -> DescriptorResult<String> {
    let count = r.u32()? as usize;
    let bytes = count.checked_mul(2).ok_or_else(size_overflow)?;
    let data = r.take(bytes)?;
    let capacity = count.checked_mul(3).ok_or_else(size_overflow)?;
    if let Some(budget) = budget.as_deref_mut() {
        budget
            .charge("descriptor text", &[capacity])
            .map_err(DescriptorFailure::Resource)?;
    }
    let mut value = String::new();
    value.try_reserve_exact(capacity).map_err(|_| {
        DescriptorFailure::Resource(PsdError::Unsupported(
            "descriptor text allocation refused after decode budget admission".into(),
        ))
    })?;
    value.extend(
        char::decode_utf16(
            data.as_chunks::<2>()
                .0
                .iter()
                .map(|unit| u16::from_be_bytes([unit[0], unit[1]])),
        )
        .map(|decoded| decoded.unwrap_or(char::REPLACEMENT_CHARACTER)),
    );
    Ok(value)
}

fn skip_unicode_string(r: &mut Reader) -> Result<(), PsdError> {
    let count = r.u32()? as usize;
    r.skip(count.checked_mul(2).ok_or_else(size_overflow)?)
}

fn read_key_bytes<'a>(r: &mut Reader<'a>) -> Result<&'a [u8], PsdError> {
    let len = r.u32()? as usize;
    r.take(if len == 0 { 4 } else { len })
}

fn skip_length_and_key(r: &mut Reader) -> Result<(), PsdError> {
    read_key_bytes(r).map(|_| ())
}

/// Skip a `DescriptorBlock` (version header plus body); used by the linked-layer
/// parser, which keeps the descriptor bytes opaque.
pub(crate) fn skip_descriptor_block(r: &mut Reader) -> Result<(), PsdError> {
    read_descriptor(r).map(|_| ())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkedPath {
    Full,
    Original,
    Relative,
}

/// Locate top-level linked-file path strings without rewriting other fields.
pub(crate) fn linked_path_fields(data: &[u8]) -> Option<Vec<(LinkedPath, std::ops::Range<usize>)>> {
    let mut reader = Reader::new(data);
    if reader.u32().ok()? != 16 {
        return None;
    }
    skip_unicode_string(&mut reader).ok()?;
    skip_length_and_key(&mut reader).ok()?;
    let count = reader.u32().ok()?;
    let mut fields = Vec::new();
    let mut full_path = false;
    for _ in 0..count {
        let key = read_key_bytes(&mut reader).ok()?;
        let kind: [u8; 4] = reader.take(4).ok()?.try_into().ok()?;
        let path = match key {
            b"fullPath" => LinkedPath::Full,
            b"originalPath" => LinkedPath::Original,
            b"relPath" => LinkedPath::Relative,
            _ => {
                skip_descriptor_value(&mut reader, kind, 1).ok()?;
                continue;
            }
        };
        if kind != *b"TEXT" {
            return None;
        }
        let start = reader.pos;
        let text = read_unicode_string(&mut reader).ok()?;
        if path == LinkedPath::Full {
            if full_path || text.trim_matches('\0').is_empty() {
                return None;
            }
            full_path = true;
        }
        fields.push((path, start..reader.pos));
    }
    (full_path && reader.remaining() == 0).then_some(fields)
}

fn skip_descriptor_value(r: &mut Reader, ostype: [u8; 4], depth: u32) -> Result<(), PsdError> {
    if depth > MAX_DEPTH {
        return Err(too_deep());
    }
    match ostype.as_slice() {
        b"Objc" | b"GlbO" => skip_descriptor_body(r, depth),
        b"obj " | b"VlLs" => skip_list(r, depth),
        b"doub" => r.skip(8),
        b"UntF" => r.skip(12),
        b"UnFl" => {
            r.skip(4)?;
            let n = r.u32()? as usize;
            r.skip(n.checked_mul(8).ok_or_else(too_deep)?)
        }
        b"TEXT" => {
            skip_unicode_string(r)?;
            Ok(())
        }
        b"enum" => {
            skip_length_and_key(r)?;
            skip_length_and_key(r)?;
            Ok(())
        }
        b"long" | b"indx" | b"Idnt" => r.skip(4),
        b"comp" => r.skip(8),
        b"bool" => r.skip(1),
        b"type" | b"GlbC" | b"Clss" => {
            skip_unicode_string(r)?;
            skip_length_and_key(r)?;
            Ok(())
        }
        b"alis" | b"tdta" | b"Pth " => {
            let n = r.u32()? as usize;
            r.skip(n)
        }
        b"ObAr" => {
            r.skip(4)?;
            skip_descriptor_body(r, depth)
        }
        b"prop" => {
            skip_unicode_string(r)?;
            skip_length_and_key(r)?;
            skip_length_and_key(r)?;
            Ok(())
        }
        b"Enmr" => {
            skip_unicode_string(r)?;
            skip_length_and_key(r)?;
            skip_length_and_key(r)?;
            skip_length_and_key(r)?;
            Ok(())
        }
        b"rele" => {
            skip_unicode_string(r)?;
            skip_length_and_key(r)?;
            r.skip(4)
        }
        b"name" => {
            skip_unicode_string(r)?;
            skip_length_and_key(r)?;
            skip_unicode_string(r)?;
            Ok(())
        }
        _ => Err(PsdError::Unsupported(format!(
            "descriptor ostype {:?}",
            String::from_utf8_lossy(&ostype)
        ))),
    }
}

fn skip_descriptor_body(r: &mut Reader, depth: u32) -> Result<(), PsdError> {
    if depth > MAX_DEPTH {
        return Err(too_deep());
    }
    skip_unicode_string(r)?; // name
    skip_length_and_key(r)?; // classID
    let count = r.u32()?;
    for _ in 0..count {
        skip_length_and_key(r)?; // item key
        let ostype = arr4(r.take(4)?);
        skip_descriptor_value(r, ostype, depth + 1)?;
    }
    Ok(())
}

fn skip_list(r: &mut Reader, depth: u32) -> Result<(), PsdError> {
    if depth > MAX_DEPTH {
        return Err(too_deep());
    }
    let count = r.u32()?;
    for _ in 0..count {
        let ostype = arr4(r.take(4)?);
        skip_descriptor_value(r, ostype, depth + 1)?;
    }
    Ok(())
}

fn too_deep() -> PsdError {
    PsdError::Invalid("descriptor nesting too deep".into())
}

fn size_overflow() -> PsdError {
    PsdError::Invalid("descriptor size overflow".into())
}

fn arr4(s: &[u8]) -> [u8; 4] {
    let mut a = [0u8; 4];
    a.copy_from_slice(&s[..4]);
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(name: &str, class_id: &[u8], items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
        DescValue::Object {
            name: name.to_string(),
            class_id: class_id.to_vec(),
            items,
        }
    }

    fn sample_object() -> DescValue {
        let mut raw = b"comp".to_vec();
        raw.extend_from_slice(&[0u8; 8]);
        obj(
            "",
            b"null",
            vec![
                (b"ABCD".to_vec(), DescValue::Long(-7)),
                (b"toolongkey".to_vec(), DescValue::Text("hi\0".into())),
                (b"dbl ".to_vec(), DescValue::Double(1.5)),
                (b"flag".to_vec(), DescValue::Bool(true)),
                (
                    b"unit".to_vec(),
                    DescValue::UnitFloat {
                        unit: *b"#Prc",
                        value: 0.5,
                    },
                ),
                (
                    b"enum".to_vec(),
                    DescValue::Enum {
                        kind: b"Mnm ".to_vec(),
                        value: b"ntv ".to_vec(),
                    },
                ),
                (
                    b"list".to_vec(),
                    DescValue::List(vec![DescValue::Long(1), DescValue::Text("x".into())]),
                ),
                (
                    b"nest".to_vec(),
                    obj(
                        "nested",
                        b"warp",
                        vec![(b"k".to_vec(), DescValue::Double(-2.0))],
                    ),
                ),
                (b"raw ".to_vec(), DescValue::Raw(raw)),
            ],
        )
    }

    #[test]
    fn descriptor_round_trips_typed_values_and_raw() {
        let value = sample_object();
        let bytes = write_descriptor(&value);
        let mut r = Reader::new(&bytes);
        let back = read_descriptor(&mut r).expect("round-trips");
        assert_eq!(back, value);
        assert_eq!(r.remaining(), 0, "the reader consumed the whole descriptor");
    }

    #[test]
    fn public_read_descriptor_round_trips_and_rejects_truncation() {
        let value = sample_object();
        let bytes = crate::write_descriptor(&value);
        assert_eq!(crate::read_descriptor(&bytes).expect("parses"), value);
        assert!(crate::read_descriptor(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn truncated_descriptor_errors_without_panic() {
        let bytes = write_descriptor(&sample_object());
        for cut in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..cut]);
            assert!(read_descriptor(&mut r).is_err(), "cut {cut} must error");
        }
    }

    #[test]
    fn unknown_ostype_before_known_value_is_skipped() {
        let value = obj(
            "",
            b"null",
            vec![
                (
                    b"comp".to_vec(),
                    DescValue::Raw({
                        let mut raw = b"comp".to_vec();
                        raw.extend_from_slice(&[1u8; 8]);
                        raw
                    }),
                ),
                (b"after".to_vec(), DescValue::Long(9)),
            ],
        );
        let bytes = write_descriptor(&value);
        let back = read_descriptor(&mut Reader::new(&bytes)).unwrap();
        let DescValue::Object { items, .. } = &back else {
            panic!("object");
        };
        assert_eq!(get_object_item(items, b"after"), Some(&DescValue::Long(9)));
    }
}
