use pictura_color::{ConversionOptions, Policy, Profile};

use pictura_core::*;

use crate::budget::DecodeBudget;
use crate::common::*;
use crate::depth::{depth_bits, planar_len, row_bytes, undo_prediction};
use crate::error::PsdError;
use crate::mask_params;
use crate::probe::ImageBudget;

mod decode;
mod header_mode;
mod merged;
mod normalize;
mod record;
use record::read_layer_record;
mod stream;
mod tagged_layers;

#[cfg(test)]
pub(crate) use decode::decode_rle_channel;
pub(crate) use decode::inflate;
pub(crate) use decode::{count_bytes, decode_channel_data, decode_packbits, read_rle_count};
use decode::{plan_channel_tasks, read_rle, read_rle_from};
use header_mode::{color_mode_from_code, palette_from};
use normalize::normalize;

/// The deepest group nesting a file may declare. CS6 nests groups ten deep;
/// the cap keeps the recursive compositor and `Drop` off the stack limit.
pub(crate) const MAX_GROUP_DEPTH: usize = 64;

/// Options for opening a PSD/PSB: the incoming-profile policy, an optional
/// working CMYK profile, and the conversion options used by a policy pass.
#[derive(Debug, Clone, Default)]
pub struct OpenOptions {
    pub policy: Policy,
    pub working_cmyk: Option<Vec<u8>>,
    pub conversion: ConversionOptions,
    /// Skip decoding the merged image of an 8-bit RGB file with layers and no
    /// extra channels: the application keeps no composite for such a document
    /// and composites what it shows (#1216), so decoding a canvas-sized image
    /// only to drop it is the open's largest allocation. The document still
    /// records that the file carried one, and its composite is empty.
    pub skip_layered_merged: bool,
    /// The opened file, pinned (#1218): its 8-bit raw and RLE layer channels
    /// are indexed and backed by it instead of decoded. `bytes` must be this
    /// file's contents. `None` decodes everything at open.
    pub source: Option<std::sync::Arc<crate::SourcePin>>,
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
    read_psd_with_image_policy(bytes, options, budget, false)
}

/// Read the layer structure independently of an unusable merged image.
pub fn read_psd_layer_structure(
    bytes: &[u8],
    options: &OpenOptions,
    budget: ImageBudget,
) -> Result<Document, PsdError> {
    read_psd_with_image_policy(bytes, options, budget, true)
}

fn read_psd_with_image_policy(
    bytes: &[u8],
    options: &OpenOptions,
    budget: ImageBudget,
    skip_image_data: bool,
) -> Result<Document, PsdError> {
    let mut doc = read_container(
        bytes,
        options,
        &mut DecodeBudget::new(budget),
        &mut None,
        skip_image_data,
    )?;
    resolve_document_resources(&mut doc);
    Ok(doc)
}

pub(crate) fn resolve_document_resources(doc: &mut Document) {
    crate::path_resources::resolve_paths(doc);
    crate::guide_resources::resolve_guides(doc);
    crate::variable_resources::resolve(doc);
    crate::layer_comps::resolve(doc);
    crate::global_light::resolve_global_light(doc);
}
pub(crate) fn read_container(
    bytes: &[u8],
    options: &OpenOptions,
    budget: &mut DecodeBudget,
    preview: &mut Option<crate::MergedPreview>,
    skip_image_data: bool,
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
        v => return Err(PsdError::NativeVersion(v)),
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
    // The composite's planes are charged once the layer section says which are
    // allocated, and always before they are (#1273): a skipped merged image
    // allocates only its extra channels, and a missing one a tiled blank.
    let charge_composite = |budget: &mut DecodeBudget, planes: usize| {
        (0..planes).try_for_each(|_| {
            charge_plane(
                budget,
                "composite plane",
                width as usize,
                height as usize,
                depth,
            )
        })
    };

    // Color mode data section: 4-byte length + opaque bytes, kept verbatim.
    // An Indexed palette is interpreted and consumed (see `normalize`).
    let color_mode_len = r.u32()? as usize;
    let color_mode_data = r.take(color_mode_len)?.to_vec();
    let palette = palette_from(mode, &color_mode_data)?;
    // Image resources section: 4-byte length + opaque bytes, kept verbatim.
    let resources_len = r.u32()? as usize;
    let image_resources = r.take(resources_len)?.to_vec();
    // Layer and mask information section: 4-byte length (8 in PSB).
    let (mut layers, global_layer_mask, layer_section_extra, layer_compression, merged_alpha) =
        read_layer_section(
            &mut r,
            is_psb,
            color_count,
            depth,
            retain_planes,
            budget,
            options.source.as_ref(),
        )?;
    let merged_alpha = merged_alpha && mode == ColorMode::Rgb;
    // Derive the smart-object view from the preserved bytes; a malformed
    // descriptor or linked-layer record degrades to Unresolved (design D5).
    crate::smart_object::resolve_smart_objects_within(
        &mut layers,
        &layer_section_extra,
        is_psb,
        budget,
    )?;
    // Derive the vector-mask view from the preserved `vmsk`/`vsms` blocks.
    // ponytail: this is a read snapshot. In-session resize/crop/orientation do
    // not re-derive it; the raw block stays authoritative and a reload re-derives.
    crate::vector_mask::resolve_vector_masks(&mut layers, width, height);
    // Derive the type-tool view from the preserved `TySh` blocks, then apply
    // the document-level `Txt2` text-frame paths to those layers.
    crate::type_tool::resolve_type_tools_within(&mut layers, budget)?;
    crate::type_path::resolve_text_paths_within(&mut layers, &layer_section_extra, is_psb, budget)?;
    crate::normalize_content::normalize_tree_within(&mut layers, budget)?;

    // "Maximize Compatibility" off: a layered file may end after the layer
    // section with no merged composite. A file with no layers at all and no
    // trailing data is still malformed and falls through to the truncation error.
    let layered = !layers.is_empty();
    let real_merged =
        !layered || crate::compatibility::has_real_merged_data(&image_resources) != Some(false);
    let sections = merged::Sections {
        width,
        height,
        mode,
        source_depth,
        layer_compression: layer_compression.unwrap_or_default(),
        is_psb,
        layers,
        color_mode_data,
        image_resources,
        global_layer_mask,
        layer_section_extra,
    };
    if skip_image_data || r.remaining() == 0 && layered {
        // The blank stand-in is tiled zeros, which hold no allocation.
        let blank = PixelBuffer {
            width,
            height,
            channels: color_count as u8,
            data: Plane::filled(width as usize, height as usize, color_count, 0u8),
        };
        let doc =
            merged::without_merged_image(sections, blank, false, Compression::Rle, Vec::new());
        return Ok(crate::icc::apply_icc_policy_with_options(
            normalize(doc, mode, depth, palette.as_ref(), options, false),
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
    let skippable_rgb = mode == ColorMode::Rgb && depth == 8;
    let skip_preview = options.skip_layered_merged && layered && skippable_rgb;
    let skip_placeholder =
        !real_merged && (skippable_rgb || (depth >= 8 && header_channels == color_count));
    if (skip_preview || skip_placeholder)
        && matches!(composite_compression, Compression::Rle | Compression::Raw)
    {
        // Only the extra channels (saved selections, merged transparency)
        // are decoded; the colour planes' bytes are skipped, or indexed as a
        // preview for a lazy open (#1218).
        if let Some(pin) = options.source.as_ref().filter(|_| real_merged) {
            *preview = crate::MergedPreview::index(
                pin,
                r.data,
                r.pos,
                compression,
                header_channels,
                width,
                height,
                is_psb,
            )
            .ok();
        }
        charge_composite(budget, header_channels.saturating_sub(color_count))?;
        let colour_bytes = planar_len(color_count, stride, height)?;
        let extras = match composite_compression {
            Compression::Rle => {
                let rows = header_channels
                    .checked_mul(height)
                    .ok_or_else(|| PsdError::Invalid("image dimensions overflow".into()))?;
                let per_row =
                    2 * std::mem::size_of::<usize>() + std::mem::size_of::<Result<(), PsdError>>();
                budget.charge("RLE row metadata", &[rows, per_row])?;
                read_rle_from(&mut r, header_channels, color_count, stride, height, is_psb)?
            }
            _ => r.take(planar_len(header_channels, stride, height)?)?[colour_bytes..].to_vec(),
        };
        let composite = if real_merged {
            PixelBuffer::new(0, 0, color_count as u8)
        } else {
            PixelBuffer {
                width: width as u32,
                height: height as u32,
                channels: color_count as u8,
                data: Plane::filled(width, height, color_count, 0),
            }
        };
        let mut doc = merged::without_merged_image(
            sections,
            composite,
            real_merged,
            composite_compression,
            merged::extra_channels(
                &extras[if merged_alpha && header_channels > color_count {
                    stride * height
                } else {
                    0
                }..],
                stride * height,
            ),
        );
        doc.composite
            .set_signed_merged_alpha(merged_alpha && real_merged && header_channels > color_count);
        return Ok(crate::icc::apply_icc_policy_with_options(
            normalize(doc, mode, depth, palette.as_ref(), options, false),
            options.policy,
            options.conversion,
        ));
    }
    charge_composite(budget, header_channels)?;
    let mut data = match composite_compression {
        Compression::Raw => r
            .take(planar_len(header_channels, stride, height)?)?
            .to_vec(),
        Compression::Rle => {
            // The RLE decode builds a count vector, a prefix-sum offset vector,
            // and one result per row; charge that metadata before allocating it.
            let rows = header_channels
                .checked_mul(height)
                .ok_or_else(|| PsdError::Invalid("image dimensions overflow".into()))?;
            let per_row =
                2 * std::mem::size_of::<usize>() + std::mem::size_of::<Result<(), PsdError>>();
            budget.charge("RLE row metadata", &[rows, per_row])?;
            read_rle(&mut r, header_channels, stride, height, is_psb)?
        }
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
    let source_planes = native.map(|samples| {
        let samples = if merged_alpha && !real_merged && header_channels > color_count {
            let mut bytes = samples.to_bytes();
            let start = color_count * stride * height;
            bytes.drain(start..start + stride * height);
            Samples::from_bytes(&bytes, store_depth)
        } else {
            samples
        };
        SourcePlanes {
            depth: store_depth,
            width: width as u32,
            height: height as u32,
            samples,
        }
    });
    let composite_channels =
        color_count + usize::from(merged_alpha && real_merged && header_channels > color_count);
    let (mut composite, mut channels) = split_planes(
        data,
        composite_channels,
        width,
        height,
        plane,
        header_channels,
    )?;
    if merged_alpha && !real_merged && header_channels > color_count && !channels.is_empty() {
        channels.remove(0);
        for (index, channel) in channels.iter_mut().enumerate() {
            channel.id = index as i16;
        }
    }

    let bitmap_composite_decoded = mode == ColorMode::Bitmap && depth == 1;
    if bitmap_composite_decoded {
        composite =
            normalize::convert_pixels(composite, mode, depth, None, None, options.conversion);
    }

    if !real_merged {
        composite.data = Plane::filled(width, height, usize::from(composite.channels), 0);
    }
    let keep_native = real_merged || !channels.is_empty();
    let mut doc = merged::without_merged_image(
        sections,
        composite,
        real_merged,
        composite_compression,
        channels,
    );
    doc.composite
        .set_signed_merged_alpha(merged_alpha && real_merged && header_channels > color_count);
    doc.source
        .replace_planes(source_planes.filter(|_| keep_native))
        .expect("valid source planes");
    Ok(crate::icc::apply_icc_policy_with_options(
        normalize(
            doc,
            mode,
            depth,
            palette.as_ref(),
            options,
            bitmap_composite_decoded,
        ),
        options.policy,
        options.conversion,
    ))
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
type LayerSection = (LayerStack, Vec<u8>, Vec<u8>, Option<Compression>, bool);

fn read_layer_section(
    r: &mut Reader,
    is_psb: bool,
    color_channels: usize,
    depth: u16,
    retain: bool,
    budget: &mut DecodeBudget,
    source: Option<&std::sync::Arc<crate::SourcePin>>,
) -> Result<LayerSection, PsdError> {
    let section_len = if is_psb {
        r.u64()? as usize
    } else {
        r.u32()? as usize
    };
    if section_len == 0 {
        return Ok((LayerStack::default(), Vec::new(), Vec::new(), None, false));
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
    let mut merged_alpha = false;
    if info_len != 0 {
        let info_end = r
            .pos
            .checked_add(info_len)
            .ok_or_else(|| PsdError::Invalid("layer info length overflow".into()))?;
        if info_end > section_end {
            return Err(PsdError::Invalid("layer info exceeds layer section".into()));
        }
        (layers, layer_compression, merged_alpha) = read_layer_info(
            r,
            is_psb,
            info_end,
            color_channels,
            depth,
            retain,
            budget,
            source,
        )?;
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
    let mut layer_section_extra = r.take(extra_len)?.to_vec();
    if info_len == 0 {
        let tagged = tagged_layers::read_tagged_layer_info(
            &mut layer_section_extra,
            is_psb,
            color_channels,
            depth,
            retain,
            budget,
        )?;
        if let Some(read) = tagged {
            (layers, layer_compression, merged_alpha) = read;
        }
        merged_alpha |=
            crate::compatibility::has_merged_transparency_marker(&layer_section_extra, is_psb);
    }
    Ok((
        layers,
        global_layer_mask,
        layer_section_extra,
        layer_compression,
        merged_alpha,
    ))
}

#[allow(clippy::too_many_arguments)]
fn read_layer_info(
    r: &mut Reader,
    is_psb: bool,
    info_end: usize,
    color_channels: usize,
    depth: u16,
    retain: bool,
    budget: &mut DecodeBudget,
    source: Option<&std::sync::Arc<crate::SourcePin>>,
) -> Result<(LayerStack, Option<Compression>, bool), PsdError> {
    let count = r.i16()?;
    let n = count.unsigned_abs() as usize;
    let mut raws: Vec<RawLayer> = Vec::new();
    for _ in 0..n {
        raws.push(read_layer_record(r, is_psb)?);
    }

    // Phase A (serial): locate every channel from the parsed records and charge
    // the decode budget in record order. Channel data starts where the records
    // end; every channel's slice is the rest of the file from its offset, so no
    // task depends on another's cursor.
    let base = r.pos;
    let channel_count: usize = raws.iter().map(|raw| raw.channel_ids.len()).sum();
    let per_channel = std::mem::size_of::<decode::PlannedChannel>()
        + std::mem::size_of::<decode::ChannelTask>()
        + std::mem::size_of::<Result<decode::ChannelRead, PsdError>>();
    budget.charge("layer channel metadata", &[channel_count, per_channel])?;
    let mut planned = Vec::with_capacity(channel_count);
    for raw in &raws {
        let layer_w = raw.layer.rect.width().max(0) as usize;
        let layer_h = raw.layer.rect.height().max(0) as usize;
        let mask_dims = raw.layer.mask.as_ref().map(|m| {
            (
                m.rect.width().max(0) as usize,
                m.rect.height().max(0) as usize,
            )
        });
        for (&id, &len) in raw.channel_ids.iter().zip(raw.channel_lens.iter()) {
            // The user layer mask channel (-2) is sized by the mask rect, which
            // may differ from the layer rect.
            let is_color = id >= 0 && (id as usize) < color_channels;
            let modeled = id == -2 || id == -1 || is_color;
            let (width, height) = if modeled && id == -2 {
                mask_dims.unwrap_or((layer_w, layer_h))
            } else {
                (layer_w, layer_h)
            };
            planned.push(decode::PlannedChannel {
                id,
                len,
                width,
                height,
            });
        }
    }
    let (tasks, total) = plan_channel_tasks(&planned, color_channels, depth, retain, budget)?;

    // Phase B (parallel): decode every channel from its own slice, a row at a
    // time into tiles.
    let mut results =
        decode::decode_channel_tasks(r.data, base, &tasks, is_psb, depth, retain, source)
            .into_iter();

    // Phase C (serial): reassemble in record order; the first error in that
    // order is the one returned.
    let mut layer_compression = None;
    for raw in raws.iter_mut() {
        let mut channels = Vec::new();
        let mut raw_channels = Vec::new();
        let mut mask_data = None;
        let mut retained: Vec<(i16, Samples)> = Vec::new();
        for &id in raw.channel_ids.iter() {
            let (data, code, native) = results
                .next()
                .ok_or_else(|| PsdError::Invalid("missing layer channel decode".into()))??;
            let is_color = id >= 0 && (id as usize) < color_channels;
            if retain {
                if let Some(native) = native {
                    retained.push((id, native));
                }
            }
            // Channels outside the modeled set are preserved verbatim at depth
            // 8 and re-wrapped as raw 8-bit streams at depth 1/16/32.
            if id != -2 && id != -1 && !is_color {
                let data = if depth == 8 {
                    data
                } else {
                    let mut stream = Vec::with_capacity(2 + data.len());
                    stream.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
                    stream.extend_from_slice(&data);
                    stream.into()
                };
                raw_channels.push(RawChannel { id, data });
                continue;
            }
            // Section dividers and folder records carry placeholder channels that
            // `build_tree` drops; only a normal record's channel is a survivor.
            if raw.section.is_none() && layer_compression.is_none() {
                layer_compression = Compression::from_code(code);
            }
            match id {
                -2 => mask_data = Some(data),
                _ => channels.push(Channel { id, data }),
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
        raw.layer.set_channels(channels);
        let content = raw.layer.content().clone();
        let mut parts = pictura_core::LayerParts::from(std::mem::take(&mut raw.layer));
        parts.raw_channels = raw_channels;
        raw.layer = pictura_core::Layer::from_parts(parts, content);
        if let Some(data) = mask_data {
            match raw.layer.mask.as_mut() {
                Some(mask) => mask.data = Some(data),
                None => {
                    raw.layer.mask = Some(LayerMask {
                        rect: raw.layer.rect,
                        default_color: 0,
                        disabled: false,
                        flags: 0,
                        data: Some(data.to_vec().into()),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // The records and channel data must still fit the declared layer info; a
    // channel declared past it is refused even though its bytes were in file.
    r.pos = base.saturating_add(total);
    if r.pos > info_end {
        return Err(PsdError::Invalid("layer records exceed layer info".into()));
    }
    r.pos = info_end;
    let mut layers = build_tree(raws)?;
    derive_background(&mut layers);
    Ok((layers, layer_compression, count < 0))
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

/// PSD has no background bit: the bottom top-level, non-group layer named
/// `"Background"` is the Background (design D5, marked inferred).
fn derive_background(layers: &mut LayerStack) {
    if let Some(bottom) = layers.first_mut() {
        if !bottom.is_group() && bottom.name == "Background" {
            bottom.background = true;
        }
    }
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
                group.set_group_children(pictura_core::LayerStack::default());
                group.set_channels(Vec::new());
                group.mask = None;
                // A folder with no open divider closes an empty group; the
                // root level is never popped.
                let children = if stack.len() > 1 {
                    stack.pop().unwrap_or_default()
                } else {
                    Vec::new()
                };
                group.set_group_children(LayerStack::from_layers(children));
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
