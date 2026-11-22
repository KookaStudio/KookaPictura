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

pub mod action_descriptor;
pub mod action_file;
mod advanced_blending;
mod budget;
mod color_mode;
mod common;
mod compatibility;
mod crs_xmp;
mod depth;
mod descriptor;
mod dng;
mod document_version;
mod duotone;
mod engine_data;
mod engine_data_encode;
pub mod eps;
mod error;
mod exif;
mod filter_effects;
mod ghostscript;
mod global_light;
mod guide_resources;
mod icc;
mod icc_convert;
pub mod illustrator;
mod image_resources;
mod iptc;
mod layer_comps;
pub mod layer_content;
mod layer_content_budget;
mod normalize_content;
pub use normalize_content::{edit_layer, layer_from_parts, retire_layer_content};
mod source_content;
mod stage_content;
pub use layer_content::*;
pub use stage_content::encoded_fill as encode_fill_content;
mod lazy;
mod lens_correction;
mod live_shape;
mod mapped;
mod mask_params;
pub mod measurement_export;
mod measurement_scale_resource;
mod metadata;
mod path_resources;
mod patterns;
mod patterns_write;
pub mod pdf;
mod pictura_filter;
mod pictura_raw;
pub(crate) mod png;
pub mod preset_library;
pub mod print_resources;
mod probe;
mod profile_probe;
pub(crate) mod raster_metadata;
mod read;
mod save;
mod smart_filter;
mod smart_object;
mod smart_writer;
mod stack_marker;
mod svg;
mod type_path;
mod type_plain;
mod type_shape;
mod type_tool;
mod type_write;
mod variable_resources;
mod vector_marker;
mod vector_mask;
mod vector_transforms;
mod web;
mod webp;
mod write;
mod write_bitmap;
mod write_cache;
mod write_cmyk;
mod write_duotone;
mod write_indexed;
mod write_layers;
mod write_stream;
pub mod xmp;
pub mod zoomify;

#[cfg(test)]
mod probe_tests;
#[cfg(test)]
mod smart_writer_tests;
#[cfg(test)]
mod tests;

pub use advanced_blending::encode_blend_if;
pub use budget::{available_memory, total_memory};
pub use color_mode::{
    bitmap_rows_to_rgb, cmyk_to_rgb, cmyk_to_rgb_with_profile, indexed_to_rgb, lab_to_rgb,
    rgb_to_cmyk_with_profile, rgb_to_lab, xyz_d50_to_srgb_u8,
};
pub use crs_xmp::{crs_settings_from_xmp, set_crs_property};
pub use descriptor::{write_descriptor, DescValue};
pub use dng::{
    as_shot_transform, decode_raw_source, demosaic, develop_raw, downscale_camera, raw_xmp,
    read_dng, read_linear_dng, read_original_raw, read_raw, sniff_raw, sniff_raw_prefix,
    to_pro_photo, white_balance_transform, write_dng, CameraRgb, CameraTransform, CfaPattern,
    DemosaicMethod, DevelopedRgb, DngColor, DngCompatibility, DngCrop, DngImage, DngPixels,
    DngRgb8, DngWrite, LinearDng, ProPhotoRgb, RawFormat, RawLook, RawSource, WhiteBalance,
};
pub use document_version::{
    document_version, native_merged_has_transparency, unsupported_document_feature,
    DocumentVersion, DocumentVersionStamp, VersionAccess, VersionInfo, CURRENT_DOCUMENT_VERSION,
    PICTURA_WRITER_NAME, VERSION_INFO,
};
pub use duotone::{parse_duotone, DuotoneInk, DuotoneSpec, InkColor};
pub use engine_data::{extract_fonts, extract_style, parse_engine_data, EngineValue};
pub use engine_data_encode::encode_engine_data;
pub use error::PsdError;
pub use exif::{exif_tag_name, parse_exif, Exif, ExifValue};
pub use filter_effects::remove_filter_mask;
pub use ghostscript::{sniff_vector, Ghostscript, Raster, RasterError, VectorKind};
pub use global_light::resolve_global_light;
pub use icc::{
    assign_cmyk_profile, assign_document_profile, buffer_to_srgb, convert_document,
    convert_document_with, document_cmyk_profile, embed_document_icc, profile_description,
    remove_document_profile, strip_document_icc,
};
pub use image_resources::{
    alpha_names, append_channel_display_info, channel_selected_areas, decode_image_resources,
    display_info, document_resolution, encode_image_resources, frame_image_resource,
    masked_display_info, remove_channel_display_info, set_alpha_names, set_channel_selected_areas,
    set_display_info, set_document_resolution, set_recovery_resources, set_vanishing_point_data,
    spot_channel_indices, vanishing_point_data, ChannelDisplayInfo, ImageResource, Resolution,
    ALPHA_NAMES_PASCAL, ALPHA_NAMES_UNICODE, DISPLAY_INFO, EXIF_DATA_1, EXIF_DATA_3, ICC_PROFILE,
    IPTC_NAA, MASKED_AREAS_MODE, RESOLUTION_INFO, SELECTED_AREAS_MODE, SPOT_CHANNEL_MODE,
    VANISHING_POINT, VANISHING_POINT_NAME, XMP_METADATA,
};
pub use iptc::{encode_iptc, iptc_field_name, parse_iptc, Iptc};
pub use lens_correction::{
    attach_lens_correction_filter, decode_lens_correction_slots,
    DEFAULT_SLOTS as LENS_CORRECTION_DEFAULT_SLOTS, LENS_CORRECTION_FILTER_ID,
    LENS_CORRECTION_FILTER_NAME,
};
pub use metadata::iptc_extension;
pub use metadata::{
    apply_template, export_template, photoshop_rows, read_metadata, set_file_info_fields,
    set_file_info_values, set_iptc_fields, set_iptc_values, set_xmp_fields, set_xmp_packet,
    set_xmp_values, xmp_properties, DocumentMetadata, MergeMode,
};
pub use patterns::{builtin_patterns, decode_patterns, PatternPixels};
pub use patterns_write::{add_document_pattern, encode_rgb_pattern, encode_rgba_pattern};
pub use pictura_filter::{
    decode_pictura_filter, encode_pictura_filter, FilterStep, FILTER_GALLERY_NAME,
    PICTURA_FILTER_ID,
};
pub use pictura_raw::{
    attach_pictura_raw_filter, decode_pictura_raw_settings, encode_pictura_raw_fltr,
    pictura_raw_from_text, pictura_raw_from_xmp, pictura_raw_to_text, pictura_raw_to_xmp,
    raw_settings_key, CAMERA_RAW_FILTER_ID, CAMERA_RAW_FILTER_NAME,
};
pub use read::read_psd_layer_structure;
pub use smart_object::{
    can_bake_smart_filter_blending, external_link_location, read_placed_source,
    repair_external_link, repairable_external_link, ExternalLinkReplacement, PlacedSource,
};
pub use smart_writer::new_vector_source_id;
// Re-exported so the app's profile commands can name a `Profile` without adding
// a direct dependency on `pictura-color`.
pub use lazy::{read_psd_lazy, MergedPreview, SourcePin};
pub use live_shape::{
    decode_live_shape, encode_live_shape, LiveShape, ORIGIN_ELLIPSE, ORIGIN_RECTANGLE,
    ORIGIN_ROUNDED_RECTANGLE,
};
pub use mapped::MappedFile;
pub use pictura_color::{ConversionOptions, Intent, Policy, Profile};
pub use png::{encode_base64 as encode_png_base64, encode_rgba as encode_png_rgba};
pub use probe::{probe_image, ImageBudget, ImageFormat, ImageProbe, ImportError, LimitKind};
pub use profile_probe::{
    probe_jpeg_profile, probe_png_profile, probe_psd_profile, probe_raster_profile,
    probe_tiff_profile, ProfileProbe,
};
pub use raster_metadata::{
    embed_raster_metadata, raster_image_resources, raster_metadata, RasterMetadata,
    JPEG_XMP_MAX_BYTES,
};
pub use read::{
    read_psd, read_psd_with, read_psd_with_options, read_psd_within, read_psd_within_options,
    OpenOptions,
};
pub use save::{
    save_document, save_document_with_jpeg, save_document_with_jpeg_guarded,
    streams as save_streams, BeforeWrite, JpegEncode, SaveKind, SaveOptions,
};
pub use smart_filter::{
    append_smart_filter, attach_smart_filter, clear_smart_filters, delete_smart_filter,
    reorder_smart_filters, replace_smart_filter, set_camera_raw_option, set_filter_mask_enabled,
    set_smart_filter_enabled, set_smart_filters_enabled,
};
pub use smart_object::{
    layer_references_smart_source, remove_linked_source, replace_embedded_payload,
};
pub use svg::{
    blend_css, ellipse_element, encode_svg, image_element, path_data, path_element, rect_element,
    rounded_rect_element, subpath_data, text_element, SvgContent, SvgGradient, SvgGradientStop,
    SvgNode, SvgOptions, SvgPaint, SvgPattern, SvgStroke, SvgStrokePosition,
};
pub use type_plain::type_is_plain;
pub use type_tool::{decode_type_warp, encode_type_tool};
pub use type_write::{author_type_tool, type_tool_has_uniform_runs};
pub use vector_mask::{
    decode_vector_mask, decode_vector_mask_paths, encode_vector_mask, encode_vector_mask_view,
    encode_vector_mask_with_rule,
};
pub use web::{
    embed_web_metadata, encode_act, encode_gif, encode_wbmp, pack_indexed, palette_delete,
    palette_edit, palette_map_transparency, palette_sort, parse_act, quantize, quantize_custom,
    quantize_locked, sfw_metadata, slice_html, unpack_indexed, web_safe_palette, ColorReduction,
    ColorTable, Dither, Indexed, MetadataLevel, PaletteOptions, PaletteSort, SliceCell,
    SliceHtmlOptions, SliceLayout, WebMetadata,
};
pub use webp::{decode_webp, encode_webp, encode_webp_lossy};
pub use write::{
    write_native_compatibility_cached, write_psb, write_psb_cached, write_psd, write_psd_cached,
    write_psd_to, write_versioned, write_versioned_to, write_versioned_to_checked, writes_psb,
    NativeWriteError, VersionedWrite,
};
pub use write_cache::{warm_encode_idents, EncodeCache, EncodeStats};
pub use write_indexed::{gif_indexed_source_is_current, indexed_source_is_current};
pub use xmp::{parse_xmp, patch_xmp, patch_xmp_values, to_xmp_packet, XmpField, XmpProperties};

/// Read a Pictura Raw filter's `Fltr` options from a
/// [`pictura_core::SmartFilter::options`] byte buffer.
/// `N` bytes from the operating system's random source, for encryption salts,
/// file identifiers, and unguessable scratch names.
pub fn os_random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    // Every supported OS has a random source; there is nothing to fall back to.
    getrandom::fill(&mut bytes).expect("the operating system's random source");
    bytes
}

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
