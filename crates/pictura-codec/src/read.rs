use pictura_color::{ConversionOptions, Policy, Profile};

use pictura_core::*;

use crate::budget::DecodeBudget;
use crate::common::*;
use crate::depth::{depth_bits, planar_len, row_bytes, undo_prediction};
use crate::error::PsdError;
use crate::mask_params;
use crate::probe::ImageBudget;

mod decode;

pub(crate) use decode::decode_channel_data;
#[cfg(test)]
pub(crate) use decode::decode_rle_channel;
pub(crate) use decode::inflate;
use decode::{read_channel_data, read_rle};

/// The deepest group nesting a file may declare. CS6 nests groups ten deep;
/// the cap keeps the recursive compositor and `Drop` off the stack limit.
pub(crate) const MAX_GROUP_DEPTH: usize = 64;

/// Fallback for an Indexed document's palette; unreachable because
/// `palette_from` errors on a malformed Indexed palette before conversion.
const EMPTY_PALETTE: [u8; 768] = [0; 768];

/// Options for opening a PSD/PSB: the incoming-profile policy, an optional
/// working CMYK profile, and the conversion options used by a policy pass.
#[derive(Debug, Clone, Default)]
pub struct OpenOptions {
    pub policy: Policy,
    pub working_cmyk: Option<Vec<u8>>,
    pub conversion: ConversionOptions,
}

/// Read a PSD/PSB applying the historical **Convert** incoming-profile policy.
///
/// ponytail: this is the Convert convenience kept for existing callers (including
/// the render smart-object payload read); the application opens with the user's
/// persisted settings through [`read_psd_with_options`].
pub fn read_psd(bytes: &[u8]) -> Result<Document, PsdError> {
    read_psd_with(bytes, Policy::Convert)
}

/// Parse a PSD (or PSB) file applying an incoming-profile `policy` with the
/// library defaults (sRGB working space, no working CMYK profile).
pub fn read_psd_with(bytes: &[u8], policy: Policy) -> Result<Document, PsdError> {
    read_psd_with_options(
        bytes,
        &OpenOptions {
            policy,
            conversion: legacy_conversion(),
            ..OpenOptions::default()
        },
    )
}

/// The historical library conversion settings: relative colorimetric, no
/// black-point compensation, no dither.
fn legacy_conversion() -> ConversionOptions {
    ConversionOptions {
        intent: pictura_color::Intent::RelativeColorimetric,
        black_point_compensation: false,
        dither: false,
    }
}

/// Parse a PSD/PSB with the application's full colour settings.
pub fn read_psd_with_options(bytes: &[u8], options: &OpenOptions) -> Result<Document, PsdError> {
    read_psd_within_options(bytes, options, ImageBudget::default())
}

/// [`read_psd_with`] under an explicit allocation `budget`: no plane larger
/// than `budget.max_alloc_bytes` is allocated, and a document claiming more
/// than a fixed multiple of it in all is refused before decoding (#844).
pub fn read_psd_within(
    bytes: &[u8],
    policy: Policy,
    budget: ImageBudget,
) -> Result<Document, PsdError> {
    read_psd_within_options(
        bytes,
        &OpenOptions {
            policy,
            conversion: legacy_conversion(),
            ..OpenOptions::default()
        },
        budget,
    )
}

/// [`read_psd_with_options`] under an explicit allocation budget.
pub fn read_psd_within_options(
    bytes: &[u8],
    options: &OpenOptions,
    budget: ImageBudget,
) -> Result<Document, PsdError> {
    let mut doc = read_container(bytes, options, &mut DecodeBudget::new(budget))?;
    crate::path_resources::resolve_paths(&mut doc);
    crate::guide_resources::resolve_guides(&mut doc);
    Ok(doc)
}

fn read_container(
    bytes: &[u8],
    options: &OpenOptions,
    budget: &mut DecodeBudget,
) -> Result<Document, PsdError> {
    let mut r = Reader::new(bytes);
    let sig = r.u32()?;
    if sig != SIGNATURE {
        return Err(PsdError::BadSignature(sig));
    }

    let version = r.u16()?;
    let is_psb = match version {
        VERSION_PSD => false,
        VERSION_PSB => true,
        v => return Err(PsdError::Unsupported(format!("PSD version {v}"))),
    };

    r.skip(6)?; // reserved
    let channels = r.u16()?;
    let height = r.u32()?;
    let width = r.u32()?;
    let depth = r.u16()?;
    let mode_code = r.u16()?;

    let max_dim = if is_psb { MAX_DIM_PSB } else { MAX_DIM_PSD };
    if channels == 0 || channels > MAX_CHANNELS {
        return Err(PsdError::Invalid(format!("channel count {channels}")));
    }
    if width == 0 || height == 0 {
        return Err(PsdError::Invalid("zero image dimension".into()));
    }
    if width > max_dim || height > max_dim {
        return Err(PsdError::Unsupported(format!(
            "dimension {width}x{height} exceeds {max_dim}"
        )));
    }
    let mode = color_mode_from_code(mode_code)?;
    // Depth 1 is Bitmap-only; Multichannel/Duotone open only at depth 8;
    // 16/32 is Grayscale/RGB/CMYK/Lab only (see `psd-bit-depth`).
    let depth_ok = match depth {
        1 => mode == ColorMode::Bitmap,
        8 => true,
        16 | 32 => matches!(
            mode,
            ColorMode::Grayscale | ColorMode::Rgb | ColorMode::Lab | ColorMode::Cmyk
        ),
        _ => false,
    };
    if !depth_ok {
        return Err(PsdError::Unsupported(format!("bit depth {depth}")));
    }
    // Multichannel opens only for header channel count 1 or 3; other counts stay Unsupported.
    if mode == ColorMode::Multichannel && !matches!(channels, 1 | 3) {
        return Err(PsdError::Unsupported(format!(
            "Multichannel channel count {channels}"
        )));
    }
    // Multichannel has no fixed `color_channels()`; the header count is authoritative.
    let color_count = if mode == ColorMode::Multichannel {
        channels as usize
    } else {
        mode.color_channels() as usize
    };
    // Retain native planes for 16/32 Grayscale/RGB/Lab/CMYK, and 8-bit Lab/
    // CMYK/Indexed/Duotone/Multichannel, plus the depth-1 Bitmap packed plane.
    let retain_planes = (matches!(depth, 16 | 32)
        && matches!(
            mode,
            ColorMode::Grayscale | ColorMode::Rgb | ColorMode::Lab | ColorMode::Cmyk
        ))
        || (depth == 8
            && matches!(
                mode,
                ColorMode::Lab
                    | ColorMode::Cmyk
                    | ColorMode::Indexed
                    | ColorMode::Duotone
                    | ColorMode::Multichannel
            ))
        || (depth == 1 && mode == ColorMode::Bitmap);
    let source_depth = depth_bits(depth);
    // The composite's planes (or the blank one a layered file without a merged
    // image gets) are charged before any section is read.
    for _ in 0..channels {
        charge_plane(
            budget,
            "composite plane",
            width as usize,
            height as usize,
            depth,
        )?;
    }

    // Color mode data section: 4-byte length + opaque bytes, kept verbatim.
    // An Indexed palette is interpreted and consumed (see `normalize`).
    let color_mode_len = r.u32()? as usize;
    let color_mode_data = r.take(color_mode_len)?.to_vec();
    let palette = palette_from(mode, &color_mode_data)?;
    // Image resources section: 4-byte length + opaque bytes, kept verbatim.
    let resources_len = r.u32()? as usize;
    let image_resources = r.take(resources_len)?.to_vec();
    // Layer and mask information section: 4-byte length (8 in PSB).
    let (mut layers, global_layer_mask, layer_section_extra, layer_compression) =
        read_layer_section(&mut r, is_psb, color_count, depth, retain_planes, budget)?;
    // Derive the smart-object view from the preserved bytes; a malformed
    // descriptor or linked-layer record degrades to Unresolved (design D5).
    crate::smart_object::resolve_smart_objects(&mut layers, &layer_section_extra, is_psb);
    // Derive the vector-mask view from the preserved `vmsk` blocks.
    // ponytail: this is a read snapshot. In-session resize/crop/orientation do
    // not re-derive it; the raw block stays authoritative and a reload re-derives.
    crate::vector_mask::resolve_vector_masks(&mut layers, width, height);
    // Derive the type-tool view from the preserved `TySh` blocks, then apply
    // the document-level `Txt2` text-frame paths to those layers.
    crate::type_tool::resolve_type_tools(&mut layers);
    crate::type_path::resolve_text_paths(&mut layers, &layer_section_extra, is_psb);

    // "Maximize Compatibility" off: a layered file may end after the layer
    // section with no merged composite. A file with no layers at all and no
    // trailing data is still malformed and falls through to the truncation error.
    if r.remaining() == 0 && !layers.is_empty() {
        let doc = Document {
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
            composite: PixelBuffer::new(width, height, color_count as u8),
            merged_composite_present: false,
            composite_compression: Compression::Rle,
            layer_compression: layer_compression.unwrap_or_default(),
            is_psb,
            layers,
            channels: Vec::new(),
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
        };
        return Ok(crate::icc::apply_icc_policy_with_options(
            normalize(doc, mode, depth, palette.as_ref(), options),
            options.policy,
            options.conversion,
        ));
    }

    // Image data section: 2-byte compression method, then one plane per header
    // channel (color channels first, then alpha/spot/selection channels).
    let compression = r.u16()?;
    let composite_compression = Compression::from_code(compression)
        .ok_or_else(|| PsdError::Unsupported(format!("compression {compression}")))?;
    let header_channels = channels as usize;
    let width = width as usize;
    let height = height as usize;
    let stride = row_bytes(width, depth);
    if depth == 1 && matches!(compression, COMPRESSION_ZIP | COMPRESSION_ZIP_PREDICTION) {
        return Err(PsdError::Unsupported("ZIP compression at depth 1".into()));
    }
    let mut data = match composite_compression {
        Compression::Raw => r
            .take(planar_len(header_channels, stride, height)?)?
            .to_vec(),
        Compression::Rle => read_rle(&mut r, header_channels, stride, height, is_psb)?,
        Compression::Zip | Compression::ZipPrediction => {
            let expected = planar_len(header_channels, stride, height)?;
            let remaining = r.remaining();
            let payload = r.take(remaining)?.to_vec();
            let mut data = inflate(&payload, expected)?;
            if composite_compression == Compression::ZipPrediction {
                undo_prediction(&mut data, width, header_channels * height, depth);
            }
            data
        }
    };
    // Decode the retained native bytes into typed samples, then narrow them so
    // every later step sees 8-bit planes. A depth-1 read keeps its store at
    // `One` (the raw packed plane); a 16/32-bit read keeps its native sample
    // width. ponytail: costs 2x/4x the plane size while open.
    let store_depth = if depth == 1 {
        BitDepth::One
    } else {
        depth_bits(depth).unwrap_or(BitDepth::Eight)
    };
    let native = retain_planes.then(|| Samples::from_bytes(&data, store_depth));
    let plane = if matches!(depth, 16 | 32) {
        data = native
            .as_ref()
            .expect("16/32-bit reads retain their native samples")
            .narrow_to_u8();
        width * height
    } else {
        stride * height
    };
    let source_planes = native.map(|samples| SourcePlanes {
        depth: store_depth,
        width: width as u32,
        height: height as u32,
        samples,
    });
    let (composite, channels) =
        split_planes(data, color_count, width, height, plane, header_channels)?;

    let doc = Document {
        width: width as u32,
        height: height as u32,
        mode,
        depth: BitDepth::Eight,
        source_mode: None,
        source_depth,
        source_planes,
        source_palette: None,
        source_icc: None,
        document_icc: None,
        composite,
        merged_composite_present: true,
        composite_compression,
        layer_compression: layer_compression.unwrap_or_default(),
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
    };
    Ok(crate::icc::apply_icc_policy_with_options(
        normalize(doc, mode, depth, palette.as_ref(), options),
        options.policy,
        options.conversion,
    ))
}

/// The PSD `header.color_mode` code to the engine's [`ColorMode`]. Bitmap,
/// Grayscale, Indexed, RGB, CMYK, Multichannel, Duotone, and Lab are accepted;
/// every other code is unsupported.
fn color_mode_from_code(code: u16) -> Result<ColorMode, PsdError> {
    Ok(match code {
        MODE_BITMAP => ColorMode::Bitmap,
        MODE_GRAYSCALE => ColorMode::Grayscale,
        MODE_INDEXED => ColorMode::Indexed,
        MODE_RGB => ColorMode::Rgb,
        MODE_CMYK => ColorMode::Cmyk,
        MODE_MULTICHANNEL => ColorMode::Multichannel,
        MODE_DUOTONE => ColorMode::Duotone,
        MODE_LAB => ColorMode::Lab,
        c => return Err(PsdError::Unsupported(format!("color mode {c}"))),
    })
}

/// The 768-byte Indexed palette (256 red, then green, then blue), or `None` for
/// a mode that has no palette. Any other length is malformed, never a panic.
fn palette_from(mode: ColorMode, data: &[u8]) -> Result<Option<[u8; 768]>, PsdError> {
    if mode != ColorMode::Indexed {
        return Ok(None);
    }
    if data.len() != 768 {
        return Err(PsdError::Invalid(format!(
            "indexed palette length {}",
            data.len()
        )));
    }
    let mut palette = [0u8; 768];
    palette.copy_from_slice(data);
    Ok(Some(palette))
}

/// Normalize a non-RGB document to the engine's working mode. Grayscale and RGB
/// pass through untouched; Bitmap/Indexed/CMYK/Lab become RGB, record the source
/// mode, and convert every color plane (composite and every layer, including a
/// grouped layer's descendants).
fn normalize(
    mut doc: Document,
    header_mode: ColorMode,
    depth: u16,
    palette: Option<&[u8; 768]>,
    options: &OpenOptions,
) -> Document {
    if matches!(header_mode, ColorMode::Grayscale | ColorMode::Rgb) {
        return doc;
    }
    if header_mode == ColorMode::Lab {
        crate::color_mode::retain_lab_layer_planes(&mut doc.layers, depth);
    }
    if header_mode == ColorMode::Cmyk {
        crate::color_mode::retain_cmyk_layer_planes(&mut doc.layers, depth);
    }
    if header_mode == ColorMode::Indexed {
        crate::color_mode::retain_indexed_layer_planes(&mut doc.layers, depth);
    }
    let palette = palette.unwrap_or(&EMPTY_PALETTE);

    let embedded_cmyk = if header_mode == ColorMode::Cmyk {
        crate::icc::cmyk_profile_from_resources(&doc)
    } else {
        None
    };
    let working_cmyk = options
        .working_cmyk
        .as_deref()
        .and_then(|bytes| Profile::from_icc(bytes).ok());
    let decode_profile = if options.policy == Policy::Off {
        None
    } else {
        embedded_cmyk.as_ref().or(working_cmyk.as_ref())
    };

    doc.composite = convert_pixels(
        doc.composite,
        header_mode,
        depth,
        Some(palette),
        decode_profile,
        options.conversion,
    );
    for layer in &mut doc.layers {
        crate::color_mode::convert_layer_color_channels(
            layer,
            header_mode,
            palette,
            decode_profile,
            options.conversion,
        );
    }
    doc.mode = ColorMode::Rgb;
    doc.depth = BitDepth::Eight;
    doc.source_mode = Some(header_mode);
    if header_mode == ColorMode::Indexed {
        doc.source_palette = Some(*palette);
        doc.color_mode_data.clear();
    }
    if header_mode == ColorMode::Cmyk {
        doc.document_icc = None;
        match options.policy {
            Policy::Off => {
                crate::icc::set_icc_resource(&mut doc, None);
                doc.source_icc = None;
            }
            Policy::Preserve => {
                doc.source_icc = None;
                // An untagged CMYK file is treated as the working CMYK; attach
                // the working profile so a save re-encodes consistently. An
                // embedded profile is kept byte-for-byte.
                if embedded_cmyk.is_none() {
                    if let Some(working) = working_cmyk.as_ref() {
                        let icc = working.to_icc();
                        crate::icc::set_icc_resource(&mut doc, Some(&icc));
                    }
                }
            }
            Policy::Convert => {
                let source_bytes = crate::icc::icc_resource_bytes(&doc);
                if let (Some(src), Some(dst)) = (embedded_cmyk.as_ref(), working_cmyk.as_ref()) {
                    crate::color_mode::convert_retained_cmyk(
                        &mut doc,
                        src,
                        dst,
                        options.conversion,
                    );
                    let icc = dst.to_icc();
                    crate::icc::set_icc_resource(&mut doc, Some(&icc));
                } else if let Some(dst) = working_cmyk.as_ref() {
                    let icc = dst.to_icc();
                    crate::icc::set_icc_resource(&mut doc, Some(&icc));
                } else if let Some(src) = embedded_cmyk.as_ref() {
                    let icc = src.to_icc();
                    crate::icc::set_icc_resource(&mut doc, Some(&icc));
                } else {
                    crate::icc::set_icc_resource(&mut doc, None);
                }
                doc.source_icc = source_bytes.map(Into::into);
            }
        }
    }
    doc
}

/// Convert a composite's color planes to planar RGB.
fn convert_pixels(
    buf: PixelBuffer,
    mode: ColorMode,
    depth: u16,
    palette: Option<&[u8; 768]>,
    cmyk_profile: Option<&Profile>,
    options: ConversionOptions,
) -> PixelBuffer {
    use crate::color_mode::*;
    let data = match mode {
        ColorMode::Bitmap if depth == 1 => {
            bitmap_rows_to_rgb(&buf.data, buf.width as usize, buf.height as usize)
        }
        ColorMode::Bitmap => gray_to_rgb(&buf.data),
        ColorMode::Indexed => indexed_to_rgb(&buf.data, palette.unwrap_or(&EMPTY_PALETTE)),
        ColorMode::Cmyk => cmyk_profile
            .and_then(|profile| cmyk_to_rgb_with_profile(&buf.data, profile, options))
            .unwrap_or_else(|| cmyk_to_rgb(&buf.data)),
        ColorMode::Lab => lab_to_rgb(&buf.data),
        ColorMode::Duotone => gray_to_rgb(&buf.data),
        ColorMode::Multichannel if buf.channels == 1 => gray_to_rgb(&buf.data),
        ColorMode::Multichannel => cmy_to_rgb(&buf.data),
        ColorMode::Grayscale | ColorMode::Rgb => buf.data.to_vec(),
    };
    PixelBuffer {
        width: buf.width,
        height: buf.height,
        channels: 3,
        data: data.into(),
    }
}

/// Split the planar image-data section into the mode's color planes (the
/// composite) and the trailing extra channels (saved selections / alpha).
/// `plane` is one channel's byte length (depth-aware). `color_channels` is the
/// header-authoritative color count (Multichannel's channel count, else the
/// mode's fixed count).
fn split_planes(
    mut data: Vec<u8>,
    color_channels: usize,
    width: usize,
    height: usize,
    plane: usize,
    header_channels: usize,
) -> Result<(PixelBuffer, Vec<Channel>), PsdError> {
    if header_channels < color_channels {
        return Err(PsdError::Invalid(format!(
            "header has {header_channels} channels for a {color_channels}-channel mode"
        )));
    }
    let extra = data.split_off(color_channels * plane);
    let channels = extra
        .chunks_exact(plane)
        .enumerate()
        .map(|(i, plane)| Channel {
            id: i as i16,
            data: plane.to_vec().into(),
        })
        .collect();
    Ok((
        PixelBuffer {
            width: width as u32,
            height: height as u32,
            channels: color_channels as u8,
            data: data.into(),
        },
        channels,
    ))
}

// ---------------------------------------------------------------------------
// Layer and mask information section — reading
// ---------------------------------------------------------------------------

/// One layer record before the tree is assembled. Channel info (id + declared
/// data length) is kept separate so channel image data can be read in a second
/// pass, after all records.
struct RawLayer {
    layer: Layer,
    channel_ids: Vec<i16>,
    channel_lens: Vec<usize>,
    section: Option<u32>,
}

/// `(layers, global_layer_mask, layer_section_extra, layer_compression)` read
/// from the section; the compression is `None` when no engine-encoded layer
/// channel was present.
type LayerSection = (LayerStack, Vec<u8>, Vec<u8>, Option<Compression>);

fn read_layer_section(
    r: &mut Reader,
    is_psb: bool,
    color_channels: usize,
    depth: u16,
    retain: bool,
    budget: &mut DecodeBudget,
) -> Result<LayerSection, PsdError> {
    let section_len = if is_psb {
        r.u64()? as usize
    } else {
        r.u32()? as usize
    };
    if section_len == 0 {
        return Ok((LayerStack::default(), Vec::new(), Vec::new(), None));
    }
    let section_end = r
        .pos
        .checked_add(section_len)
        .ok_or_else(|| PsdError::Invalid("layer section length overflow".into()))?;
    if section_end > r.data.len() {
        return Err(PsdError::Truncated);
    }

    let info_len = if is_psb {
        r.u64()? as usize
    } else {
        r.u32()? as usize
    };
    let mut layers = LayerStack::default();
    let mut layer_compression = None;
    if info_len != 0 {
        let info_end = r
            .pos
            .checked_add(info_len)
            .ok_or_else(|| PsdError::Invalid("layer info length overflow".into()))?;
        if info_end > section_end {
            return Err(PsdError::Invalid("layer info exceeds layer section".into()));
        }
        (layers, layer_compression) =
            read_layer_info(r, is_psb, info_end, color_channels, depth, retain, budget)?;
        r.pos = info_end;
    }

    // Global layer mask info: 4-byte length + opaque bytes, kept verbatim.
    let global_len = r.u32()? as usize;
    let global_layer_mask = r.take(global_len)?.to_vec();
    if r.pos > section_end {
        return Err(PsdError::Invalid(
            "global layer mask exceeds section".into(),
        ));
    }
    // Remaining bytes are trailing global additional-layer information.
    let extra_len = section_end - r.pos;
    let layer_section_extra = r.take(extra_len)?.to_vec();
    Ok((
        layers,
        global_layer_mask,
        layer_section_extra,
        layer_compression,
    ))
}

fn read_layer_info(
    r: &mut Reader,
    is_psb: bool,
    info_end: usize,
    color_channels: usize,
    depth: u16,
    retain: bool,
    budget: &mut DecodeBudget,
) -> Result<(LayerStack, Option<Compression>), PsdError> {
    let count = r.i16()?;
    let n = count.unsigned_abs() as usize;
    let mut raws: Vec<RawLayer> = Vec::new();
    for _ in 0..n {
        raws.push(read_layer_record(r, is_psb)?);
    }

    // First engine-encoded layer channel's compression; a document mixing kinds
    // within a category normalizes to the first seen.
    // ponytail: one kind per category, per-channel fidelity if ever needed.
    let mut layer_compression = None;

    for raw in raws.iter_mut() {
        let layer_w = raw.layer.rect.width().max(0) as usize;
        let layer_h = raw.layer.rect.height().max(0) as usize;
        let mask_dims = raw.layer.mask.as_ref().map(|m| {
            (
                m.rect.width().max(0) as usize,
                m.rect.height().max(0) as usize,
            )
        });

        let mut channels = Vec::new();
        let mut raw_channels = Vec::new();
        let mut mask_data = None;
        let mut retained: Vec<(i16, Samples)> = Vec::new();
        for (&id, &len) in raw.channel_ids.iter().zip(raw.channel_lens.iter()) {
            // Channels outside the modeled set (a mode's extra color planes are
            // decoded; spot/selection channels, notably -3, and any positive id
            // beyond the color channels). At depth 8 the full on-disk stream
            // (compression header included) is preserved verbatim; at depth
            // 1/16/32 it is decoded at the document depth, narrowed to an 8-bit
            // plane, and re-wrapped as a raw 8-bit stream for the engine.
            let is_color = id >= 0 && (id as usize) < color_channels;
            if id != -2 && id != -1 && !is_color {
                let (data, native) = if depth == 8 {
                    (r.take(len)?.to_vec(), None)
                } else {
                    charge_channel(budget, len, layer_w, layer_h, depth)?;
                    let (plane, _, native) =
                        read_channel_data(r, len, layer_w, layer_h, is_psb, depth)?;
                    let mut stream = Vec::with_capacity(2 + plane.len());
                    stream.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
                    stream.extend_from_slice(&plane);
                    (stream, native)
                };
                if retain {
                    if let Some(native) = native {
                        retained.push((id, native));
                    }
                }
                raw_channels.push(RawChannel {
                    id,
                    data: data.into(),
                });
                continue;
            }
            // The user layer mask channel (-2) is sized by the mask rect, which
            // may differ from the layer rect.
            let (w, h) = if id == -2 {
                mask_dims.unwrap_or((layer_w, layer_h))
            } else {
                (layer_w, layer_h)
            };
            charge_channel(budget, len, w, h, depth)?;
            let (data, code, native) = read_channel_data(r, len, w, h, is_psb, depth)?;
            if retain {
                if let Some(native) = native {
                    retained.push((id, native));
                }
            }
            // Section dividers and folder records carry placeholder channels that
            // `build_tree` drops; only a normal record's channel is a survivor.
            if raw.section.is_none() && layer_compression.is_none() {
                layer_compression = Compression::from_code(code);
            }
            match id {
                -2 => mask_data = Some(data),
                _ => channels.push(Channel {
                    id,
                    data: data.into(),
                }),
            }
        }
        // `build_tree` drops folder/divider records; an `lsct=0` layer is kept.
        let dropped = matches!(
            raw.section,
            Some(SECTION_DIVIDER | SECTION_OPEN_FOLDER | SECTION_CLOSED_FOLDER)
        );
        if retain && !dropped && !retained.is_empty() {
            if let Some(bits) = depth_bits(depth) {
                raw.layer.source_channels =
                    Some(SourceChannels::new(bits, raw.layer.rect, retained));
            }
        }
        raw.layer.channels = channels;
        raw.layer.raw_channels = raw_channels;
        if let Some(data) = mask_data {
            match raw.layer.mask.as_mut() {
                Some(mask) => mask.data = Some(data.into()),
                None => {
                    raw.layer.mask = Some(LayerMask {
                        rect: raw.layer.rect,
                        default_color: 0,
                        disabled: false,
                        flags: 0,
                        data: Some(data.into()),
                        ..Default::default()
                    });
                }
            }
        }
    }

    if r.pos > info_end {
        return Err(PsdError::Invalid("layer records exceed layer info".into()));
    }
    r.pos = info_end;
    let mut layers = build_tree(raws)?;
    derive_background(&mut layers);
    Ok((layers, layer_compression))
}

/// Charge the allocations one `width` × `height` plane costs while it decodes
/// at `depth`: the native plane plus its 8-bit narrowing, or a depth-1 plane
/// expanded through one three-plane RGB allocation and then copied.
fn charge_plane(
    budget: &mut DecodeBudget,
    what: &str,
    width: usize,
    height: usize,
    depth: u16,
) -> Result<(), PsdError> {
    match depth {
        1 => budget.charge(what, &[width, height, 3])?,
        16 | 32 => budget.charge(what, &[width, height, depth as usize / 8])?,
        _ => 0,
    };
    budget.charge(what, &[width, height])?;
    Ok(())
}

/// Charge one layer channel's decoded plane before it is allocated. An empty
/// channel (`declared_len == 0`) decodes to nothing and costs nothing.
fn charge_channel(
    budget: &mut DecodeBudget,
    declared_len: usize,
    width: usize,
    height: usize,
    depth: u16,
) -> Result<(), PsdError> {
    if declared_len != 0 {
        charge_plane(budget, "layer channel", width, height, depth)?;
    }
    Ok(())
}

/// PSD has no background bit: the bottom top-level, non-group layer named
/// `"Background"` is the Background (design D5, marked inferred).
fn derive_background(layers: &mut LayerStack) {
    if let Some(bottom) = layers.first_mut() {
        if !bottom.is_group && bottom.name == "Background" {
            bottom.background = true;
        }
    }
}

fn read_layer_record(r: &mut Reader, is_psb: bool) -> Result<RawLayer, PsdError> {
    let rect = PsdRect {
        top: r.i32()?,
        left: r.i32()?,
        bottom: r.i32()?,
        right: r.i32()?,
    };
    let nch = r.u16()?;
    if nch > MAX_CHANNELS {
        return Err(PsdError::Invalid(format!("layer channel count {nch}")));
    }
    let mut channel_ids = Vec::with_capacity(nch as usize);
    let mut channel_lens = Vec::with_capacity(nch as usize);
    for _ in 0..nch {
        channel_ids.push(r.i16()?);
        channel_lens.push(if is_psb {
            r.u64()? as usize
        } else {
            r.u32()? as usize
        });
    }

    let mut signature = [0u8; 4];
    signature.copy_from_slice(r.take(4)?);
    if &signature != b"8BIM" {
        return Err(PsdError::Invalid("bad layer blend signature".into()));
    }
    let mut key = [0u8; 4];
    key.copy_from_slice(r.take(4)?);
    // An unrecognized key degrades to Normal and is preserved verbatim; a
    // recognized mode already round-trips through BlendMode.
    let mut blend_key = None;
    let mut blend = match BlendMode::from_psd_key(key) {
        Some(mode) => mode,
        None => {
            blend_key = Some(key);
            BlendMode::Normal
        }
    };

    let opacity = r.u8()?;
    let clipping = r.u8()? != 0;
    let flags = r.u8()?;
    let _filler = r.u8()?;

    // M36 attribute defaults; the tagged blocks below override them.
    let mut fill = 255u8;
    let mut lock = LockFlags::default();
    let mut color = ColorLabel::None;
    let (mut knockout, mut blend_clipping, mut blend_interior) = (Knockout::None, true, true);

    let extra_len = r.u32()? as usize;
    let extra = r.take(extra_len)?;
    let mut er = Reader::new(extra);

    // Layer mask / adjustment layer data block.
    let mask_len = er.u32()? as usize;
    let mut mask = None;
    let mut vector_params = None;
    if mask_len > 0 {
        let bytes = er.take(mask_len)?;
        let mut mr = Reader::new(bytes);
        let mask_rect = PsdRect {
            top: mr.i32()?,
            left: mr.i32()?,
            bottom: mr.i32()?,
            right: mr.i32()?,
        };
        let default_color = mr.u8()?;
        let mask_flags = mr.u8()?;
        let has_real = channel_ids.contains(&-3);
        let tail = mask_params::read_tail(mask_flags, bytes.get(18..).unwrap_or(&[]), has_real);
        vector_params = mask_params::vector_placeholder(tail.vector);
        // A record with no `-2` channel and no rect only carries parameters.
        let empty = mask_rect.width() <= 0 || mask_rect.height() <= 0;
        if channel_ids.contains(&-2) || !empty {
            mask = Some(LayerMask {
                rect: mask_rect,
                default_color,
                disabled: mask_flags & mask_params::FLAG_DISABLED != 0,
                flags: mask_flags,
                data: None,
                extra: tail.extra.into(),
                density: tail.user.0,
                feather_bits: tail.user.1,
            });
        }
    }

    // Layer blending ranges, kept verbatim for lossless re-save.
    let ranges_len = er.u32()? as usize;
    let blending_ranges = er.take(ranges_len)?.to_vec();
    let blend_if = crate::advanced_blending::parse_blend_if(&blending_ranges);

    // Legacy Pascal name, padded so (length byte + chars) is a multiple of 4.
    let name_len = er.u8()? as usize;
    let name_bytes = er.take(name_len)?;
    let mut name = String::from_utf8_lossy(name_bytes).into_owned();
    let pad = (4 - ((name_len + 1) % 4)) % 4;
    er.skip(pad)?;

    // Additional layer information: 'luni' (Unicode name), 'lsct' (group marker),
    // and the adjustment block for adjustment layers (stored verbatim).
    let mut section = None;
    let mut adjustment = None;
    let mut extra_blocks = Vec::new();
    while er.remaining() >= 12 {
        let mut tag_sig = [0u8; 4];
        tag_sig.copy_from_slice(er.take(4)?);
        // psd-tools accepts 8BIM/8B64 for a tagged block; normalized to 8BIM on write.
        if &tag_sig != b"8BIM" && &tag_sig != b"8B64" {
            return Err(PsdError::Invalid("bad tagged block signature".into()));
        }
        let mut tag_key = [0u8; 4];
        tag_key.copy_from_slice(er.take(4)?);
        let tag_len = if is_psb && is_psb_big_key(&tag_key) {
            er.u64()? as usize
        } else {
            er.u32()? as usize
        };
        let data = er.take(tag_len)?;
        if tag_len % 2 == 1 {
            // Tagged block data is padded to an even length.
            let _ = er.skip(1);
        }
        match &tag_key {
            b"luni" => {
                if let Some(unicode) = parse_luni(data) {
                    name = unicode;
                }
            }
            b"lspf" if data.len() >= 4 => {
                let value = u32::from_be_bytes(data[0..4].try_into().unwrap());
                lock = lock_from_bits(value as u8 & 0x0F);
            }
            b"lclr" if data.len() >= 2 => {
                let value = u16::from_be_bytes(data[0..2].try_into().unwrap());
                color = ColorLabel::from_byte(value as u8);
            }
            b"iOpa" if !data.is_empty() => {
                fill = data[0];
            }
            b"knko" => knockout = Knockout::from_byte(data.first().copied().unwrap_or(0)),
            b"clbl" => blend_clipping = data.first().is_none_or(|&b| b != 0),
            b"infx" => blend_interior = data.first().is_none_or(|&b| b != 0),
            b"lsct" if data.len() >= 4 => {
                section = Some(u32::from_be_bytes(data[0..4].try_into().unwrap()));
                // The reference/psd-tools store a group's blend key inside 'lsct'
                // (after the '8BIM' signature); the folder record's own key is
                // usually 'norm'. Prefer the 'lsct' key so pass-through groups
                // load as PassThrough.
                if data.len() >= 12 && &data[4..8] == b"8BIM" {
                    let lsct_key: [u8; 4] = data[8..12].try_into().unwrap();
                    if let Some(m) = BlendMode::from_psd_key(lsct_key) {
                        blend = m;
                    }
                }
            }
            b"lsct" => {}
            k if is_adjustment_key(k) => {
                if adjustment.is_none() {
                    adjustment = Some(AdjustmentData {
                        key: tag_key,
                        data: data.to_vec().into(),
                    });
                } else {
                    // A second adjustment key is not modeled twice; keep it raw.
                    extra_blocks.push(LayerBlock {
                        key: tag_key,
                        data: data.to_vec().into(),
                    });
                }
            }
            _ => extra_blocks.push(LayerBlock {
                key: tag_key,
                data: data.to_vec().into(),
            }),
        }
    }

    // The reference's ≤5-era transparency-protected bit shares the transparency lock.
    if flags & 0x01 != 0 {
        lock = lock.with(LockFlags::TRANSPARENCY, true);
    }

    // CS6 default type locks when the layer carries a preserved TySh block.
    if extra_blocks.iter().any(|b| b.key == *b"TySh") {
        lock = lock
            .with(LockFlags::TRANSPARENCY, true)
            .with(LockFlags::PIXELS, true);
    }

    Ok(RawLayer {
        layer: Layer {
            name,
            rect,
            blend,
            opacity,
            fill,
            lock,
            color,
            clipping,
            visible: flags & 0x02 == 0,
            mask,
            adjustment,
            channels: Vec::new(),
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            blend_key,
            blending_ranges: blending_ranges.into(),
            knockout,
            blend_clipping,
            blend_interior,
            blend_if,
            extra_blocks,
            raw_channels: Vec::new(),
            smart_object: None,
            vector_mask: vector_params,
            type_tool: None,
            applied_character_style: None,
            applied_paragraph_style: None,
            type_overrides: Default::default(),
            source_channels: None,
        },
        channel_ids,
        channel_lens,
        section,
    })
}

/// Build [`LockFlags`] from the four `lspf`/record `flags` low bits. The
/// newtype has no public bit constructor, so set each lock explicitly.
fn lock_from_bits(bits: u8) -> LockFlags {
    LockFlags::default()
        .with(LockFlags::TRANSPARENCY, bits & LockFlags::TRANSPARENCY != 0)
        .with(LockFlags::PIXELS, bits & LockFlags::PIXELS != 0)
        .with(LockFlags::POSITION, bits & LockFlags::POSITION != 0)
        .with(LockFlags::NESTING, bits & LockFlags::NESTING != 0)
}

/// Decode a `'luni'` tagged block: a `u32` UTF-16 code-unit count followed by
/// that many big-endian `u16` units (a trailing null may follow).
fn parse_luni(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }
    let count = u32::from_be_bytes(data[0..4].try_into().ok()?) as usize;
    if count == 0 {
        return None;
    }
    let bytes = count.checked_mul(2)?;
    let end = (4 + bytes).min(data.len());
    let (pairs, _) = data[4..end].as_chunks::<2>();
    let units: Vec<u16> = pairs
        .iter()
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    Some(
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string(),
    )
}

/// Assemble the bottom-first layer tree from flat records. Section dividers
/// (`'lsct'` type 3) open a group; folder records (types 1/2) close it. Nesting
/// deeper than [`MAX_GROUP_DEPTH`] is refused.
fn build_tree(raws: Vec<RawLayer>) -> Result<LayerStack, PsdError> {
    let mut stack: Vec<Vec<Layer>> = vec![Vec::new()];
    for raw in raws {
        match raw.section {
            Some(SECTION_DIVIDER) => {
                if stack.len() > MAX_GROUP_DEPTH {
                    return Err(PsdError::Unsupported(format!(
                        "layer groups nested deeper than {MAX_GROUP_DEPTH}"
                    )));
                }
                stack.push(Vec::new());
            }
            Some(SECTION_OPEN_FOLDER) | Some(SECTION_CLOSED_FOLDER) => {
                let mut group = raw.layer;
                group.is_group = true;
                group.channels = Vec::new();
                group.mask = None;
                // A folder with no open divider closes an empty group; the
                // root level is never popped.
                let children = if stack.len() > 1 {
                    stack.pop().unwrap_or_default()
                } else {
                    Vec::new()
                };
                group.children = LayerStack::from_layers(children);
                stack.last_mut().unwrap().push(group);
            }
            _ => stack.last_mut().unwrap().push(raw.layer),
        }
    }
    // Unbalanced section dividers (bounding sections with no folder) are
    // flattened into their parent rather than dropped.
    while stack.len() > 1 {
        let contents = stack.pop().unwrap();
        stack.last_mut().unwrap().extend(contents);
    }
    Ok(LayerStack::from_layers(stack.pop().unwrap()))
}

#[cfg(test)]
mod tests;
