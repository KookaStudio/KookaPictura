//! Pictura's PSD/PSB version record and Adobe Version Info.

use pictura_core::Document;

use crate::common::{is_psb_big_key, Reader};
use crate::image_resources::{
    decode_section, encode_image_resources, frame_image_resource, ImageResource, XMP_METADATA,
};
use crate::{xmp::tree, PsdError};

pub const VERSION_INFO: u16 = 1057;
pub const CURRENT_DOCUMENT_VERSION: u16 = 1;
pub const PICTURA_WRITER_NAME: &str = "Kooka Pictura";
const PRIVATE_VERSION: u16 = 4973;
const PRIVATE_NAME: &str = "KookaPicturaDocumentVersion";
const VERSION_NAMESPACE: &str =
    "https://kookastudio.github.io/KookaPictura/ns/document-version/1.0/";
const XMP_FIELDS: [&str; 3] = ["opDocumentVersion", "minReaderVersion", "writerVersion"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionInfo {
    pub version: u32,
    pub has_real_merged_data: bool,
    pub writer: String,
    pub reader: String,
    pub file_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentVersionStamp {
    pub op_document_version: u16,
    pub min_reader_version: u16,
    pub writer_version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum VersionAccess {
    #[default]
    Editable,
    NewerCompatible,
    ReadOnly(&'static str),
}

impl VersionAccess {
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::ReadOnly(_))
    }

    pub fn notice(&self) -> Option<&'static str> {
        match self {
            Self::Editable => None,
            Self::NewerCompatible => {
                Some("This document was written by a newer version of Kooka Pictura.")
            }
            Self::ReadOnly(reason) => Some(reason),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DocumentVersion {
    pub stamp: Option<DocumentVersionStamp>,
    pub version_info: Option<VersionInfo>,
    pub access: VersionAccess,
}

fn parse_info(data: &[u8]) -> Option<VersionInfo> {
    let mut r = Reader::new(data);
    let version = r.u32().ok()?;
    if version != 1 {
        return None;
    }
    let has_real_merged_data = match r.u8().ok()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let writer = unicode_name(&mut r)?;
    let reader = unicode_name(&mut r)?;
    let file_version = r.u32().ok()?;
    (r.remaining() == 0).then_some(VersionInfo {
        version,
        has_real_merged_data,
        writer,
        reader,
        file_version,
    })
}

fn unicode_name(r: &mut Reader<'_>) -> Option<String> {
    let size = (r.u32().ok()? as usize).checked_mul(2)?;
    let bytes = r.take(size).ok()?;
    let units: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(u16::from_be_bytes)
        .collect();
    String::from_utf16(units.strip_suffix(&[0]).unwrap_or(&units)).ok()
}

fn new_info(merged: bool) -> ImageResource {
    let mut data = vec![0, 0, 0, 1, u8::from(merged)];
    for name in [PICTURA_WRITER_NAME, PICTURA_WRITER_NAME] {
        data.extend_from_slice(&((name.encode_utf16().count() + 1) as u32).to_be_bytes());
        for unit in name.encode_utf16().chain([0]) {
            data.extend_from_slice(&unit.to_be_bytes());
        }
    }
    data.extend_from_slice(&1u32.to_be_bytes());
    frame_image_resource(VERSION_INFO, "", &data)
}

fn is_owned(record: &ImageResource) -> bool {
    record.id == PRIVATE_VERSION && record.name == PRIVATE_NAME
}

fn parse_stamp(data: &[u8]) -> Option<DocumentVersionStamp> {
    let mut r = Reader::new(data);
    if r.take(4).ok()? != b"KPVR" {
        return None;
    }
    let layout = r.u16().ok()?;
    let op_document_version = r.u16().ok()?;
    let min_reader_version = r.u16().ok()?;
    let writer_len = usize::from(r.u16().ok()?);
    let writer_version = std::str::from_utf8(r.take(writer_len).ok()?).ok()?;
    if layout == 0
        || min_reader_version == 0
        || min_reader_version > op_document_version
        || writer_version.is_empty()
        || op_document_version <= CURRENT_DOCUMENT_VERSION && (layout != 1 || r.remaining() != 0)
    {
        return None;
    }
    Some(DocumentVersionStamp {
        op_document_version,
        min_reader_version,
        writer_version: writer_version.to_owned(),
    })
}

fn encode_stamp(stamp: &DocumentVersionStamp) -> Result<ImageResource, PsdError> {
    let writer = stamp.writer_version.as_bytes();
    let len = u16::try_from(writer.len())
        .ok()
        .filter(|n| *n != 0)
        .ok_or_else(|| PsdError::Invalid("invalid Pictura writer version length".into()))?;
    let mut bytes = b"KPVR".to_vec();
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&stamp.op_document_version.to_be_bytes());
    bytes.extend_from_slice(&stamp.min_reader_version.to_be_bytes());
    bytes.extend_from_slice(&len.to_be_bytes());
    bytes.extend_from_slice(writer);
    Ok(frame_image_resource(PRIVATE_VERSION, PRIVATE_NAME, &bytes))
}

#[derive(Default)]
struct Mirror {
    values: [Option<String>; 2],
    present: bool,
    ambiguous: bool,
    record: Option<usize>,
}

fn mirror(records: &[ImageResource]) -> Mirror {
    let mut mirror = Mirror::default();
    for (index, record) in records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.id == XMP_METADATA)
    {
        let packet = String::from_utf8_lossy(&record.data);
        let tree = tree::read(&packet, VERSION_NAMESPACE);
        let mut seen = [false; 3];
        for leaf in &tree.leaves {
            let [name] = leaf.path.as_slice() else {
                continue;
            };
            let Some(field) = XMP_FIELDS.iter().position(|key| name.as_str() == *key) else {
                continue;
            };
            if seen[field] {
                mirror.ambiguous = true;
            }
            seen[field] = true;
            if field < 2 {
                mirror.values[field] = Some(leaf.value.clone());
            }
        }
        for element in &tree.elements {
            let Some(name) = element.path.first() else {
                continue;
            };
            let Some(field) = XMP_FIELDS.iter().position(|key| name.as_str() == *key) else {
                continue;
            };
            if !seen[field] || element.unsafe_ {
                mirror.ambiguous = true;
            }
            seen[field] = true;
        }
        if seen.into_iter().any(|found| found) {
            mirror.present = true;
            mirror.ambiguous |= mirror.record.replace(index).is_some() || tree.malformed;
        }
    }
    mirror
}

pub fn document_version(doc: &Document) -> DocumentVersion {
    let (records, end) = decode_section(&doc.image_resources);
    inspect(&records, end == doc.image_resources.len())
}

/// The signed layer count in an Adobe container identifies the first extra
/// merged channel as transparency. High-depth files can store their count
/// inside a global layer-info record instead.
pub fn native_merged_has_transparency(bytes: &[u8], is_psb: bool) -> bool {
    fn inspect(bytes: &[u8], psb: bool) -> Result<bool, PsdError> {
        fn length(r: &mut Reader<'_>, wide: bool) -> Result<usize, PsdError> {
            if wide {
                usize::try_from(r.u64()?).map_err(|_| PsdError::Truncated)
            } else {
                Ok(r.u32()? as usize)
            }
        }
        let mut file = Reader::new(bytes);
        file.skip(26)?;
        for _ in 0..2 {
            let len = file.u32()? as usize;
            file.skip(len)?;
        }
        let len = length(&mut file, psb)?;
        let mut layers = Reader::new(file.take(len)?);
        let info = length(&mut layers, psb)?;
        if info != 0 {
            return Ok(Reader::new(layers.take(info)?).i16()? < 0);
        }
        let mask = layers.u32()? as usize;
        layers.skip(mask)?;
        while layers.remaining() != 0 {
            if !matches!(layers.take(4)?, b"8BIM" | b"8B64") {
                return Ok(false);
            }
            let key: [u8; 4] = layers
                .take(4)?
                .try_into()
                .map_err(|_| PsdError::Truncated)?;
            let len = length(&mut layers, psb && is_psb_big_key(&key))?;
            let data = layers.take(len)?;
            if matches!(&key, b"Mtrn" | b"Mt16" | b"Mt32")
                || matches!(&key, b"Layr" | b"Lr16" | b"Lr32")
                    && Reader::new(data).i16().is_ok_and(|count| count < 0)
            {
                return Ok(true);
            }
            layers.skip((4 - len % 4) % 4)?;
        }
        Ok(false)
    }
    inspect(bytes, is_psb).unwrap_or(false)
}

/// Structural evidence that a native document cannot be safely rewritten
/// after edits. The application also checks what its renderer can display.
pub fn unsupported_document_feature(doc: &Document) -> Option<&'static str> {
    use pictura_core::SmartObjectKind;

    fn layers_issue(layers: &pictura_core::LayerStack, doc: &Document) -> Option<&'static str> {
        for layer in layers {
            if layer.blend_key.is_some() {
                return Some("A layer uses an unsupported blend mode.");
            }
            if layer
                .smart_object()
                .is_some_and(|object| match object.kind {
                    SmartObjectKind::External => {
                        !layer
                            .channels()
                            .iter()
                            .any(|channel| channel.id == 0 && !channel.data.is_empty())
                            || crate::read_placed_source(&object.config_descriptor).is_none()
                            || !crate::repairable_external_link(
                                &doc.layer_section_extra,
                                &object.uuid,
                                doc.is_psb,
                            )
                    }
                    SmartObjectKind::Alias | SmartObjectKind::Unresolved => true,
                    _ => false,
                })
            {
                return Some("A linked Smart Object cannot be safely resolved.");
            }
            for block in layer.extra_blocks() {
                match &block.key {
                b"artb" | b"artd" | b"abdd" => return Some("This document uses an unsupported Photoshop artboard."),
                b"TySh" if layer.type_tool().is_some() || layer.fallback_type_tool().is_some() => {}
                b"TySh" => return Some("A layer contains an unreadable Photoshop type record."),
                b"vmsk" | b"vsms" if layer.vector_mask.is_some() => {}
                b"SoLd" | b"SoLE" | b"plLd" | b"PlLd" if layer.smart_object().is_some() => {}
                b"lfx2" | b"lmfx" | b"lrFX" | b"lyid" | b"shmd" | b"lnsr"
                | b"lspf" | b"lclr" | b"iOpa" | b"clbl" | b"infx" | b"tsly"
                | b"lmgm" | b"vmgm" | b"knko" | b"PiSt" | b"vogk" | b"vstk"
                | b"vscg" | b"sn2P" | b"fxrp" => {}
                _ => return Some("A layer contains unclassified Photoshop data that may depend on its appearance or position."),
            }
            }
            if !layer.raw_channels().is_empty() {
                return Some("A layer contains channels whose editing behavior is unsupported.");
            }
            if let Some(reason) = layers_issue(layer.children(), doc) {
                return Some(reason);
            }
        }
        None
    }

    if let Some(reason) = layers_issue(&doc.layers, doc) {
        return Some(reason);
    }
    match global_feature(&doc.layer_section_extra, doc.is_psb) {
        Ok(reason) => reason,
        Err(_) => Some("The document-level Photoshop records cannot be safely interpreted."),
    }
}

fn global_feature(bytes: &[u8], psb: bool) -> Result<Option<&'static str>, PsdError> {
    let mut reader = Reader::new(bytes);
    while reader.remaining() != 0 {
        if !matches!(reader.take(4)?, b"8BIM" | b"8B64") {
            return Err(PsdError::Invalid("invalid global Photoshop record".into()));
        }
        let key: [u8; 4] = reader
            .take(4)?
            .try_into()
            .map_err(|_| PsdError::Truncated)?;
        let size = if psb && is_psb_big_key(&key) {
            usize::try_from(reader.u64()?).map_err(|_| PsdError::Truncated)?
        } else {
            reader.u32()? as usize
        };
        let data = reader.take(size)?;
        reader.skip((4 - size % 4) % 4)?;
        match &key {
            b"artb" | b"artd" | b"abdd" => return Ok(Some("This document uses an unsupported Photoshop artboard.")),
            b"lnkE" if crate::smart_object::validate_external_link_records(data).is_none() => {
                return Ok(Some("This document contains unsupported external Smart Object records."));
            }
            b"lnkD" | b"lnk2" | b"lnk3" | b"lnkE" | b"Txt2" | b"Patt" | b"Pat2" | b"Pat3"
            | b"FEid" | b"FXid" | b"FMsk" | b"Mtrn" | b"Mt16" | b"Mt32" | b"Anno"
            | b"cinf" => {}
            _ => return Ok(Some("The document contains unclassified Photoshop data that may depend on its layer structure.")),
        }
    }
    Ok(None)
}

fn inspect(records: &[ImageResource], complete: bool) -> DocumentVersion {
    let mut private = records.iter().filter(|r| is_owned(r));
    let owned = private.next();
    let stamp = owned.and_then(|r| parse_stamp(&r.data));
    let mut info_records = records.iter().filter(|r| r.id == VERSION_INFO);
    let info_record = info_records.next();
    let duplicate_info = info_records.next().is_some();
    let version_info = if complete && !duplicate_info {
        info_record.and_then(|record| parse_info(&record.data))
    } else {
        None
    };
    let read_only = if !complete {
        Some(
            "The image-resource section is incomplete, so the document version cannot be verified.",
        )
    } else if private.next().is_some() || owned.is_some() && stamp.is_none() {
        Some("The Kooka Pictura document version record is damaged or duplicated.")
    } else if info_record.is_some() && version_info.is_none() {
        Some("PSD Version Info (1057) is damaged, unsupported, or duplicated.")
    } else if stamp
        .as_ref()
        .is_some_and(|s| s.min_reader_version > CURRENT_DOCUMENT_VERSION)
    {
        Some("This document requires a newer version of Kooka Pictura.")
    } else {
        let mirror = mirror(records);
        let matches = stamp.as_ref().is_some_and(|s| {
            let [version, min_reader] = mirror
                .values
                .each_ref()
                .map(|v| v.as_deref().and_then(|v| v.trim().parse::<u16>().ok()));
            version == Some(s.op_document_version) && min_reader == Some(s.min_reader_version)
        });
        (mirror.present && (mirror.ambiguous || !matches))
            .then_some("The XMP document version disagrees with the Kooka Pictura version record.")
    };
    let access = if let Some(reason) = read_only {
        VersionAccess::ReadOnly(reason)
    } else if stamp
        .as_ref()
        .is_some_and(|s| s.op_document_version > CURRENT_DOCUMENT_VERSION)
    {
        VersionAccess::NewerCompatible
    } else {
        VersionAccess::Editable
    };
    DocumentVersion {
        stamp,
        version_info,
        access,
    }
}

fn mirror_packet(packet: &str, stamp: &DocumentVersionStamp) -> Option<String> {
    let values = [
        stamp.op_document_version.to_string(),
        stamp.min_reader_version.to_string(),
        stamp.writer_version.clone(),
    ];
    let mut packet = packet.to_owned();
    // The generic patcher can expand a self-closing description for a single
    // insertion. Subsequent properties can then be patched into that element.
    for (name, value) in XMP_FIELDS.into_iter().zip(values) {
        packet = tree::patch(
            &packet,
            VERSION_NAMESPACE,
            &[tree::Update {
                path: vec![name.to_owned()],
                shape: tree::Shape::Simple,
                values: vec![value],
            }],
        )?;
    }
    Some(packet)
}

pub(crate) fn resources_for_versioned_write(
    section: &[u8],
    merged: bool,
    writer_version: &str,
) -> Result<(Vec<u8>, bool), PsdError> {
    let (mut resources, end) = decode_section(section);
    let current = inspect(&resources, end == section.len());
    if let VersionAccess::ReadOnly(reason) = current.access {
        return Err(PsdError::Unsupported(reason.into()));
    }
    if let Some(info) = current.version_info {
        if info.has_real_merged_data != merged {
            let record = resources
                .iter_mut()
                .find(|r| r.id == VERSION_INFO)
                .expect("inspected 1057");
            let start = record.raw.len() - record.data.len() - record.data.len() % 2;
            record.raw[start + 4] = u8::from(merged);
            record.data[4] = u8::from(merged);
        }
    } else {
        resources.push(new_info(merged));
    }
    let stamp = if current.access == VersionAccess::NewerCompatible {
        current.stamp.expect("compatible newer stamp")
    } else {
        let stamp = DocumentVersionStamp {
            op_document_version: CURRENT_DOCUMENT_VERSION,
            min_reader_version: CURRENT_DOCUMENT_VERSION,
            writer_version: writer_version.to_owned(),
        };
        let encoded = encode_stamp(&stamp)?;
        if let Some(existing) = resources.iter_mut().find(|r| is_owned(r)) {
            *existing = encoded;
        } else {
            resources.push(encoded);
        }
        stamp
    };
    let xmp = mirror(&resources)
        .record
        .or_else(|| resources.iter().position(|r| r.id == XMP_METADATA));
    const EMPTY_XMP: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description></rdf:Description></rdf:RDF></x:xmpmeta>"#;
    let packet = if let Some(index) = xmp {
        std::str::from_utf8(&resources[index].data).ok()
    } else {
        Some(EMPTY_XMP)
    };
    let patched = packet.and_then(|packet| mirror_packet(packet, &stamp));
    let skipped_mirror = patched.is_none();
    if let Some(patched) = patched {
        if let Some(index) = xmp {
            if resources[index].data != patched.as_bytes() {
                resources[index] =
                    frame_image_resource(XMP_METADATA, &resources[index].name, patched.as_bytes());
            }
        } else {
            resources.push(frame_image_resource(XMP_METADATA, "", patched.as_bytes()));
        }
    }
    Ok((encode_image_resources(&resources), skipped_mirror))
}

#[cfg(test)]
mod tests;
