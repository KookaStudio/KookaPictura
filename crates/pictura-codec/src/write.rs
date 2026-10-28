use pictura_core::*;
use std::borrow::Cow;
use std::sync::Arc;

use crate::common::*;
use crate::depth::{apply_prediction, depth_bits, depth_of, output_depth, row_bytes};
use crate::error::PsdError;
use crate::write_cache::{CompositeKey, EncodeCache};
use crate::write_indexed::writes_indexed;
use crate::write_layers::write_layer_info;

// ---------------------------------------------------------------------------
// Layer and mask information section — writing
// ---------------------------------------------------------------------------

/// The Lab bytes to emit for a Lab output's composite color planes: the
/// retained Lab planes exactly when every plane's forward RGB conversion still
/// matches the working composite, else the working RGB re-encoded with the
/// approximate inverse. The document is never mutated. The retained plane is
/// the 8-bit Lab read store at depth 8, or the native store narrowed to 8-bit
/// at 16/32; for an RGB/Grayscale document there is none.
///
/// ponytail: the three Lab planes are re-encoded as a unit, so editing one
/// channel also re-encodes its unchanged siblings (Lab `a`/`b` depend on all of
/// R/G/B). Per-channel change tracking would be the upgrade if that matters.
fn lab_composite_planes(doc: &Document, depth: u16, plane: usize) -> Vec<u8> {
    let color_channels = doc.composite.channels as usize;
    let current = &doc.composite.data[..color_channels * plane];
    let retained: Option<Vec<Vec<u8>>> = (0..color_channels)
        .map(|c| crate::color_mode::composite_retained_8(doc, depth, c))
        .collect();
    if let Some(retained) = retained {
        let retained: Vec<u8> = retained.concat();
        if retained.len() == plane * color_channels
            && crate::color_mode::lab_to_rgb(&retained).as_slice() == current
        {
            return retained;
        }
    }
    crate::color_mode::rgb_to_lab(current)
}

pub(crate) fn write_tag(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8], psb: bool) {
    // A per-layer block: the reference declares an even length with the pad byte
    // inside it; psd-tools reads exactly the declared length (padding=1 means no
    // external pad), so an odd declared length would mis-frame the next block.
    let declared = data.len() + (data.len() & 1);
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    if psb && is_psb_big_key(key) {
        out.extend_from_slice(&(declared as u64).to_be_bytes());
    } else {
        out.extend_from_slice(&(declared as u32).to_be_bytes());
    }
    out.extend_from_slice(data);
    if declared != data.len() {
        out.push(0);
    }
}

/// A document-level (global) additional-layer-information block. psd-tools reads
/// these with `TaggedBlocks.read(..., padding=4)`: the declared length is the
/// exact data length and the block is padded externally to a 4-byte boundary.
/// The big-key width rule is the same as for per-layer blocks.
pub(crate) fn write_tag_document(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8], psb: bool) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    if psb && is_psb_big_key(key) {
        out.extend_from_slice(&(data.len() as u64).to_be_bytes());
    } else {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(data);
    let pad = (4 - data.len() % 4) % 4;
    out.extend(std::iter::repeat_n(0u8, pad));
}

/// Re-frame preserved document-level tagged blocks for the destination
/// container: each block's data is copied verbatim and its exact length is
/// re-emitted in the destination width, then the block is re-padded externally
/// to a 4-byte boundary. When the source and destination widths agree the bytes
/// are returned unchanged. An unparseable block stops the walk and the remaining
/// bytes are copied verbatim.
fn reframe_document_extra(bytes: &[u8], src_psb: bool, dst_psb: bool) -> Vec<u8> {
    if src_psb == dst_psb {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut r = Reader::new(bytes);
    loop {
        let start = r.pos;
        let Ok(sig) = r.take(4) else { break };
        if sig != b"8BIM" && sig != b"8B64" {
            r.pos = start;
            break;
        }
        let Ok(key) = r.take(4) else {
            r.pos = start;
            break;
        };
        let key: [u8; 4] = key.try_into().unwrap();
        let len = if src_psb && is_psb_big_key(&key) {
            match r.u64() {
                Ok(len) => usize::try_from(len).unwrap_or(usize::MAX),
                Err(_) => {
                    r.pos = start;
                    break;
                }
            }
        } else {
            match r.u32() {
                Ok(len) => len as usize,
                Err(_) => {
                    r.pos = start;
                    break;
                }
            }
        };
        let Ok(data) = r.take(len) else {
            r.pos = start;
            break;
        };
        // A global tagged block is padded externally to a 4-byte boundary.
        let pad = (4 - len % 4) % 4;
        if r.skip(pad).is_err() {
            r.pos = start;
            break;
        }
        write_tag_document(&mut out, &key, data, dst_psb);
    }
    out.extend_from_slice(&bytes[r.pos..]);
    out
}

// ---------------------------------------------------------------------------
// PackBits RLE encoding
// ---------------------------------------------------------------------------

/// Append one PackBits-encoded scanline to `out`. Runs of three or more equal
/// bytes become a repeat packet (control `1 - n`, then the byte); all other
/// bytes become literal packets of up to 128. The split is fixed, so repeated
/// writes are byte-identical.
fn encode_packbits_row(row: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < row.len() {
        let mut run = 1;
        while i + run < row.len() && row[i + run] == row[i] && run < 128 {
            run += 1;
        }
        if run >= 3 {
            out.push((1 - run as i32) as i8 as u8);
            out.push(row[i]);
            i += run;
            continue;
        }
        let start = i;
        let mut end = i;
        while end < row.len() && end - start < 128 {
            if end + 2 < row.len() && row[end] == row[end + 1] && row[end] == row[end + 2] {
                break;
            }
            end += 1;
        }
        out.push((end - start - 1) as u8);
        out.extend_from_slice(&row[start..end]);
        i = end;
    }
}

/// The native-depth plane to emit for one channel. When `depth` is 8 the
/// current plane is borrowed unchanged. At 16/32, `retained` (the decoded source
/// samples, when its length matches the plane) is used if narrowing it yields
/// `current`; otherwise `current` is widened. A length other than exactly
/// `width * height` is rejected, so widening cannot index out of bounds.
pub(crate) fn native_plane<'a>(
    retained: Option<&Samples>,
    current: &'a [u8],
    width: usize,
    height: usize,
    depth: u16,
) -> Result<Cow<'a, [u8]>, PsdError> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel plane size overflow".into()))?;
    if current.len() != pixels {
        return Err(PsdError::Invalid("channel plane length mismatch".into()));
    }
    if depth == 8 {
        return Ok(Cow::Borrowed(current));
    }
    let native = depth_bits(depth).ok_or_else(|| PsdError::Invalid("sample depth".into()))?;
    if let Some(ret) = retained {
        if ret.len() == pixels && ret.narrow_to_u8() == current {
            return Ok(Cow::Owned(ret.to_bytes()));
        }
    }
    Ok(Cow::Owned(
        Samples::widen_from_u8(current, native).to_bytes(),
    ))
}

/// The retained native plane at `index` (composite color channels then document
/// extras) when the document's store matches the output `depth` and canvas.
pub(crate) fn composite_retained(doc: &Document, depth: u16, index: usize) -> Option<Samples> {
    let store = doc.source_planes.as_ref()?;
    if depth_of(Some(store.depth)) != depth
        || store.width != doc.width
        || store.height != doc.height
    {
        return None;
    }
    if store.depth == BitDepth::One {
        return (index == 0).then(|| store.samples.clone());
    }
    let plane = doc.width as usize * doc.height as usize;
    store.samples.slice(index * plane..(index + 1) * plane)
}

/// The retained native plane for layer channel `id`, when the layer's store
/// matches the output `depth` and the layer has not moved.
pub(crate) fn layer_retained(layer: &Layer, depth: u16, id: i16) -> Option<Samples> {
    let store = layer.source_channels.as_ref()?;
    if depth_of(Some(store.depth)) != depth || store.rect != layer.rect {
        return None;
    }
    store
        .planes
        .iter()
        .find(|(channel, _)| *channel == id)
        .map(|(_, samples)| samples.clone())
}

/// Encode `planes` (each `row_bytes * height`, plane-major) into one RLE
/// payload: all 2-byte (PSD) or 4-byte (PSB) scanline byte counts first
/// (plane-major, then row-major), then the packed rows in the same order. The
/// compression word is not included. PackBits is byte-wise, so `row_bytes` is
/// the native row stride (`width` at depth 8, `2 * width` at 16, `4 * width` at
/// 32).
pub(crate) fn encode_scanlines(
    planes: &[&[u8]],
    row_bytes: usize,
    height: usize,
    psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let plane_len = row_bytes
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("RLE plane size overflow".into()))?;
    let count_width = if psb { 4 } else { 2 };
    let mut counts = Vec::with_capacity(planes.len() * height * count_width);
    let mut rows = Vec::new();
    let mut packed = Vec::with_capacity(row_bytes + row_bytes / 128 + 1);
    for plane in planes {
        if plane.len() != plane_len {
            return Err(PsdError::Invalid("RLE plane length mismatch".into()));
        }
        for row in 0..height {
            let start = row * row_bytes;
            packed.clear();
            encode_packbits_row(&plane[start..start + row_bytes], &mut packed);
            // The PSD maximum width (30 000) cannot reach the u16 limit, so the
            // guard only fires for a PSB row approaching u32; it errs rather
            // than truncate. ponytail: one uncompressed row, not the whole plane.
            if psb {
                if packed.len() > u32::MAX as usize {
                    return Err(PsdError::Invalid(
                        "RLE scanline exceeds u32 byte count".into(),
                    ));
                }
                counts.extend_from_slice(&(packed.len() as u32).to_be_bytes());
            } else {
                if packed.len() > u16::MAX as usize {
                    return Err(PsdError::Invalid(
                        "RLE scanline exceeds u16 byte count".into(),
                    ));
                }
                counts.extend_from_slice(&(packed.len() as u16).to_be_bytes());
            }
            rows.extend_from_slice(&packed);
        }
    }
    counts.extend_from_slice(&rows);
    Ok(counts)
}

/// Concatenate the native-depth `planes` (`width` pixels by `height` rows each,
/// plane-major) and zlib-wrap the result. When `predict`, apply the forward
/// depth-specific per-row delta first (the inverse of the reader's
/// `undo_prediction`); no scanline count table.
pub(crate) fn zip_scanlines(
    planes: &[&[u8]],
    width: usize,
    height: usize,
    predict: bool,
    depth: u16,
) -> Result<Vec<u8>, PsdError> {
    let plane_len = row_bytes(width, depth)
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("ZIP plane size overflow".into()))?;
    let mut data = Vec::with_capacity(planes.len() * plane_len);
    for plane in planes {
        if plane.len() != plane_len {
            return Err(PsdError::Invalid("ZIP plane length mismatch".into()));
        }
        data.extend_from_slice(plane);
    }
    if predict {
        apply_prediction(&mut data, width, planes.len() * height, depth);
    }
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&data)
        .map_err(|_| PsdError::Invalid("ZIP encode".into()))?;
    enc.finish()
        .map_err(|_| PsdError::Invalid("ZIP encode".into()))
}

/// The complete on-disk layer-channel stream for one engine-encoded plane: the
/// compression word followed by the payload for `kind` at `depth`. `plane` is
/// the native-depth plane (`row_bytes(width, depth) * height` bytes).
pub(crate) fn channel_stream(
    kind: Compression,
    width: usize,
    height: usize,
    plane: &[u8],
    depth: u16,
    psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let stride = row_bytes(width, depth);
    let mut out = Vec::with_capacity(2 + plane.len());
    out.extend_from_slice(&kind.to_code().to_be_bytes());
    match kind {
        Compression::Rle => {
            out.extend_from_slice(&encode_scanlines(&[plane], stride, height, psb)?)
        }
        Compression::Raw => {
            if plane.len() != stride * height {
                return Err(PsdError::Invalid("raw channel length mismatch".into()));
            }
            out.extend_from_slice(plane);
        }
        Compression::Zip => {
            out.extend_from_slice(&zip_scanlines(&[plane], width, height, false, depth)?)
        }
        Compression::ZipPrediction => {
            out.extend_from_slice(&zip_scanlines(&[plane], width, height, true, depth)?)
        }
    }
    Ok(out)
}

/// Serialize a [`Document`] into a valid PSD, or into a version-2 PSB when the
/// source document was a PSB or a dimension exceeds the PSD limit.
pub fn write_psd(doc: &Document) -> Result<Vec<u8>, PsdError> {
    let psb = doc.is_psb || doc.width > MAX_DIM_PSD || doc.height > MAX_DIM_PSD;
    write_container(doc, psb, None)
}

/// Serialize a [`Document`] into a version-2 PSB regardless of its dimensions.
pub fn write_psb(doc: &Document) -> Result<Vec<u8>, PsdError> {
    write_container(doc, true, None)
}

/// As [`write_psd`], reusing encoded layer channels and the composite block from
/// `cache`. The bytes equal an uncached encode.
pub fn write_psd_cached(doc: &Document, cache: &EncodeCache) -> Result<Vec<u8>, PsdError> {
    let psb = doc.is_psb || doc.width > MAX_DIM_PSD || doc.height > MAX_DIM_PSD;
    write_container(doc, psb, Some(cache))
}

/// As [`write_psb`], reusing encoded layer channels and the composite block from
/// `cache`. The bytes equal an uncached encode.
pub fn write_psb_cached(doc: &Document, cache: &EncodeCache) -> Result<Vec<u8>, PsdError> {
    write_container(doc, true, Some(cache))
}

fn write_container(
    doc: &Document,
    psb: bool,
    cache: Option<&EncodeCache>,
) -> Result<Vec<u8>, PsdError> {
    if doc.depth != BitDepth::Eight {
        return Err(PsdError::Unsupported("write supports 8-bit only".into()));
    }
    // The working model stays 8-bit; the output depth is the recorded source
    // depth (8 when the read retained no samples, including a converted mode),
    // so an open→save of a 16/32-bit file is not a silent downgrade. A flat,
    // unchanged depth-1 Bitmap source forces depth 1 and header mode Bitmap.
    let bitmap_mode = crate::write_bitmap::writes_bitmap(doc);
    let depth = if bitmap_mode { 1 } else { output_depth(doc)? };
    let plane = doc.width as usize * doc.height as usize;
    // A Lab source writes header mode Lab with three color channels: an 8-bit
    // read re-encodes the working RGB, and a 16/32-bit read re-emits the
    // retained native planes (see `lab_composite_planes`). At depth 8 a
    // construct with no recorded depth still fires; a 16/32 doc does because it
    // retained a native store, hence `depth != 8`.
    let lab_mode = doc.source_mode == Some(ColorMode::Lab)
        && doc.composite.channels == 3
        && (depth != 8 || doc.source_depth.is_none());
    // A CMYK source writes header mode CMYK with four color channels: an 8-bit
    // read re-encodes the working RGB, and a 16/32-bit read re-emits the
    // retained native planes.
    let cmyk_mode = doc.source_mode == Some(ColorMode::Cmyk)
        && doc.composite.channels == 3
        && (depth != 8 || doc.source_depth.is_none());
    // An 8-bit Indexed source writes header mode Indexed with one index channel
    // and the retained palette, but only while the composite and every pixel
    // layer still expand to the working RGB; an edit falls back to RGB.
    let indexed_mode = writes_indexed(doc, depth, plane);
    // Flat unedited Duotone/Multichannel re-emits retained plates; edits fall back to RGB.
    let flat_source = crate::write_duotone::flat_source_mode(doc, depth, plane);
    let mode_code = if bitmap_mode {
        MODE_BITMAP
    } else if let Some((code, _, _)) = flat_source {
        code
    } else if lab_mode {
        MODE_LAB
    } else if cmyk_mode {
        MODE_CMYK
    } else if indexed_mode {
        MODE_INDEXED
    } else {
        match doc.mode {
            ColorMode::Grayscale => MODE_GRAYSCALE,
            ColorMode::Rgb => MODE_RGB,
            m => return Err(PsdError::Unsupported(format!("write color mode {m:?}"))),
        }
    };
    let color_channels = doc.composite.channels as usize;
    // Header channel count follows the output mode: 4 CMYK, 1 Indexed, 1-or-3
    // flat Duotone/Multichannel, else the working count. Composite data stays working-term.
    let out_color_channels = if bitmap_mode {
        1
    } else if cmyk_mode {
        4
    } else if indexed_mode {
        1
    } else if let Some((_, n, _)) = flat_source {
        n
    } else {
        color_channels
    };
    let channels = out_color_channels + doc.channels.len();
    if channels == 0 || channels > MAX_CHANNELS as usize {
        return Err(PsdError::Invalid(format!("channel count {channels}")));
    }
    let max_dim = if psb { MAX_DIM_PSB } else { MAX_DIM_PSD };
    if doc.width == 0 || doc.height == 0 || doc.width > max_dim || doc.height > max_dim {
        return Err(PsdError::Unsupported(format!(
            "dimension {}x{}",
            doc.width, doc.height
        )));
    }
    if doc.composite.width != doc.width || doc.composite.height != doc.height {
        return Err(PsdError::Invalid(
            "composite size does not match document".into(),
        ));
    }
    if doc.composite.data.len() != color_channels * plane {
        return Err(PsdError::Invalid("composite data length mismatch".into()));
    }
    crate::write_bitmap::validate_extra_channels(doc, bitmap_mode)?;

    let mut out = Vec::with_capacity(26 + 12 + 2 + doc.composite.data.len());
    out.extend_from_slice(&SIGNATURE.to_be_bytes());
    out.extend_from_slice(&(if psb { VERSION_PSB } else { VERSION_PSD }).to_be_bytes());
    out.extend_from_slice(&[0u8; 6]); // reserved
    out.extend_from_slice(&(channels as u16).to_be_bytes());
    out.extend_from_slice(&doc.height.to_be_bytes());
    out.extend_from_slice(&doc.width.to_be_bytes());
    out.extend_from_slice(&depth.to_be_bytes()); // depth
    out.extend_from_slice(&mode_code.to_be_bytes());
    // An Indexed output re-emits the retained palette (the read consumed
    // `color_mode_data`); every other mode replays the preserved section.
    let color_mode_data: &[u8] = match (indexed_mode, doc.source_palette.as_ref()) {
        (true, Some(palette)) => palette,
        _ => &doc.color_mode_data,
    };
    out.extend_from_slice(&(color_mode_data.len() as u32).to_be_bytes());
    out.extend_from_slice(color_mode_data);
    let resources = crate::guide_resources::output_resources(doc, mode_code);
    out.extend_from_slice(&(resources.len() as u32).to_be_bytes());
    out.extend_from_slice(&resources);

    // Authored smart objects append a document-level linked-record block; the
    // preserved trailing bytes stay untouched, so an existing file is unchanged.
    // A PSD-sourced document written as a PSB re-frames preserved big-key blocks
    // to u64 lengths so psd-tools can parse them.
    let extra =
        crate::type_path::encode_text_paths(&doc.layers, &doc.layer_section_extra, doc.is_psb);
    let mut extra = reframe_document_extra(&extra, doc.is_psb, psb);
    let authoring = crate::smart_writer::collect_authoring(&doc.layers);
    if !authoring.is_empty() {
        extra.extend_from_slice(&crate::smart_writer::author_lnk2_bytes(&authoring, psb));
    }

    let mut used_layer_keys = Vec::new();
    if doc.layers.is_empty() && doc.global_layer_mask.is_empty() && extra.is_empty() {
        if psb {
            out.extend_from_slice(&0u64.to_be_bytes()); // zero-length layer/mask section
        } else {
            out.extend_from_slice(&0u32.to_be_bytes()); // zero-length layer/mask section
        }
    } else {
        let (info, keys) = write_layer_info(doc, psb, lab_mode, cmyk_mode, indexed_mode, cache)?;
        used_layer_keys = keys;
        let len_width = if psb { 8 } else { 4 };
        let section_len = len_width + info.len() + 4 + doc.global_layer_mask.len() + extra.len();
        if psb {
            out.extend_from_slice(&(section_len as u64).to_be_bytes());
            out.extend_from_slice(&(info.len() as u64).to_be_bytes());
        } else {
            out.extend_from_slice(&(section_len as u32).to_be_bytes());
            out.extend_from_slice(&(info.len() as u32).to_be_bytes());
        }
        out.extend_from_slice(&info);
        out.extend_from_slice(&(doc.global_layer_mask.len() as u32).to_be_bytes());
        out.extend_from_slice(&doc.global_layer_mask);
        out.extend_from_slice(&extra);
    }

    // "Maximize Compatibility" off: the source ended after the layer section, so
    // emit no image-data section; the reader then reports no merged composite.
    if !doc.merged_composite_present {
        if let Some(cache) = cache {
            cache.prune(&used_layer_keys, None);
        }
        return Ok(out);
    }

    let width = doc.width as usize;
    let height = doc.height as usize;
    let cacheable_composite = cache.is_some()
        && doc.source_mode.is_none()
        && depth == 8
        && !lab_mode
        && !cmyk_mode
        && !indexed_mode
        && flat_source.is_none()
        && !bitmap_mode;
    let composite_key = cacheable_composite.then(|| CompositeKey {
        ident: doc.composite.data.ident(),
        extra_idents: doc
            .channels
            .iter()
            .map(|channel| channel.data.ident())
            .collect(),
        width: doc.width,
        height: doc.height,
        channels: channels as u16,
        mode_code,
        depth,
        compression: doc.composite_compression.to_code(),
        psb,
    });
    if let (Some(cache), Some(key)) = (cache, composite_key.as_ref()) {
        if let Some(bytes) = cache.get_composite(key) {
            out.extend_from_slice(&bytes);
            cache.prune(&used_layer_keys, Some(key));
            return Ok(out);
        }
    }
    let block_start = out.len();
    out.extend_from_slice(&doc.composite_compression.to_code().to_be_bytes());
    // The Lab, CMYK, or Indexed output re-emits retained planes exactly when
    // they are unchanged, and otherwise re-encodes from the working RGB into a
    // temp buffer (Indexed never re-encodes: an edit already chose the RGB mode).
    let lab_composite = lab_mode.then(|| lab_composite_planes(doc, depth, plane));
    let cmyk_composite =
        cmyk_mode.then(|| crate::write_cmyk::cmyk_composite_planes(doc, depth, plane));
    let indexed_composite = indexed_mode.then(|| {
        composite_retained(doc, depth, 0)
            .expect("indexed_mode checked the composite plane")
            .to_bytes()
    });
    let flat_composite = flat_source.as_ref().map(|(_, _, planes)| planes);
    let planes: Vec<Cow<[u8]>> = if bitmap_mode {
        crate::write_bitmap::bitmap_planes(doc)?
    } else {
        let mut planes = Vec::with_capacity(channels);
        for c in 0..out_color_channels {
            let current = if let Some(lab) = &lab_composite {
                &lab[c * plane..(c + 1) * plane]
            } else if let Some(cmyk) = &cmyk_composite {
                &cmyk[c * plane..(c + 1) * plane]
            } else if let Some(indexed) = &indexed_composite {
                indexed.as_slice()
            } else if let Some(flat) = &flat_composite {
                &flat[c * plane..(c + 1) * plane]
            } else {
                &doc.composite.data[c * plane..(c + 1) * plane]
            };
            planes.push(native_plane(
                composite_retained(doc, depth, c).as_ref(),
                current,
                width,
                height,
                depth,
            )?);
        }
        if crate::write_bitmap::is_bitmap_source(doc) {
            for expanded in crate::write_bitmap::fallback_extra_planes(doc)? {
                planes.push(Cow::Owned(expanded));
            }
        } else {
            for (i, channel) in doc.channels.iter().enumerate() {
                planes.push(native_plane(
                    // A CMYK output stores four color planes before the extras,
                    // so the retained extras offset is the output color count.
                    composite_retained(doc, depth, out_color_channels + i).as_ref(),
                    &channel.data,
                    width,
                    height,
                    depth,
                )?);
            }
        }
        planes
    };
    let planes: Vec<&[u8]> = planes.iter().map(Cow::as_ref).collect();
    match doc.composite_compression {
        Compression::Rle => out.extend_from_slice(&encode_scanlines(
            &planes,
            row_bytes(width, depth),
            height,
            psb,
        )?),
        Compression::Raw => {
            for plane in &planes {
                out.extend_from_slice(plane);
            }
        }
        Compression::Zip => {
            out.extend_from_slice(&zip_scanlines(&planes, width, height, false, depth)?)
        }
        Compression::ZipPrediction => {
            out.extend_from_slice(&zip_scanlines(&planes, width, height, true, depth)?)
        }
    }
    if let (Some(cache), Some(key)) = (cache, composite_key.as_ref()) {
        cache.put_composite(key.clone(), Arc::from(&out[block_start..]));
        cache.prune(&used_layer_keys, Some(key));
    } else if let Some(cache) = cache {
        cache.prune(&used_layer_keys, None);
    }
    Ok(out)
}
