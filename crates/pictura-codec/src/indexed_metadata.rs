use pictura_core::{Document, PixelBuffer};

use crate::image_resources::{decode_section, encode_image_resources, frame_image_resource};

const COLOR_COUNT: u16 = 1046;
const TRANSPARENCY_INDEX: u16 = 1047;

pub fn indexed_metadata(doc: &Document) -> (usize, Option<u8>) {
    let (resources, _) = decode_section(&doc.image_resources);
    let value = |id| {
        let data = &resources.iter().find(|resource| resource.id == id)?.data;
        let bytes: [u8; 2] = data.as_slice().try_into().ok()?;
        Some(u16::from_be_bytes(bytes))
    };
    let count = value(COLOR_COUNT).filter(|n| (1..=256).contains(n)).unwrap_or(256);
    let transparent = value(TRANSPARENCY_INDEX).filter(|i| *i < count).map(|i| i as u8);
    (usize::from(count), transparent)
}

pub fn set_indexed_metadata(doc: &mut Document, count: Option<u16>, transparent: Option<u8>) {
    let (mut resources, consumed) = decode_section(&doc.image_resources);
    resources.retain(|resource| !matches!(resource.id, COLOR_COUNT | TRANSPARENCY_INDEX));
    for (id, value) in [(COLOR_COUNT, count), (TRANSPARENCY_INDEX, transparent.map(u16::from))] {
        if let Some(value) = value {
            resources.push(frame_image_resource(id, "", &value.to_be_bytes()));
        }
    }
    let mut section = encode_image_resources(&resources);
    section.extend_from_slice(&doc.image_resources[consumed..]);
    doc.image_resources = section.into();
}

pub(crate) fn without_indexed_metadata(section: Vec<u8>) -> Vec<u8> {
    let (mut resources, consumed) = decode_section(&section);
    let before = resources.len();
    resources.retain(|resource| !matches!(resource.id, COLOR_COUNT | TRANSPARENCY_INDEX));
    if resources.len() == before {
        return section;
    }
    let mut filtered = encode_image_resources(&resources);
    filtered.extend_from_slice(&section[consumed..]);
    filtered
}

pub fn indexed_working_buffer(
    width: u32,
    height: u32,
    indices: &[u8],
    palette: &[u8; 768],
    transparent: Option<u8>,
) -> PixelBuffer {
    let mut data = crate::indexed_to_rgb(indices, palette);
    if let Some(clear) = transparent {
        data.extend(indices.iter().map(|&i| if i == clear { 0 } else { 255 }));
    }
    PixelBuffer {
        width,
        height,
        channels: if transparent.is_some() { 4 } else { 3 },
        data: data.into(),
    }
}

pub(crate) fn restore_layer_transparency(layers: &mut pictura_core::LayerStack, clear: u8) {
    for layer in layers {
        if let Some(indices) = crate::color_mode::layer_retained_8(layer, 8, 0) {
            let alpha = layer.channels_mut();
            if let Some(channel) = alpha.iter_mut().find(|channel| channel.id == -1) {
                if channel.data.len() == indices.len() {
                    for (i, index) in indices.into_iter().enumerate() {
                        if index == clear {
                            channel.data[i] = 0;
                        }
                    }
                }
            } else {
                alpha.push(pictura_core::Channel {
                    id: -1,
                    data: indices.into_iter().map(|i| if i == clear { 0 } else { 255 })
                        .collect::<Vec<_>>().into(),
                });
            }
            layer.background = false;
        }
        if let Some(children) = layer.children_mut() {
            restore_layer_transparency(children, clear);
        }
    }
}
