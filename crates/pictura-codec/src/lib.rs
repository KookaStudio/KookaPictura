//! Image codecs. M0 scope: minimal PSD/PSB read + write of a **single composite
//! image**. M1-B adds the **Layer and Mask Information** section: layer records,
//! channel image data (raw + PackBits RLE), group/section markers, Unicode
//! names, and raster layer masks, on top of the unchanged composite path.
//!
//! - Read: file header, color mode data and image resources (kept verbatim),
//!   the layer/mask section (parsed when present), then the image data section.
//!   Supported: 8-bit, RGB or Grayscale, composite and layer channel
//!   compression 0 (raw), 1 (RLE/PackBits), 2 (ZIP), or 3 (ZIP-with-prediction).
//!   An unrecognized layer blend key degrades to Normal but is kept verbatim for
//!   re-save, unmodeled layer blocks and channels are retained, and a layered
//!   file with no merged composite ("Maximize Compatibility" off) yields a
//!   zeroed composite instead of a truncation error.
//! - Write: emit a valid PSD whose layer section round-trips through
//!   [`read_psd`], using PackBits RLE channel data and `'luni'`/`'lsct'` tagged
//!   blocks, and re-emit every preserved verbatim block so an open→save loses
//!   nothing.
//! - Anything outside the supported subset returns [`PsdError::Unsupported`],
//!   never a panic.

mod advanced_blending;
mod color_mode;
mod common;
mod crs_xmp;
mod depth;
mod descriptor;
mod duotone;
mod engine_data;
mod engine_data_encode;
mod error;
mod exif;
mod guide_resources;
mod icc;
mod image_resources;
mod iptc;
mod live_shape;
mod metadata;
mod path_resources;
mod patterns;
mod patterns_write;
mod pictura_raw;
mod probe;
mod read;
mod smart_filter;
mod smart_object;
mod smart_writer;
mod type_tool;
mod type_write;
mod vector_mask;
mod web;
mod write;
mod write_bitmap;
mod write_duotone;
mod write_indexed;
mod xmp;

#[cfg(test)]
mod probe_tests;
#[cfg(test)]
mod smart_writer_tests;
#[cfg(test)]
mod tests;

pub use advanced_blending::encode_blend_if;
pub use color_mode::{
    bitmap_rows_to_rgb, cmyk_to_rgb, indexed_to_rgb, lab_to_rgb, rgb_to_lab, xyz_d50_to_srgb_u8,
};
pub use crs_xmp::set_crs_property;
pub use descriptor::{write_descriptor, DescValue};
pub use duotone::{parse_duotone, DuotoneInk, DuotoneSpec, InkColor};
pub use engine_data::{extract_fonts, extract_style, parse_engine_data, EngineValue};
pub use engine_data_encode::encode_engine_data;
pub use error::PsdError;
pub use exif::{exif_tag_name, parse_exif, Exif, ExifValue};
pub use icc::{assign_document_profile, buffer_to_srgb, convert_document, profile_description};
pub use image_resources::{
    decode_image_resources, document_resolution, encode_image_resources, frame_image_resource,
    set_document_resolution, ImageResource, Resolution, EXIF_DATA_1, EXIF_DATA_3, ICC_PROFILE,
    IPTC_NAA, RESOLUTION_INFO, XMP_METADATA,
};
pub use iptc::{encode_iptc, iptc_field_name, parse_iptc, Iptc};
pub use metadata::{
    apply_template, export_template, read_metadata, set_file_info_fields, set_iptc_fields,
    set_xmp_fields, set_xmp_values, xmp_properties, DocumentMetadata, MergeMode,
};
pub use patterns::{decode_patterns, PatternPixels};
pub use patterns_write::{add_document_pattern, encode_rgb_pattern};
pub use pictura_raw::{
    attach_pictura_raw_filter, attach_smart_filter, decode_pictura_raw_settings,
    encode_pictura_raw_fltr, CAMERA_RAW_FILTER_ID, CAMERA_RAW_FILTER_NAME,
};
// Re-exported so the app's profile commands can name a `Profile` without adding
// a direct dependency on `pictura-color`.
pub use live_shape::{
    decode_live_shape, encode_live_shape, LiveShape, ORIGIN_ELLIPSE, ORIGIN_RECTANGLE,
    ORIGIN_ROUNDED_RECTANGLE,
};
pub use pictura_color::{Policy, Profile};
pub use probe::{probe_image, ImageBudget, ImageFormat, ImageProbe, ImportError, LimitKind};
pub use read::{read_psd, read_psd_with};
pub use smart_filter::{
    clear_smart_filters, delete_smart_filter, reorder_smart_filters, set_camera_raw_option,
    set_smart_filter_enabled, set_smart_filters_enabled,
};
pub use smart_object::remove_linked_source;
pub use type_tool::encode_type_tool;
pub use type_write::author_type_tool;
pub use vector_mask::{
    decode_vector_mask, decode_vector_mask_paths, encode_vector_mask, encode_vector_mask_view,
};
pub use web::{
    encode_gif, encode_wbmp, quantize, web_safe_palette, ColorReduction, Dither, Indexed,
    PaletteOptions,
};
pub use write::{write_psb, write_psd};
pub use xmp::{parse_xmp, patch_xmp, patch_xmp_values, to_xmp_packet, XmpField, XmpProperties};

/// Read a Pictura Raw filter's `Fltr` options from a
/// [`pictura_core::SmartFilter::options`] byte buffer.
pub fn camera_raw_options(options: &[u8]) -> Result<DescValue, PsdError> {
    let mut reader = common::Reader::new(options);
    descriptor::read_descriptor(&mut reader)
}

/// Read a bare version-16 `DescriptorBlock` (for example an adjustment-layer
/// payload) into its [`DescValue::Object`].
pub fn read_descriptor(bytes: &[u8]) -> Result<DescValue, PsdError> {
    let mut reader = common::Reader::new(bytes);
    descriptor::read_descriptor(&mut reader)
}
