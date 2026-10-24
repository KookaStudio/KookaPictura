//! Exact Action Manager descriptor codec for version-16 `.atn` files.
//!
//! Unlike the tolerant PSD descriptor view, this model preserves the
//! exact `ActionKey` form, item order, duplicate keys, packed `ObAr`/`UnFl`
//! data, and raw `Pth ` payloads. A descriptor has no version or length prefix
//! here: the step's `hasDescriptor` marker delimits it.

use std::fmt;

use crate::action_file::RawActionSet;
use crate::common::Reader;

const MAX_DEPTH: u32 = 64;
const MAX_ELEMENTS: usize = 1 << 20;

/// An Action Manager identifier: an interned four-character code or a
/// length-prefixed byte string. The form is part of the value.
///
/// The wire has no empty-string form because a zero length selects the code
/// form; writers emit the null code for an empty [`ActionKey::String`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionKey {
    Code([u8; 4]),
    String(Vec<u8>),
}

/// A class name plus its identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionClass {
    pub name: String,
    pub id: ActionKey,
}

/// A descriptor object.
///
/// `name` is the class-name string stored at the start of the object. Readers
/// set `class.name` to the same string; writers prefer `name` and fall back to
/// `class.name` when `name` is empty.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionDescriptor {
    pub name: String,
    pub class: ActionClass,
    pub items: Vec<(ActionKey, ActionValue)>,
}

/// A decoded Action Manager value.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionValue {
    Reference(Vec<ActionReferenceItem>),
    Descriptor(Box<ActionDescriptor>),
    GlobalObject(Box<ActionDescriptor>),
    List(Vec<ActionValue>),
    Double(f64),
    UnitFloat {
        unit: [u8; 4],
        value: f64,
    },
    UnitFloats {
        unit: [u8; 4],
        values: Vec<f64>,
    },
    Text(String),
    Enumerated {
        kind: ActionKey,
        value: ActionKey,
    },
    Integer(i32),
    LargeInteger(i64),
    Boolean(bool),
    /// A boolean byte outside `0`/`1`, preserved verbatim.
    BooleanRaw(u8),
    Class(ActionClass),
    GlobalClass(ActionClass),
    /// Alias payload without its big-endian length field.
    Alias(Vec<u8>),
    /// Raw-data payload without its big-endian length field.
    RawData(Vec<u8>),
    /// Path payload after the big-endian length; its little-endian fields are
    /// preserved verbatim.
    Path(Vec<u8>),
    ObjectArray(ObjectArray),
}

/// Reference forms used by `obj ` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceForm {
    Property,
    Class,
    Enumerated,
    Offset,
    Identifier,
    Index,
    Name,
}

/// One item in an Action Manager reference.
///
/// `key` carries the enumerated type for [`ReferenceForm::Enumerated`];
/// `value` carries the property key, enum value, integer, or name.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionReferenceItem {
    pub form: ReferenceForm,
    pub class: ActionClass,
    pub key: Option<ActionKey>,
    pub value: ReferenceValue,
}

/// The trailing value of a reference item.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceValue {
    None,
    Key(ActionKey),
    /// A signed `rele` offset.
    Integer(i32),
    /// An unsigned `Idnt` or `indx` value.
    Unsigned(u32),
    Text(String),
}

/// One packed `ObAr` item: its key, unit id, and one value per object.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectArrayItem {
    pub key: ActionKey,
    pub unit: [u8; 4],
    pub values: Vec<f64>,
}

/// Packed object-array value (`ObAr`). The class is shared by every object and
/// each item carries one [`ActionValue::UnitFloat`] per object.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectArray {
    class: ActionClass,
    object_count: usize,
    items: Vec<ObjectArrayItem>,
}

impl ObjectArray {
    /// Build a uniform packed array. Returns `None` when an item does not carry
    /// exactly `object_count` values, so non-uniform arrays cannot be written.
    pub fn new(
        class: ActionClass,
        object_count: usize,
        items: Vec<ObjectArrayItem>,
    ) -> Option<Self> {
        if object_count > u32::MAX as usize
            || items.iter().any(|item| item.values.len() != object_count)
        {
            return None;
        }
        Some(Self {
            class,
            object_count,
            items,
        })
    }

    pub fn class(&self) -> &ActionClass {
        &self.class
    }

    pub fn object_count(&self) -> usize {
        self.object_count
    }

    pub fn items(&self) -> &[ObjectArrayItem] {
        &self.items
    }
}

/// A bounded, offset-carrying decode failure.
///
/// `partial_sets` is populated only by [`crate::action_file::read_psp`], which
/// returns the sets decoded before a malformed set or trailing bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct AtnError {
    pub message: String,
    pub offset: usize,
    pub partial_sets: Vec<RawActionSet>,
}

impl fmt::Display for AtnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (offset {})", self.message, self.offset)
    }
}

impl std::error::Error for AtnError {}

pub(crate) fn error(message: impl Into<String>, offset: usize) -> AtnError {
    AtnError {
        message: message.into(),
        offset,
        partial_sets: Vec::new(),
    }
}

fn truncated(offset: usize) -> AtnError {
    error("truncated action data", offset)
}

pub(crate) fn take<'a>(r: &mut Reader<'a>, n: usize) -> Result<&'a [u8], AtnError> {
    let at = r.pos;
    r.take(n).map_err(|_| truncated(at))
}

fn take4(r: &mut Reader) -> Result<[u8; 4], AtnError> {
    let bytes = take(r, 4)?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(crate) fn take_u8(r: &mut Reader) -> Result<u8, AtnError> {
    let at = r.pos;
    r.u8().map_err(|_| truncated(at))
}

pub(crate) fn take_u16(r: &mut Reader) -> Result<u16, AtnError> {
    let at = r.pos;
    r.u16().map_err(|_| truncated(at))
}

pub(crate) fn take_u32(r: &mut Reader) -> Result<u32, AtnError> {
    let at = r.pos;
    r.u32().map_err(|_| truncated(at))
}

pub(crate) fn take_i32(r: &mut Reader) -> Result<i32, AtnError> {
    let at = r.pos;
    r.i32().map_err(|_| truncated(at))
}

fn take_u64(r: &mut Reader) -> Result<u64, AtnError> {
    let at = r.pos;
    r.u64().map_err(|_| truncated(at))
}

pub(crate) fn bounded_count(
    r: &Reader,
    count: u32,
    min_bytes: usize,
    what: &str,
) -> Result<usize, AtnError> {
    let at = r.pos;
    let n = count as usize;
    if n > MAX_ELEMENTS {
        return Err(error(
            format!("{what} count {count} exceeds the {MAX_ELEMENTS} bound"),
            at,
        ));
    }
    if min_bytes != 0 && n.saturating_mul(min_bytes) > r.remaining() {
        return Err(error(
            format!(
                "{what} count {count} exceeds the {} remaining bytes",
                r.remaining()
            ),
            at,
        ));
    }
    Ok(n)
}

fn check_depth(depth: u32, offset: usize) -> Result<(), AtnError> {
    if depth > MAX_DEPTH {
        Err(error("action descriptor nesting too deep", offset))
    } else {
        Ok(())
    }
}

pub(crate) fn read_utf16_units(r: &mut Reader) -> Result<Vec<u16>, AtnError> {
    let count = take_u32(r)?;
    let n = bounded_count(r, count, 2, "UTF-16 string")?;
    let size = n
        .checked_mul(2)
        .ok_or_else(|| error("UTF-16 string size overflow", r.pos))?;
    let bytes = take(r, size)?;
    let (pairs, _) = bytes.as_chunks::<2>();
    Ok(pairs.iter().map(|pair| u16::from_be_bytes(*pair)).collect())
}

fn read_utf16_string(r: &mut Reader) -> Result<String, AtnError> {
    let at = r.pos;
    let units = read_utf16_units(r)?;
    String::from_utf16(&units).map_err(|_| error("invalid UTF-16 string", at))
}

pub(crate) fn read_bytes(r: &mut Reader, what: &str) -> Result<Vec<u8>, AtnError> {
    let count = take_u32(r)?;
    let n = bounded_count(r, count, 1, what)?;
    Ok(take(r, n)?.to_vec())
}

fn read_key(r: &mut Reader) -> Result<ActionKey, AtnError> {
    let len = take_u32(r)?;
    if len == 0 {
        Ok(ActionKey::Code(take4(r)?))
    } else {
        let n = bounded_count(r, len, 1, "identifier")?;
        Ok(ActionKey::String(take(r, n)?.to_vec()))
    }
}

fn read_class(r: &mut Reader) -> Result<ActionClass, AtnError> {
    Ok(ActionClass {
        name: read_utf16_string(r)?,
        id: read_key(r)?,
    })
}

/// Decode one descriptor object body from a version-16 `.atn`.
///
/// `integer_references_have_class` is true for action files: their `Idnt` and
/// `indx` reference items carry the desired class before the integer. Other
/// descriptor streams omit it. Trailing bytes after the body are rejected.
pub fn read_action_descriptor(
    bytes: &[u8],
    integer_references_have_class: bool,
) -> Result<ActionDescriptor, AtnError> {
    let mut r = Reader::new(bytes);
    let descriptor = read_descriptor_body(&mut r, 0, integer_references_have_class)?;
    if r.pos != bytes.len() {
        return Err(error("trailing bytes after action descriptor", r.pos));
    }
    Ok(descriptor)
}

/// Encode one descriptor object body, without a version or length prefix.
pub fn write_action_descriptor(
    desc: &ActionDescriptor,
    integer_references_have_class: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_action_descriptor_to(desc, integer_references_have_class, &mut out);
    out
}

pub(crate) fn read_action_descriptor_from(
    r: &mut Reader,
    integer_references_have_class: bool,
) -> Result<ActionDescriptor, AtnError> {
    read_descriptor_body(r, 0, integer_references_have_class)
}

pub(crate) fn write_action_descriptor_to(
    desc: &ActionDescriptor,
    integer_references_have_class: bool,
    out: &mut Vec<u8>,
) {
    write_descriptor_body(desc, integer_references_have_class, out);
}

fn read_descriptor_body(
    r: &mut Reader,
    depth: u32,
    integer_references_have_class: bool,
) -> Result<ActionDescriptor, AtnError> {
    check_depth(depth, r.pos)?;
    let name = read_utf16_string(r)?;
    let id = read_key(r)?;
    let class = ActionClass {
        name: name.clone(),
        id,
    };
    let count = take_u32(r)?;
    let n = bounded_count(r, count, 8, "descriptor item")?;
    let mut items = Vec::new();
    for _ in 0..n {
        let key = read_key(r)?;
        let value = read_value(r, depth + 1, integer_references_have_class)?;
        items.push((key, value));
    }
    Ok(ActionDescriptor { name, class, items })
}

fn read_value(
    r: &mut Reader,
    depth: u32,
    integer_references_have_class: bool,
) -> Result<ActionValue, AtnError> {
    check_depth(depth, r.pos)?;
    let start = r.pos;
    let ostype = take4(r)?;
    Ok(match &ostype {
        b"Objc" => ActionValue::Descriptor(Box::new(read_descriptor_body(
            r,
            depth,
            integer_references_have_class,
        )?)),
        b"GlbO" => ActionValue::GlobalObject(Box::new(read_descriptor_body(
            r,
            depth,
            integer_references_have_class,
        )?)),
        b"VlLs" => {
            let count = take_u32(r)?;
            let n = bounded_count(r, count, 4, "list item")?;
            let mut values = Vec::new();
            for _ in 0..n {
                values.push(read_value(r, depth + 1, integer_references_have_class)?);
            }
            ActionValue::List(values)
        }
        b"doub" => ActionValue::Double(f64::from_bits(take_u64(r)?)),
        b"UntF" => ActionValue::UnitFloat {
            unit: take4(r)?,
            value: f64::from_bits(take_u64(r)?),
        },
        b"UnFl" => {
            let unit = take4(r)?;
            let count = take_u32(r)?;
            let n = bounded_count(r, count, 8, "unit-float value")?;
            let mut values = Vec::new();
            for _ in 0..n {
                values.push(f64::from_bits(take_u64(r)?));
            }
            ActionValue::UnitFloats { unit, values }
        }
        b"TEXT" => ActionValue::Text(read_utf16_string(r)?),
        b"enum" => ActionValue::Enumerated {
            kind: read_key(r)?,
            value: read_key(r)?,
        },
        b"long" => ActionValue::Integer(take_i32(r)?),
        b"comp" => ActionValue::LargeInteger(take_u64(r)? as i64),
        b"bool" => match take_u8(r)? {
            0 => ActionValue::Boolean(false),
            1 => ActionValue::Boolean(true),
            raw => ActionValue::BooleanRaw(raw),
        },
        b"type" => ActionValue::Class(read_class(r)?),
        b"GlbC" => ActionValue::GlobalClass(read_class(r)?),
        b"alis" => ActionValue::Alias(read_bytes(r, "alias")?),
        b"tdta" => ActionValue::RawData(read_bytes(r, "raw data")?),
        b"Pth " => ActionValue::Path(read_bytes(r, "path")?),
        b"ObAr" => {
            ActionValue::ObjectArray(read_object_array(r, depth, integer_references_have_class)?)
        }
        b"obj " => ActionValue::Reference(read_reference(r, integer_references_have_class)?),
        _ => {
            return Err(error(
                format!(
                    "unknown action descriptor type {:?}",
                    String::from_utf8_lossy(&ostype)
                ),
                start,
            ))
        }
    })
}

fn read_reference(
    r: &mut Reader,
    integer_references_have_class: bool,
) -> Result<Vec<ActionReferenceItem>, AtnError> {
    let count = take_u32(r)?;
    let n = bounded_count(r, count, 4, "reference item")?;
    let mut items = Vec::new();
    for _ in 0..n {
        items.push(read_reference_item(r, integer_references_have_class)?);
    }
    Ok(items)
}

fn read_reference_item(
    r: &mut Reader,
    integer_references_have_class: bool,
) -> Result<ActionReferenceItem, AtnError> {
    let start = r.pos;
    let form_bytes = take4(r)?;
    let form = match &form_bytes {
        b"prop" => ReferenceForm::Property,
        b"Clss" => ReferenceForm::Class,
        b"Enmr" => ReferenceForm::Enumerated,
        b"rele" => ReferenceForm::Offset,
        b"Idnt" => ReferenceForm::Identifier,
        b"indx" => ReferenceForm::Index,
        b"name" => ReferenceForm::Name,
        _ => {
            return Err(error(
                format!(
                    "unknown action reference form {:?}",
                    String::from_utf8_lossy(&form_bytes)
                ),
                start,
            ))
        }
    };
    let integer_form = matches!(form, ReferenceForm::Identifier | ReferenceForm::Index);
    let class = if integer_form && !integer_references_have_class {
        ActionClass {
            name: String::new(),
            id: ActionKey::Code([0; 4]),
        }
    } else {
        read_class(r)?
    };
    let (key, value) = match form {
        ReferenceForm::Property => (None, ReferenceValue::Key(read_key(r)?)),
        ReferenceForm::Class => (None, ReferenceValue::None),
        ReferenceForm::Enumerated => {
            let kind = read_key(r)?;
            let value = read_key(r)?;
            (Some(kind), ReferenceValue::Key(value))
        }
        ReferenceForm::Offset => (None, ReferenceValue::Integer(take_i32(r)?)),
        ReferenceForm::Identifier | ReferenceForm::Index => {
            (None, ReferenceValue::Unsigned(take_u32(r)?))
        }
        ReferenceForm::Name => (None, ReferenceValue::Text(read_utf16_string(r)?)),
    };
    Ok(ActionReferenceItem {
        form,
        class,
        key,
        value,
    })
}

fn read_object_array(
    r: &mut Reader,
    depth: u32,
    _integer_references_have_class: bool,
) -> Result<ObjectArray, AtnError> {
    check_depth(depth, r.pos)?;
    let object_count = take_u32(r)?;
    let objects = bounded_count(r, object_count, 0, "object-array object")?;
    let class = read_class(r)?;
    let item_count_at = r.pos;
    let item_count = take_u32(r)?;
    let items = bounded_count(r, item_count, 17, "object-array item")?;
    let product = objects.checked_mul(items).ok_or_else(|| {
        error(
            format!("object-array value count {objects} x {items} overflows"),
            item_count_at,
        )
    })?;
    if product > MAX_ELEMENTS {
        return Err(error(
            format!(
                "object-array value count {objects} x {items} exceeds the {MAX_ELEMENTS} bound"
            ),
            item_count_at,
        ));
    }
    if product.saturating_mul(8) > r.remaining() {
        return Err(error(
            format!(
                "object-array values need {} bytes but only {} remain",
                product * 8,
                r.remaining()
            ),
            item_count_at,
        ));
    }
    let mut packed = Vec::with_capacity(items);
    for _ in 0..items {
        let key = read_key(r)?;
        let start = r.pos;
        let ostype = take4(r)?;
        if &ostype != b"UnFl" {
            return Err(error(
                format!(
                    "unrecognized object-array item type {:?}",
                    String::from_utf8_lossy(&ostype)
                ),
                start,
            ));
        }
        let unit = take4(r)?;
        let count = take_u32(r)?;
        if count as usize != objects {
            return Err(error(
                format!("unit-float count {count} does not match object count {objects}"),
                r.pos.saturating_sub(4),
            ));
        }
        let n = bounded_count(r, count, 8, "unit-float value")?;
        let mut values = Vec::with_capacity(n);
        for _ in 0..n {
            values.push(f64::from_bits(take_u64(r)?));
        }
        packed.push(ObjectArrayItem { key, unit, values });
    }
    Ok(ObjectArray {
        class,
        object_count: objects,
        items: packed,
    })
}

fn write_descriptor_body(
    desc: &ActionDescriptor,
    integer_references_have_class: bool,
    out: &mut Vec<u8>,
) {
    let name = if desc.name.is_empty() {
        &desc.class.name
    } else {
        &desc.name
    };
    write_utf16(name, out);
    write_key(&desc.class.id, out);
    out.extend_from_slice(&(desc.items.len() as u32).to_be_bytes());
    for (key, value) in &desc.items {
        write_key(key, out);
        write_value(value, integer_references_have_class, out);
    }
}

fn write_value(value: &ActionValue, integer_references_have_class: bool, out: &mut Vec<u8>) {
    match value {
        ActionValue::Reference(items) => {
            out.extend_from_slice(b"obj ");
            out.extend_from_slice(&(items.len() as u32).to_be_bytes());
            for item in items {
                write_reference_item(item, integer_references_have_class, out);
            }
        }
        ActionValue::Descriptor(desc) => {
            out.extend_from_slice(b"Objc");
            write_descriptor_body(desc, integer_references_have_class, out);
        }
        ActionValue::GlobalObject(desc) => {
            out.extend_from_slice(b"GlbO");
            write_descriptor_body(desc, integer_references_have_class, out);
        }
        ActionValue::List(values) => {
            out.extend_from_slice(b"VlLs");
            out.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                write_value(value, integer_references_have_class, out);
            }
        }
        ActionValue::Double(value) => {
            out.extend_from_slice(b"doub");
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        ActionValue::UnitFloat { unit, value } => {
            out.extend_from_slice(b"UntF");
            out.extend_from_slice(unit);
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        ActionValue::UnitFloats { unit, values } => {
            out.extend_from_slice(b"UnFl");
            out.extend_from_slice(unit);
            out.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                out.extend_from_slice(&value.to_bits().to_be_bytes());
            }
        }
        ActionValue::Text(value) => {
            out.extend_from_slice(b"TEXT");
            write_utf16(value, out);
        }
        ActionValue::Enumerated { kind, value } => {
            out.extend_from_slice(b"enum");
            write_key(kind, out);
            write_key(value, out);
        }
        ActionValue::Integer(value) => {
            out.extend_from_slice(b"long");
            out.extend_from_slice(&value.to_be_bytes());
        }
        ActionValue::LargeInteger(value) => {
            out.extend_from_slice(b"comp");
            out.extend_from_slice(&value.to_be_bytes());
        }
        ActionValue::Boolean(value) => {
            out.extend_from_slice(b"bool");
            out.push(u8::from(*value));
        }
        ActionValue::BooleanRaw(value) => {
            out.extend_from_slice(b"bool");
            out.push(*value);
        }
        ActionValue::Class(class) => {
            out.extend_from_slice(b"type");
            write_class(class, out);
        }
        ActionValue::GlobalClass(class) => {
            out.extend_from_slice(b"GlbC");
            write_class(class, out);
        }
        ActionValue::Alias(bytes) => {
            out.extend_from_slice(b"alis");
            write_bytes(bytes, out);
        }
        ActionValue::RawData(bytes) => {
            out.extend_from_slice(b"tdta");
            write_bytes(bytes, out);
        }
        ActionValue::Path(bytes) => {
            out.extend_from_slice(b"Pth ");
            write_bytes(bytes, out);
        }
        ActionValue::ObjectArray(array) => write_object_array(array, out),
    }
}

fn write_reference_item(
    item: &ActionReferenceItem,
    integer_references_have_class: bool,
    out: &mut Vec<u8>,
) {
    out.extend_from_slice(match item.form {
        ReferenceForm::Property => b"prop",
        ReferenceForm::Class => b"Clss",
        ReferenceForm::Enumerated => b"Enmr",
        ReferenceForm::Offset => b"rele",
        ReferenceForm::Identifier => b"Idnt",
        ReferenceForm::Index => b"indx",
        ReferenceForm::Name => b"name",
    });
    let integer_form = matches!(item.form, ReferenceForm::Identifier | ReferenceForm::Index);
    if !(integer_form && !integer_references_have_class) {
        write_class(&item.class, out);
    }
    let default_key = ActionKey::Code([0; 4]);
    match item.form {
        ReferenceForm::Property => match &item.value {
            ReferenceValue::Key(key) => write_key(key, out),
            _ => write_key(&default_key, out),
        },
        ReferenceForm::Class => {}
        ReferenceForm::Enumerated => {
            write_key(item.key.as_ref().unwrap_or(&default_key), out);
            match &item.value {
                ReferenceValue::Key(value) => write_key(value, out),
                _ => write_key(&default_key, out),
            }
        }
        ReferenceForm::Offset => {
            let value = match &item.value {
                ReferenceValue::Integer(value) => *value,
                _ => 0,
            };
            out.extend_from_slice(&value.to_be_bytes());
        }
        ReferenceForm::Identifier | ReferenceForm::Index => {
            let value = match &item.value {
                ReferenceValue::Unsigned(value) => *value,
                _ => 0,
            };
            out.extend_from_slice(&value.to_be_bytes());
        }
        ReferenceForm::Name => {
            let value = match &item.value {
                ReferenceValue::Text(value) => value.as_str(),
                _ => "",
            };
            write_utf16(value, out);
        }
    }
}

fn write_object_array(array: &ObjectArray, out: &mut Vec<u8>) {
    out.extend_from_slice(b"ObAr");
    out.extend_from_slice(&(array.object_count as u32).to_be_bytes());
    write_class(&array.class, out);
    out.extend_from_slice(&(array.items.len() as u32).to_be_bytes());
    for item in &array.items {
        write_key(&item.key, out);
        out.extend_from_slice(b"UnFl");
        out.extend_from_slice(&item.unit);
        out.extend_from_slice(&(array.object_count as u32).to_be_bytes());
        debug_assert_eq!(item.values.len(), array.object_count);
        for value in &item.values {
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
    }
}

fn write_class(class: &ActionClass, out: &mut Vec<u8>) {
    write_utf16(&class.name, out);
    write_key(&class.id, out);
}

fn write_key(key: &ActionKey, out: &mut Vec<u8>) {
    match key {
        ActionKey::Code(code) => {
            out.extend_from_slice(&0u32.to_be_bytes());
            out.extend_from_slice(code);
        }
        ActionKey::String(bytes) if bytes.is_empty() => {
            out.extend_from_slice(&0u32.to_be_bytes());
            out.extend_from_slice(&[0; 4]);
        }
        ActionKey::String(bytes) => write_bytes(bytes, out),
    }
}

fn write_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn write_utf16(value: &str, out: &mut Vec<u8>) {
    let units: Vec<u16> = value.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
}
