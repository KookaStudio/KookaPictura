//! Versioned native preset sets and Photoshop preset interchange.

mod adobe;
mod brush;
pub mod csf;
mod native;
mod swatches;

use crate::action_descriptor::ActionDescriptor;
use crate::common::Reader;
use crate::{PatternPixels, PsdError};

pub use adobe::{item, key_bytes, list, number, object};
pub use native::encode_native;
pub use swatches::encode_swatches;

const MAX_LIBRARY_BYTES: usize = 128 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
const MAX_NAME_UNITS: usize = 65_535;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Brushes,
    Swatches,
    Gradients,
    Styles,
    Patterns,
    Contours,
    CustomShapes,
    Tools,
}

impl Kind {
    pub const ALL: [Self; 8] = [
        Self::Brushes,
        Self::Swatches,
        Self::Gradients,
        Self::Styles,
        Self::Patterns,
        Self::Contours,
        Self::CustomShapes,
        Self::Tools,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Native,
    Aco,
    Ase,
    Abr,
    Grd,
    Asl,
    Pat,
    Shc,
    Csh,
    Tpl,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Self> {
        Some(match ext.to_ascii_lowercase().as_str() {
            "kpset" => Self::Native,
            "aco" => Self::Aco,
            "ase" => Self::Ase,
            "abr" => Self::Abr,
            "grd" => Self::Grd,
            "asl" => Self::Asl,
            "pat" => Self::Pat,
            "shc" => Self::Shc,
            "csh" => Self::Csh,
            "tpl" => Self::Tpl,
            _ => return None,
        })
    }

    pub fn kind(self) -> Option<Kind> {
        Some(match self {
            Self::Native => return None,
            Self::Aco | Self::Ase => Kind::Swatches,
            Self::Abr => Kind::Brushes,
            Self::Grd => Kind::Gradients,
            Self::Asl => Kind::Styles,
            Self::Pat => Kind::Patterns,
            Self::Shc => Kind::Contours,
            Self::Csh => Kind::CustomShapes,
            Self::Tpl => Kind::Tools,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    pub kind: Kind,
    pub entries: Vec<Preset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub name: String,
    pub payload: Payload,
    pub opaque: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    BuiltIn(u16),
    Swatch(Swatch),
    Contour(Contour),
    Shape(Shape),
    Pattern(PatternPixels),
    Gradient(ActionDescriptor),
    Style(ActionDescriptor),
    Tool(ActionDescriptor),
    Brush(Brush),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub bounds: [i32; 4],
    pub path_records: Vec<u8>,
}

impl Shape {
    pub fn paths(&self) -> Option<Vec<pictura_core::path::Subpath>> {
        let [top, left, bottom, right] = self.bounds;
        let width = u32::try_from(right.checked_sub(left)?).ok()?;
        let height = u32::try_from(bottom.checked_sub(top)?).ok()?;
        if width == 0 || height == 0 || !self.path_records.len().is_multiple_of(26) {
            return None;
        }
        crate::vector_mask::decode_path_records(&self.path_records, width, height)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<u8>,
    pub settings: ActionDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Rgb([f32; 3]),
    Cmyk([f32; 4]),
    Lab([f32; 3]),
    Gray(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SwatchRole {
    Global = 0,
    Spot = 1,
    Normal = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swatch {
    pub color: Color,
    pub alpha: f32,
    pub role: SwatchRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contour {
    pub points: Vec<ContourPoint>,
    pub input_min: i32,
    pub input_max: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContourPoint {
    pub input: u16,
    pub output: u16,
    pub corner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message} at byte {offset}")]
pub struct Diagnostic {
    pub offset: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub library: Library,
    pub diagnostics: Vec<Diagnostic>,
}

impl Import {
    fn new(kind: Kind) -> Self {
        Self {
            library: Library {
                kind,
                entries: Vec::new(),
            },
            diagnostics: Vec::new(),
        }
    }
}

pub fn decode(kind: Kind, format: Format, bytes: &[u8]) -> Result<Import, Diagnostic> {
    if bytes.len() > MAX_LIBRARY_BYTES {
        return Err(error(0, "preset library exceeds the byte limit"));
    }
    if format.kind().is_some_and(|expected| expected != kind) {
        return Err(error(0, "preset file does not match the selected type"));
    }
    let mut reader = Reader::new(bytes);
    match format {
        Format::Native => native::decode_native(kind, &mut reader),
        Format::Aco => swatches::decode_aco(&mut reader),
        Format::Ase => swatches::decode_ase(&mut reader),
        Format::Abr => brush::decode_abr(&mut reader),
        Format::Shc => adobe::decode_shc(&mut reader),
        Format::Csh => adobe::decode_csh(&mut reader),
        Format::Grd => adobe::decode_grd(&mut reader),
        Format::Asl => adobe::decode_asl(&mut reader),
        Format::Pat => adobe::decode_pat(&mut reader),
        Format::Tpl => adobe::decode_tpl(&mut reader),
    }
}

pub fn document_pattern_id(pattern: &PatternPixels) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let mut hash = Sha256::new();
    hash.update(pattern.width.to_be_bytes());
    hash.update(pattern.height.to_be_bytes());
    hash.update(&pattern.rgba);
    let mut id = String::from("kooka-preset-");
    for byte in &hash.finalize()[..16] {
        let _ = write!(id, "{byte:02x}");
    }
    id
}

/// Embed a library pattern under its content identity and return that identity.
pub fn embed_document_pattern(
    doc: &mut pictura_core::Document,
    name: &str,
    pattern: &PatternPixels,
) -> Option<String> {
    let id = document_pattern_id(pattern);
    if let Some(existing) = crate::decode_patterns(doc)
        .into_iter()
        .find(|p| p.pattern_id == id)
    {
        return (existing.width == pattern.width
            && existing.height == pattern.height
            && existing.rgba == pattern.rgba)
            .then_some(id);
    }
    let record =
        crate::encode_rgba_pattern(name, &id, pattern.width, pattern.height, &pattern.rgba)?;
    crate::add_document_pattern(doc, &id, &record).then_some(id)
}

/// Decode the length-prefixed pattern records retained with an imported style.
pub fn style_patterns(bytes: &[u8]) -> Result<Vec<(String, PatternPixels)>, Diagnostic> {
    adobe::style_patterns(bytes)
}

fn error(offset: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        offset,
        message: message.into(),
    }
}

impl From<PsdError> for Diagnostic {
    fn from(value: PsdError) -> Self {
        error(0, value.to_string())
    }
}

fn expect(reader: &mut Reader<'_>, bytes: &[u8], message: &str) -> Result<(), Diagnostic> {
    let offset = reader.pos;
    if reader.take(bytes.len())? != bytes {
        return Err(error(offset, message));
    }
    Ok(())
}

fn count(reader: &Reader<'_>, value: u32, record_minimum: usize) -> Result<usize, Diagnostic> {
    let count = value as usize;
    if count > MAX_ENTRIES || count > reader.remaining() / record_minimum {
        return Err(error(reader.pos, "invalid preset record count"));
    }
    Ok(count)
}

fn finish(reader: &Reader<'_>) -> Result<(), Diagnostic> {
    if reader.remaining() == 0 {
        Ok(())
    } else {
        Err(error(reader.pos, "unexpected bytes after preset record"))
    }
}

fn utf16(reader: &mut Reader<'_>, size: usize) -> Result<String, Diagnostic> {
    let offset = reader.pos;
    if size > MAX_NAME_UNITS {
        return Err(error(offset, "preset name exceeds the length limit"));
    }
    let bytes = reader.take(size * 2)?;
    let mut units = bytes
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect::<Vec<_>>();
    if units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16(&units).map_err(|_| error(offset, "invalid UTF-16 preset name"))
}

fn push_len(out: &mut Vec<u8>, len: usize) -> Result<(), Diagnostic> {
    let len = u32::try_from(len).map_err(|_| error(out.len(), "preset record too large"))?;
    out.extend_from_slice(&len.to_be_bytes());
    Ok(())
}
