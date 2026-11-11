//! A document built without decoding a merged image: a layered file that has
//! none ("Maximize Compatibility" off), or one an application open skips
//! because it keeps no composite for the document (#1216).

use pictura_core::{BitDepth, Channel, ColorMode, Compression, Document, LayerStack, PixelBuffer};

/// What the read parsed before the image-data section.
pub(super) struct Sections {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) mode: ColorMode,
    pub(super) source_depth: Option<BitDepth>,
    pub(super) layer_compression: Compression,
    pub(super) is_psb: bool,
    pub(super) layers: LayerStack,
    pub(super) color_mode_data: Vec<u8>,
    pub(super) image_resources: Vec<u8>,
    pub(super) global_layer_mask: Vec<u8>,
    pub(super) layer_section_extra: Vec<u8>,
}

/// The document from `sections` with `composite` standing in for the merged
/// image and `channels` as its extra channels.
pub(super) fn without_merged_image(
    sections: Sections,
    composite: PixelBuffer,
    merged_composite_present: bool,
    composite_compression: Compression,
    channels: Vec<Channel>,
) -> Document {
    let Sections {
        width,
        height,
        mode,
        source_depth,
        layer_compression,
        is_psb,
        layers,
        color_mode_data,
        image_resources,
        global_layer_mask,
        layer_section_extra,
    } = sections;
    Document {
        width,
        height,
        mode,
        depth: BitDepth::Eight,
        source_mode: None,
        source_depth,
        source_planes: None,
        source_palette: None,
        source_icc: None,
        document_icc: None,
        composite,
        merged_composite_present,
        composite_compression,
        layer_compression,
        is_psb,
        layers,
        channels,
        color_mode_data: color_mode_data.into(),
        image_resources: image_resources.into(),
        global_layer_mask: global_layer_mask.into(),
        layer_section_extra: layer_section_extra.into(),
        slices: Vec::new(),
        annotations: Default::default(),
        guides: Vec::new(),
        work_path: Default::default(),
        saved_paths: Vec::new(),
        text_styles: Default::default(),
    }
}

/// The decoded extra channels (saved selections, merged transparency), one
/// `plane` bytes each.
pub(super) fn extra_channels(extras: &[u8], plane: usize) -> Vec<Channel> {
    extras
        .chunks_exact(plane.max(1))
        .enumerate()
        .map(|(i, data)| Channel {
            id: i as i16,
            data: data.to_vec().into(),
        })
        .collect()
}
