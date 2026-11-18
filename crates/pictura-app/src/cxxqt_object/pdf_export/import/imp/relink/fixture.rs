//! Synthetic linked-source fixtures for the Qt command and history tests.

use super::{safe_io, Document, QString};
use pictura_codec::{write_descriptor, DescValue};
use pictura_core::{
    BitDepth, Channel, ColorMode, FilterMask, LayerBlock, LayerParts, LayerStack,
    PicturaRawSettings, PsdRect, SmartObject, SmartObjectKind,
};

fn dict(items: impl IntoIterator<Item = (&'static [u8], DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: items
            .into_iter()
            .map(|(key, value)| (key.to_vec(), value))
            .collect(),
    }
}

fn unicode(out: &mut Vec<u8>, value: &str) {
    let units: Vec<_> = value.encode_utf16().chain([0]).collect();
    out.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
}

fn linked_record(data: &mut Vec<u8>, id: &str, vector: bool) {
    let name = if vector { "missing.pdf" } else { "missing.png" };
    let mut record = b"liFE".to_vec();
    record.extend_from_slice(&3u32.to_be_bytes());
    record.push(id.len() as u8);
    record.extend_from_slice(id.as_bytes());
    unicode(&mut record, name);
    record.extend_from_slice(if vector { b"PDF " } else { b"PNG " });
    record.extend_from_slice(b"8BIM");
    record.extend_from_slice(&0u64.to_be_bytes());
    record.push(0);
    let path = format!("file:///missing/{name}");
    record.extend_from_slice(&write_descriptor(&dict([
        (b"fullPath".as_slice(), DescValue::Text(path)),
        (b"relPath".as_slice(), DescValue::Text(name.to_owned())),
    ])));
    record.extend_from_slice(&0u64.to_be_bytes());
    data.extend_from_slice(&(record.len() as u64).to_be_bytes());
    data.extend_from_slice(&record);
    data.resize(data.len() + (4 - record.len() % 4) % 4, 0);
}

pub fn relink_fixture_for_test(path: &QString, vector: bool, filter: bool) -> bool {
    const SHARED: &str = "11111111-1111-4111-8111-111111111111";
    const SINGLE: &str = "22222222-2222-4222-8222-222222222222";
    let mut doc = Document::new(12, 4, ColorMode::Rgb, BitDepth::Eight);
    let path = path.to_string();
    let psb = path.ends_with(".psb");
    doc.is_psb = psb;
    let mut links = Vec::new();
    linked_record(&mut links, SHARED, vector);
    linked_record(&mut links, SINGLE, vector);
    let mut extra = b"8BIMlnk2".to_vec();
    if psb {
        extra.extend_from_slice(&(links.len() as u64).to_be_bytes());
    } else {
        extra.extend_from_slice(&(links.len() as u32).to_be_bytes());
    }
    extra.extend_from_slice(&links);
    doc.layer_section_extra = extra.into();
    let sizes = if vector { (100, 50) } else { (4, 4) };
    doc.layers = LayerStack::from_layers(
        [SHARED, SHARED, SINGLE]
            .into_iter()
            .enumerate()
            .map(|(index, uuid)| {
                let rect = PsdRect {
                    left: index as i32 * 4,
                    top: 0,
                    right: index as i32 * 4 + 4,
                    bottom: 4,
                };
                let width = (b"Wdth".as_slice(), DescValue::Long(sizes.0));
                let height = (b"Hght".as_slice(), DescValue::Long(sizes.1));
                let descriptor = dict([
                    (b"Idnt".as_slice(), DescValue::Text(uuid.to_owned())),
                    (
                        b"PgNm".as_slice(),
                        DescValue::Long(if vector { 2 } else { 1 }),
                    ),
                    (b"Crop".as_slice(), DescValue::Text("mediaBox".to_owned())),
                    (b"Sz  ".as_slice(), dict([width, height])),
                ]);
                let mut config = b"soLD".to_vec();
                config.extend_from_slice(&4u32.to_be_bytes());
                config.extend_from_slice(&write_descriptor(&descriptor));
                config.resize(config.len().next_multiple_of(4), 0);
                let channels: Vec<Channel> = [0, 1, 2, -1]
                    .into_iter()
                    .zip([255, 0, 0, 255])
                    .map(|(id, value)| Channel {
                        id,
                        data: vec![value; 16].into(),
                    })
                    .collect();
                pictura_codec::layer_from_parts(LayerParts {
                    rect,
                    name: format!("Link {}", index + 1),
                    visible: true,
                    channels,
                    extra_blocks: vec![LayerBlock {
                        key: *b"SoLd",
                        data: config.into(),
                    }],
                    ..LayerParts::default()
                })
            })
            .collect(),
    );
    if filter {
        let layer = &mut doc.layers[0];
        let mut object = SmartObject::default();
        object.kind = SmartObjectKind::External;
        object.uuid = SHARED.to_owned();
        object.filter_mask = Some(FilterMask {
            rect: layer.rect,
            data: [0, 0, 255, 255].repeat(4).into(),
        });
        pictura_codec::edit_layer(layer, |parts| parts.smart_object = Some(object));
        let mut settings = PicturaRawSettings::default();
        settings.mode.grayscale = Some(true);
        if pictura_codec::attach_pictura_raw_filter(layer, &settings).is_err() {
            return false;
        }
    }
    let saved = if psb {
        pictura_codec::write_psb(&doc)
    } else {
        pictura_codec::write_psd(&doc)
    };
    saved.is_ok_and(|bytes| safe_io::write_atomically(&path, &bytes, false).is_ok())
}
