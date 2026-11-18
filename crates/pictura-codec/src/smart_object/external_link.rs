use std::ops::Range;

use super::{arr4, descriptor, is_psb_big_key, read_pascal_string, read_unicode_string, Reader};
use crate::descriptor::LinkedPath;

/// Metadata supplied with the bytes read from the user-selected replacement.
pub struct ExternalLinkReplacement<'a> {
    pub full_path: &'a str,
    pub relative_path: &'a str,
    pub filename: &'a str,
    pub filetype: [u8; 4],
    pub bytes: &'a [u8],
    /// PSD external-link date fields in their on-disk order and byte order.
    pub timestamp: Option<[u8; 16]>,
}

struct ExternalRecord<'a> {
    data: &'a [u8],
    filename: Range<usize>,
    filetype: Range<usize>,
    cache_size: Range<usize>,
    paths: Vec<(LinkedPath, Range<usize>)>,
    timestamp: Option<Range<usize>>,
    file_size: Range<usize>,
    cache: Range<usize>,
}

impl<'a> ExternalRecord<'a> {
    fn read(data: &'a [u8]) -> Option<Self> {
        let mut reader = Reader::new(data);
        if reader.take(4).ok()? != b"liFE" {
            return None;
        }
        let version = reader.u32().ok()?;
        if !(2..=7).contains(&version) {
            return None;
        }
        read_pascal_string(&mut reader).ok()?;
        let start = reader.pos;
        read_unicode_string(&mut reader).ok()?;
        let filename = start..reader.pos;
        let start = reader.pos;
        reader.skip(4).ok()?;
        let filetype = start..reader.pos;
        reader.skip(4).ok()?;
        let start = reader.pos;
        let cache_bytes = usize::try_from(reader.u64().ok()?).ok()?;
        let cache_size = start..reader.pos;
        if reader.u8().ok()? != 0 {
            return None;
        }
        let start = reader.pos;
        descriptor::skip_descriptor_block(&mut reader).ok()?;
        let paths = descriptor::linked_path_fields(&data[start..reader.pos])?
            .into_iter()
            .map(|(relative, range)| (relative, (start + range.start)..(start + range.end)))
            .collect();
        let timestamp = if version > 3 {
            let start = reader.pos;
            reader.skip(16).ok()?;
            Some(start..reader.pos)
        } else {
            None
        };
        let start = reader.pos;
        reader.skip(8).ok()?;
        let file_size = start..reader.pos;
        let start = reader.pos;
        if version > 2 {
            reader.skip(cache_bytes).ok()?;
        }
        let mut cache = start..reader.pos;
        if version >= 5
            && !read_unicode_string(&mut reader)
                .ok()?
                .trim_matches('\0')
                .is_empty()
        {
            return None;
        }
        // The unit of nonzero Asset Mod Time and the identity of locked cloud
        // assets are not available from the replacement file.
        if version >= 6 && f64::from_bits(reader.u64().ok()?) != 0.0 {
            return None;
        }
        if version >= 7 && reader.u8().ok()? != 0 {
            return None;
        }
        if version == 2 {
            let start = reader.pos;
            reader.skip(cache_bytes).ok()?;
            cache = start..reader.pos;
        }
        (reader.remaining() == 0).then_some(Self {
            data,
            filename,
            filetype,
            cache_size,
            paths,
            timestamp,
            file_size,
            cache,
        })
    }

    fn repair(self, replacement: &ExternalLinkReplacement<'_>) -> Option<Vec<u8>> {
        let mut edited = Vec::new();
        let mut previous = 0;
        let mut replace = |range: Range<usize>, bytes: &[u8]| {
            edited.extend_from_slice(&self.data[previous..range.start]);
            edited.extend_from_slice(bytes);
            previous = range.end;
        };
        replace(self.filename, &unicode(replacement.filename));
        replace(self.filetype, &replacement.filetype);
        let cache = if self.cache.is_empty() {
            &[]
        } else {
            replacement.bytes
        };
        replace(self.cache_size, &(cache.len() as u64).to_be_bytes());
        for (kind, range) in self.paths {
            let path = match kind {
                LinkedPath::Relative => replacement.relative_path,
                LinkedPath::Full | LinkedPath::Original => replacement.full_path,
            };
            replace(range, &unicode(path));
        }
        if let Some(timestamp) = self.timestamp {
            replace(timestamp, &replacement.timestamp?);
        }
        replace(
            self.file_size,
            &(replacement.bytes.len() as u64).to_be_bytes(),
        );
        replace(self.cache, cache);
        edited.extend_from_slice(&self.data[previous..]);
        Some(edited)
    }
}

fn unicode(text: &str) -> Vec<u8> {
    let units = text.encode_utf16().chain([0]).collect::<Vec<_>>();
    let mut out = Vec::with_capacity(4 + units.len() * 2);
    out.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

/// Check a record's layout without following any linked paths.
pub fn repairable_external_link(extra: &[u8], uuid: &str, psb: bool) -> bool {
    find_external_link(extra, uuid, psb).is_some()
}

/// The location stored in a recognized external record. This is display
/// metadata and is never opened by the codec.
pub fn external_link_location(extra: &[u8], uuid: &str, psb: bool) -> Option<String> {
    let record = find_external_link(extra, uuid, psb)?.record;
    let (_, range) = record
        .paths
        .iter()
        .find(|(kind, _)| *kind == LinkedPath::Full)?;
    let path = read_unicode_string(&mut Reader::new(&record.data[range.clone()])).ok()?;
    Some(path.trim_end_matches('\0').to_owned())
}

/// Rebuild one linked record in place, preserving other records and tags.
/// The caller validates the selected file and prepares the layer previews.
pub fn repair_external_link(
    extra: &[u8],
    uuid: &str,
    psb: bool,
    replacement: &ExternalLinkReplacement<'_>,
) -> Option<Vec<u8>> {
    let location = find_external_link(extra, uuid, psb)?;
    let record = location.record.repair(replacement)?;
    let mut contents = Vec::with_capacity(location.data.len());
    contents.extend_from_slice(&location.data[..location.frame.start]);
    contents.extend_from_slice(&(record.len() as u64).to_be_bytes());
    contents.extend_from_slice(&record);
    contents.resize(contents.len() + (4 - record.len() % 4) % 4, 0);
    contents.extend_from_slice(&location.data[location.frame.end..]);
    let mut out = Vec::with_capacity(extra.len());
    out.extend_from_slice(&extra[..location.tag.start]);
    crate::write::write_tag_document(&mut out, &location.key, &contents, psb);
    out.extend_from_slice(&extra[location.tag.end..]);
    Some(out)
}

struct RecordLocation<'a> {
    key: [u8; 4],
    tag: Range<usize>,
    data: &'a [u8],
    frame: Range<usize>,
    record: ExternalRecord<'a>,
}

fn find_external_link<'a>(extra: &'a [u8], uuid: &str, psb: bool) -> Option<RecordLocation<'a>> {
    let mut reader = Reader::new(extra);
    let mut found = None;
    while reader.remaining() >= 12 {
        let from = reader.pos;
        let signature = reader.take(4).ok()?;
        if signature != b"8BIM" {
            return None;
        }
        let key = arr4(reader.take(4).ok()?);
        let size = if psb && is_psb_big_key(&key) {
            usize::try_from(reader.u64().ok()?).ok()?
        } else {
            reader.u32().ok()? as usize
        };
        let data = reader.take(size).ok()?;
        reader.skip((4 - size % 4) % 4).ok()?;
        if matches!(&key, b"lnkD" | b"lnk2" | b"lnk3" | b"lnkE") {
            let mut records = Reader::new(data);
            while records.remaining() >= 8 {
                let record_start = records.pos;
                let size = usize::try_from(records.u64().ok()?).ok()?;
                let record = records.take(size).ok()?;
                records.skip((4 - size % 4) % 4).ok()?;
                let mut header = Reader::new(record);
                header.skip(8).ok()?;
                if read_pascal_string(&mut header).ok()? == uuid {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(RecordLocation {
                        key,
                        tag: from..reader.pos,
                        data,
                        frame: record_start..records.pos,
                        record: ExternalRecord::read(record)?,
                    });
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::DescValue;
    use pictura_core::{
        BitDepth, ColorMode, Document, LayerBlock, LayerParts, LayerStack, SmartObjectKind,
    };

    fn record(uuid: &str, version: u32) -> Vec<u8> {
        let mut out = b"liFE".to_vec();
        out.extend_from_slice(&version.to_be_bytes());
        out.push(uuid.len() as u8);
        out.extend_from_slice(uuid.as_bytes());
        out.extend_from_slice(&unicode("old.psb"));
        out.extend_from_slice(b"8BPB8BIM");
        out.extend_from_slice(&2u64.to_be_bytes());
        out.push(0);
        let descriptor = DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: vec![
                (
                    b"fullPath".to_vec(),
                    DescValue::Text("file:///missing/old.psb".into()),
                ),
                (b"relPath".to_vec(), DescValue::Text("./old.psb".into())),
                (b"extra".to_vec(), DescValue::Long(17)),
            ],
        };
        out.extend_from_slice(&descriptor::write_descriptor(&descriptor));
        if version >= 4 {
            out.extend_from_slice(&[3; 16]);
        }
        out.extend_from_slice(&2u64.to_be_bytes());
        out.extend_from_slice(b"zz");
        if version >= 5 {
            out.extend_from_slice(&unicode("unrecognized child id"));
        }
        out
    }

    fn extra(psb: bool, records: &[Vec<u8>]) -> Vec<u8> {
        let mut linked = Vec::new();
        for record in records {
            linked.extend_from_slice(&(record.len() as u64).to_be_bytes());
            linked.extend_from_slice(record);
            linked.resize(linked.len() + (4 - record.len() % 4) % 4, 0);
        }
        let mut out = Vec::new();
        crate::write::write_tag_document(&mut out, b"lnk2", &linked, psb);
        crate::write::write_tag_document(&mut out, b"TstX", b"TEST", psb);
        out
    }

    fn host(psb: bool, extra: Vec<u8>, sources: &[&str]) -> Document {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.is_psb = psb;
        doc.layer_section_extra = extra.into();
        doc.layers = LayerStack::from_layers(
            sources
                .iter()
                .map(|uuid| {
                    let descriptor = DescValue::Object {
                        name: String::new(),
                        class_id: b"null".to_vec(),
                        items: vec![
                            (b"Idnt".to_vec(), DescValue::Text((*uuid).into())),
                            (b"PgNm".to_vec(), DescValue::Long(1)),
                        ],
                    };
                    let mut config = b"soLD".to_vec();
                    config.extend_from_slice(&4u32.to_be_bytes());
                    config.extend_from_slice(&descriptor::write_descriptor(&descriptor));
                    config.resize(config.len().next_multiple_of(4), 0);
                    crate::layer_from_parts(LayerParts {
                        name: (*uuid).to_owned(),
                        extra_blocks: vec![LayerBlock {
                            key: *b"SoLd",
                            data: config.into(),
                        }],
                        ..Default::default()
                    })
                })
                .collect(),
        );
        doc
    }

    fn save(doc: &Document) -> Vec<u8> {
        if doc.is_psb {
            crate::write_psb(doc).unwrap()
        } else {
            crate::write_psd(doc).unwrap()
        }
    }

    #[test]
    fn repaired_external_record_roundtrips_selectively() {
        let other = record("unrelated", 3);
        for psb in [false, true] {
            let original = extra(psb, &[record("shared", 4), other.clone()]);
            let replacement = ExternalLinkReplacement {
                full_path: "file:///assets/replacement.psd",
                relative_path: "./replacement.psd",
                filename: "replacement.psd",
                filetype: *b"8BPS",
                timestamp: Some([4; 16]),
                bytes: b"replacement reads here",
            };
            let repaired = repair_external_link(&original, "shared", psb, &replacement).unwrap();
            let doc = host(psb, repaired, &["shared", "shared", "unrelated"]);
            let opened = crate::read_psd(&save(&doc)).unwrap();
            for (layer, original) in opened.layers.iter().zip(doc.layers.iter()) {
                let object = layer.smart_object().unwrap();
                assert_eq!(object.kind, SmartObjectKind::External);
                assert_eq!(object.uuid, original.name);
                assert_eq!(layer.extra_block(b"SoLd"), original.extra_block(b"SoLd"));
            }
            let after = find_external_link(&opened.layer_section_extra, "shared", psb).unwrap();
            let record = after.record;
            let mut name = Reader::new(&record.data[record.filename]);
            assert_eq!(
                read_unicode_string(&mut name)
                    .unwrap()
                    .trim_end_matches('\0'),
                replacement.filename
            );
            assert_eq!(&record.data[record.filetype], &replacement.filetype);
            assert_eq!(&record.data[record.cache], replacement.bytes);
            assert_eq!(&record.data[record.timestamp.unwrap()], &[4; 16]);
            assert_eq!(
                &record.data[record.file_size],
                &(replacement.bytes.len() as u64).to_be_bytes()
            );
            for (kind, range) in &record.paths {
                let mut path = Reader::new(&record.data[range.clone()]);
                let expected = match kind {
                    LinkedPath::Relative => replacement.relative_path,
                    LinkedPath::Full | LinkedPath::Original => replacement.full_path,
                };
                assert_eq!(
                    read_unicode_string(&mut path)
                        .unwrap()
                        .trim_end_matches('\0'),
                    expected
                );
            }
            assert_eq!(
                external_link_location(&opened.layer_section_extra, "shared", psb).as_deref(),
                Some(replacement.full_path)
            );
            let untouched =
                find_external_link(&opened.layer_section_extra, "unrelated", psb).unwrap();
            assert_eq!(untouched.record.data, other.as_slice());
            assert!(opened.layer_section_extra.ends_with(b"TEST"));
        }
    }

    #[test]
    fn unsupported_external_record_stays_opaque() {
        for psb in [false, true] {
            let original = extra(psb, &[record("opaque", 5)]);
            assert!(!repairable_external_link(&original, "opaque", psb));
            let doc = host(psb, original.clone(), &["opaque"]);
            let opened = crate::read_psd(&save(&doc)).unwrap();
            assert_eq!(opened.layer_section_extra.as_slice(), original.as_slice());
        }
    }
}
