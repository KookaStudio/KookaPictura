use pictura_core::*;
use std::borrow::Cow;
use std::io::{Seek, Write};
use std::sync::Arc;

use crate::common::*;
use crate::depth::{depth_bits, depth_of, output_depth, row_bytes};
use crate::error::PsdError;
use crate::write_cache::{ChannelKey, CompositeKey, EncodeCache};
use crate::write_indexed::writes_indexed;
use crate::write_layers::write_layer_info;
use crate::write_stream::{self, Sink};

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
pub(crate) fn encode_packbits_row(row: &[u8], out: &mut Vec<u8>) {
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

/// A plane the encoders read one scanline at a time.
pub(crate) trait ScanRows {
    /// The bytes the plane holds.
    fn byte_len(&self) -> usize;
    /// Row `row`, `row_bytes` long: borrowed, or copied into `scratch`.
    fn scan_row<'s>(&'s self, row: usize, row_bytes: usize, scratch: &'s mut Vec<u8>) -> &'s [u8];
    /// Rows `y0 .. y0 + rows` into `out` (`rows * row_bytes` long).
    fn read_rows(&self, y0: usize, rows: usize, row_bytes: usize, out: &mut [u8]) {
        let mut scratch = Vec::new();
        for (i, dst) in out
            .chunks_exact_mut(row_bytes.max(1))
            .take(rows)
            .enumerate()
        {
            dst.copy_from_slice(self.scan_row(y0 + i, row_bytes, &mut scratch));
        }
    }
}

impl<T: AsRef<[u8]> + ?Sized> ScanRows for &T {
    fn byte_len(&self) -> usize {
        (*self).as_ref().len()
    }

    fn scan_row<'s>(&'s self, row: usize, row_bytes: usize, _: &'s mut Vec<u8>) -> &'s [u8] {
        &(*self).as_ref()[row * row_bytes..(row + 1) * row_bytes]
    }
}

/// One plane to encode: native-depth bytes, or rows of an 8-bit plane read in
/// place, so a tiled plane is never materialized to be written (#1213).
pub(crate) enum PlaneBytes<'a> {
    Bytes(Cow<'a, [u8]>),
    /// Rows `first_row .. first_row + height` of `plane`, `width` wide.
    Rows {
        plane: &'a Plane<u8>,
        first_row: usize,
        width: usize,
        height: usize,
    },
}

impl<'a> From<Cow<'a, [u8]>> for PlaneBytes<'a> {
    fn from(bytes: Cow<'a, [u8]>) -> Self {
        PlaneBytes::Bytes(bytes)
    }
}

impl ScanRows for PlaneBytes<'_> {
    fn byte_len(&self) -> usize {
        match self {
            PlaneBytes::Bytes(bytes) => bytes.len(),
            PlaneBytes::Rows {
                plane,
                first_row,
                width,
                height,
            } => {
                let whole = (first_row + height) * width <= plane.len();
                if whole {
                    width * height
                } else {
                    0
                }
            }
        }
    }

    fn scan_row<'s>(&'s self, row: usize, row_bytes: usize, scratch: &'s mut Vec<u8>) -> &'s [u8] {
        match self {
            PlaneBytes::Bytes(bytes) => &bytes[row * row_bytes..(row + 1) * row_bytes],
            PlaneBytes::Rows {
                plane,
                first_row,
                width,
                ..
            } => {
                scratch.resize(row_bytes, 0);
                let y = first_row + row;
                plane.read_rect_into(0..row_bytes, y..y + 1, *width, scratch);
                scratch
            }
        }
    }

    fn read_rows(&self, y0: usize, rows: usize, row_bytes: usize, out: &mut [u8]) {
        match self {
            PlaneBytes::Bytes(bytes) => {
                out.copy_from_slice(&bytes[y0 * row_bytes..(y0 + rows) * row_bytes]);
            }
            PlaneBytes::Rows {
                plane,
                first_row,
                width,
                ..
            } => {
                let y = first_row + y0;
                plane.read_rect_into(0..row_bytes, y..y + rows, *width, out);
            }
        }
    }
}

/// [`native_plane`] for rows `first_row ..` of an 8-bit `plane`: at depth 8 the
/// rows are read in place when encoding; deeper output widens a copy.
///
/// ponytail: a 16/32-bit output copies the 8-bit plane once to compare it with
/// the retained native samples; the native stores are flat anyway.
pub(crate) fn native_plane_rows<'a>(
    retained: Option<&Samples>,
    plane: &'a Plane<u8>,
    first_row: usize,
    width: usize,
    height: usize,
    depth: u16,
) -> Result<PlaneBytes<'a>, PsdError> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel plane size overflow".into()))?;
    // A whole number of channels: the composite stacks several, a layer
    // channel holds exactly one.
    let whole = pixels > 0 && plane.len().is_multiple_of(pixels);
    if (first_row + height) * width > plane.len() || !(whole || plane.len() == pixels) {
        return Err(PsdError::Invalid("channel plane length mismatch".into()));
    }
    if depth == 8 {
        return Ok(PlaneBytes::Rows {
            plane,
            first_row,
            width,
            height,
        });
    }
    let mut current = vec![0u8; pixels];
    plane.read_into(first_row * width, &mut current);
    let native = native_plane(retained, &current, width, height, depth)?;
    Ok(PlaneBytes::Bytes(Cow::Owned(native.into_owned())))
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
    let store = doc.source.planes().as_ref()?;
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
#[cfg(test)]
pub(crate) fn encode_scanlines<P: ScanRows + Sync>(
    planes: &[P],
    row_bytes: usize,
    height: usize,
    psb: bool,
) -> Result<Vec<u8>, PsdError> {
    write_stream::to_vec(|sink| {
        write_stream::write_payload(
            sink,
            Compression::Rle,
            planes,
            row_bytes,
            row_bytes,
            height,
            8,
            psb,
        )
    })
}

/// Concatenate the native-depth `planes` (`width` pixels by `height` rows each,
/// plane-major) and zlib-wrap the result. When `predict`, apply the forward
/// depth-specific per-row delta first (the inverse of the reader's
/// `undo_prediction`); no scanline count table.
pub(crate) fn zip_scanlines<P: ScanRows + Sync>(
    planes: &[P],
    width: usize,
    height: usize,
    predict: bool,
    depth: u16,
) -> Result<Vec<u8>, PsdError> {
    let kind = if predict {
        Compression::ZipPrediction
    } else {
        Compression::Zip
    };
    write_stream::to_vec(|sink| {
        write_stream::write_payload(
            sink,
            kind,
            planes,
            width,
            row_bytes(width, depth),
            height,
            depth,
            false,
        )
    })
}

/// Write the complete on-disk layer-channel stream for one engine-encoded
/// plane: the compression word followed by the payload for `kind` at `depth`.
/// `plane` is the native-depth plane (`row_bytes(width, depth) * height`
/// bytes).
pub(crate) fn write_channel_stream(
    sink: &mut write_stream::Sink<'_>,
    kind: Compression,
    width: usize,
    height: usize,
    plane: &PlaneBytes<'_>,
    depth: u16,
    psb: bool,
) -> Result<(), PsdError> {
    sink.put(&kind.to_code().to_be_bytes())?;
    write_stream::write_payload(
        sink,
        kind,
        std::slice::from_ref(plane),
        width,
        row_bytes(width, depth),
        height,
        depth,
        psb,
    )
}

/// Serialize a [`Document`] into a valid PSD, or into a version-2 PSB when the
/// source document was a PSB or a dimension exceeds the PSD limit.
pub fn write_psd(doc: &Document) -> Result<Vec<u8>, PsdError> {
    write_container(doc, writes_psb(doc), None, false, None).map(|result| result.bytes)
}

/// Serialize a [`Document`] into a version-2 PSB regardless of its dimensions.
pub fn write_psb(doc: &Document) -> Result<Vec<u8>, PsdError> {
    write_container(doc, true, None, false, None).map(|result| result.bytes)
}

/// As [`write_psd`], reusing encoded layer channels and the composite block from
/// `cache`. The bytes equal an uncached encode.
pub fn write_psd_cached(doc: &Document, cache: &EncodeCache) -> Result<Vec<u8>, PsdError> {
    write_container(doc, writes_psb(doc), Some(cache), false, None).map(|result| result.bytes)
}

/// As [`write_psb`], reusing encoded layer channels and the composite block from
/// `cache`. The bytes equal an uncached encode.
pub fn write_psb_cached(doc: &Document, cache: &EncodeCache) -> Result<Vec<u8>, PsdError> {
    write_container(doc, true, Some(cache), false, None).map(|result| result.bytes)
}

/// Write an interactive native save with Photoshop's compatibility marker and
/// placeholder when the real merged image is disabled.
pub fn write_native_compatibility_cached(
    doc: &Document,
    cache: &EncodeCache,
    force_psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let psb = force_psb || writes_psb(doc);
    write_container(doc, psb, Some(cache), true, None).map(|result| result.bytes)
}

pub struct VersionedWrite {
    pub bytes: Vec<u8>,
    pub skipped_xmp_mirror: bool,
}

/// Serialize an application save with version metadata. For interactive saves,
/// `compatibility_placeholder` writes a placeholder when the real merge is omitted.
pub fn write_versioned(
    doc: &Document,
    force_psb: bool,
    cache: Option<&EncodeCache>,
    writer_version: &str,
    compatibility_placeholder: bool,
) -> Result<VersionedWrite, PsdError> {
    let psb = force_psb || writes_psb(doc);
    write_container(
        doc,
        psb,
        cache,
        compatibility_placeholder,
        Some(writer_version),
    )
}

/// As [`write_versioned`], streamed into `out` at its current position as
/// [`write_psd_to`] streams; returns whether the XMP mirror was skipped.
pub fn write_versioned_to<W: Write + Seek>(
    out: &mut W,
    doc: &Document,
    force_psb: bool,
    cache: Option<&EncodeCache>,
    writer_version: &str,
    compatibility_placeholder: bool,
) -> Result<bool, PsdError> {
    let psb = force_psb || writes_psb(doc);
    let mut sink = Sink::new(out)?;
    write_container_to(
        &mut sink,
        doc,
        psb,
        cache,
        compatibility_placeholder,
        Some(writer_version),
    )
}

/// Distinguishes a native encoder refusal from failure of its output stream.
#[derive(Debug, thiserror::Error)]
pub enum NativeWriteError {
    #[error(transparent)]
    Codec(PsdError),
    #[error("write failed: {0}")]
    Io(#[source] std::io::Error),
}

/// Stream a versioned native save while preserving the underlying I/O failure.
pub fn write_versioned_to_checked<W: Write + Seek>(
    out: &mut W,
    doc: &Document,
    force_psb: bool,
    cache: Option<&EncodeCache>,
    writer_version: &str,
    compatibility_placeholder: bool,
) -> Result<bool, NativeWriteError> {
    let psb = force_psb || writes_psb(doc);
    let mut sink = Sink::new_with_io(out).map_err(NativeWriteError::Io)?;
    write_container_to(
        &mut sink,
        doc,
        psb,
        cache,
        compatibility_placeholder,
        Some(writer_version),
    )
    .map_err(|error| {
        sink.take_io_failure()
            .map_or(NativeWriteError::Codec(error), NativeWriteError::Io)
    })
}

/// The output depth and the synthesized layer modes `doc` writes.
struct LayerModes {
    depth: u16,
    lab: bool,
    cmyk: bool,
    indexed: bool,
}

fn layer_modes(doc: &Document) -> Result<LayerModes, PsdError> {
    let plane = doc.width as usize * doc.height as usize;
    let bitmap_mode = crate::write_bitmap::writes_bitmap(doc);
    let depth = if bitmap_mode { 1 } else { output_depth(doc)? };
    Ok(LayerModes {
        depth,
        lab: doc.source.mode() == Some(ColorMode::Lab)
            && doc.composite.channels == 3
            && (depth != 8 || doc.source.depth().is_none()),
        cmyk: doc.source.mode() == Some(ColorMode::Cmyk)
            && doc.composite.channels == 3
            && (depth != 8 || doc.source.depth().is_none()),
        indexed: writes_indexed(doc, depth, plane),
    })
}

/// The document-level tagged blocks after the layer section, or `None` when
/// the document has nothing to write in the layer-and-mask section.
fn layer_section_extra(doc: &Document, psb: bool, merged_alpha: Option<u16>) -> Option<Vec<u8>> {
    let extra =
        crate::type_path::encode_text_paths(&doc.layers, &doc.layer_section_extra, doc.is_psb);
    let mut extra = reframe_document_extra(&extra, doc.is_psb, psb);
    let authoring = crate::smart_writer::collect_authoring(&doc.layers);
    if !authoring.is_empty() {
        extra.extend_from_slice(&crate::smart_writer::author_lnk2_bytes(&authoring, psb));
    }
    let mut extra = crate::filter_effects::author_filter_masks(
        &extra,
        &doc.layers,
        u32::from(doc.composite.channels),
        psb,
    );
    // A flat merged image with transparency carries the depth's marker.
    if let Some(depth) = merged_alpha {
        if doc.layers.is_empty()
            && !crate::compatibility::has_merged_transparency_marker(&extra, psb)
        {
            let key = match depth {
                16 => b"Mt16",
                32 => b"Mt32",
                _ => b"Mtrn",
            };
            write_tag_document(&mut extra, key, &[], psb);
        }
    }
    (!(doc.layers.is_empty() && doc.global_layer_mask.is_empty() && extra.is_empty()))
        .then_some(extra)
}

/// A document-level tagged block whose data `body` streams into the sink: the
/// length is patched afterwards and the block padded to 4 bytes, as
/// [`write_tag_document`] frames it.
fn stream_tag_document<T>(
    sink: &mut Sink<'_>,
    key: &[u8; 4],
    psb: bool,
    body: impl FnOnce(&mut Sink<'_>) -> Result<T, PsdError>,
) -> Result<T, PsdError> {
    sink.put(b"8BIM")?;
    sink.put(key)?;
    let wide = psb && is_psb_big_key(key);
    let len_at = sink.len_placeholder(wide)?;
    let start = sink.pos();
    let out = body(sink)?;
    let len = sink.pos() - start;
    sink.patch_len(len_at, len, wide)?;
    sink.zeros(((4 - len % 4) % 4) as usize)?;
    Ok(out)
}

/// Write the layer-and-mask section, its length first; a zero length when the
/// document has nothing to write there. Returns the cache keys it used.
fn write_layer_mask_section(
    sink: &mut Sink<'_>,
    doc: &Document,
    psb: bool,
    cache: Option<&EncodeCache>,
    merged_alpha: bool,
) -> Result<Vec<ChannelKey>, PsdError> {
    let section_at = sink.len_placeholder(psb)?;
    let modes = layer_modes(doc)?;
    let Some(extra) = layer_section_extra(doc, psb, merged_alpha.then_some(modes.depth)) else {
        return Ok(Vec::new());
    };
    let start = sink.pos();
    let tagged = tagged_layer_key(modes.depth).filter(|_| !doc.layers.is_empty());
    let keys = if let Some(key) = tagged {
        // The 16/32-bit layout leaves the classic layer info empty and carries
        // it in a document-level tagged block after the global mask.
        sink.zeros(if psb { 8 } else { 4 })?;
        put_global_mask(sink, doc)?;
        stream_tag_document(sink, key, psb, |sink| {
            write_layer_info(
                sink,
                doc,
                psb,
                modes.lab,
                modes.cmyk,
                modes.indexed,
                cache,
                merged_alpha,
            )
        })?
    } else {
        let info_at = sink.len_placeholder(psb)?;
        let info_start = sink.pos();
        // A flat merged image with transparency leaves the layer info empty.
        let keys = if merged_alpha && doc.layers.is_empty() {
            Vec::new()
        } else {
            write_layer_info(
                sink,
                doc,
                psb,
                modes.lab,
                modes.cmyk,
                modes.indexed,
                cache,
                merged_alpha,
            )?
        };
        sink.patch_len(info_at, sink.pos() - info_start, psb)?;
        put_global_mask(sink, doc)?;
        keys
    };
    sink.put(&extra)?;
    sink.patch_len(section_at, sink.pos() - start, psb)?;
    Ok(keys)
}

fn put_global_mask(sink: &mut Sink<'_>, doc: &Document) -> Result<(), PsdError> {
    sink.put(&(doc.global_layer_mask.len() as u32).to_be_bytes())?;
    sink.put(&doc.global_layer_mask)
}

/// Write the Photoshop ImageSourceData (tag 37724) payload for a layered TIFF;
/// `false` when the document has nothing to write there. The PSD layer section
/// is authored big-endian, so callers must write the TIFF container big-endian:
/// the magic, block signatures, and lengths then share one byte order.
/// Photoshop documents are big-endian too, so the layer data is reused without
/// a conversion pass.
pub(crate) fn write_layer_mask_tiff_tag(
    sink: &mut Sink<'_>,
    doc: &Document,
    cache: Option<&EncodeCache>,
) -> Result<bool, PsdError> {
    let Some(extra) = layer_section_extra(doc, false, None) else {
        return Ok(false);
    };
    let modes = layer_modes(doc)?;
    sink.put(b"Adobe Photoshop Document Data Block\0")?;
    let tagged = tagged_layer_key(modes.depth).filter(|_| !doc.layers.is_empty());
    let key = tagged.unwrap_or(b"Layr");
    stream_tag_document(sink, key, false, |sink| {
        write_layer_info(
            sink,
            doc,
            false,
            modes.lab,
            modes.cmyk,
            modes.indexed,
            cache,
            false,
        )
    })?;
    if !doc.global_layer_mask.is_empty() {
        write_tag_document_to(sink, b"LMsk", &doc.global_layer_mask, false)?;
    }
    sink.put(&extra)?;
    Ok(true)
}

fn write_tag_document_to(
    sink: &mut Sink<'_>,
    key: &[u8; 4],
    data: &[u8],
    psb: bool,
) -> Result<(), PsdError> {
    let mut out = Vec::with_capacity(data.len() + 16);
    write_tag_document(&mut out, key, data, psb);
    sink.put(&out)
}

fn write_container(
    doc: &Document,
    psb: bool,
    cache: Option<&EncodeCache>,
    compatibility_placeholder: bool,
    writer_version: Option<&str>,
) -> Result<VersionedWrite, PsdError> {
    let mut skipped_xmp_mirror = false;
    let bytes = write_stream::to_vec(|sink| {
        skipped_xmp_mirror = write_container_to(
            sink,
            doc,
            psb,
            cache,
            compatibility_placeholder,
            writer_version,
        )?;
        Ok(())
    })?;
    Ok(VersionedWrite {
        bytes,
        skipped_xmp_mirror,
    })
}

/// Write `doc` as a PSD (or a version-2 PSB when `psb`) into `out` at its
/// current position, streaming the layer channels and the composite one band
/// of rows at a time (#1320). The bytes equal [`write_psd`]'s for the same
/// document; on an error `out` holds a partial file the caller discards.
pub fn write_psd_to<W: Write + Seek>(
    out: &mut W,
    doc: &Document,
    psb: bool,
    cache: Option<&EncodeCache>,
) -> Result<(), PsdError> {
    let mut sink = Sink::new(out)?;
    write_container_to(&mut sink, doc, psb, cache, false, None).map(|_| ())
}

/// Whether [`write_psd`] writes `doc` as a version-2 PSB.
pub fn writes_psb(doc: &Document) -> bool {
    doc.is_psb || doc.width > MAX_DIM_PSD || doc.height > MAX_DIM_PSD
}

/// Returns whether the versioned write skipped the XMP mirror.
fn write_container_to(
    sink: &mut Sink<'_>,
    doc: &Document,
    psb: bool,
    cache: Option<&EncodeCache>,
    compatibility_placeholder: bool,
    writer_version: Option<&str>,
) -> Result<bool, PsdError> {
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
    let lab_mode = doc.source.mode() == Some(ColorMode::Lab)
        && doc.composite.channels == 3
        && (depth != 8 || doc.source.depth().is_none());
    // A CMYK source writes header mode CMYK with four color channels: an 8-bit
    // read re-encodes the working RGB, and a 16/32-bit read re-emits the
    // retained native planes.
    let cmyk_mode = doc.source.mode() == Some(ColorMode::Cmyk)
        && doc.composite.channels == 3
        && (depth != 8 || doc.source.depth().is_none());
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

    let mut header = Vec::with_capacity(26);
    header.extend_from_slice(&SIGNATURE.to_be_bytes());
    header.extend_from_slice(&(if psb { VERSION_PSB } else { VERSION_PSD }).to_be_bytes());
    header.extend_from_slice(&[0u8; 6]); // reserved
    header.extend_from_slice(&(channels as u16).to_be_bytes());
    header.extend_from_slice(&doc.height.to_be_bytes());
    header.extend_from_slice(&doc.width.to_be_bytes());
    header.extend_from_slice(&depth.to_be_bytes()); // depth
    header.extend_from_slice(&mode_code.to_be_bytes());
    sink.put(&header)?;
    // An Indexed output re-emits the retained palette (the read consumed
    // `color_mode_data`); every other mode replays the preserved section.
    let color_mode_data: &[u8] = match (indexed_mode, doc.source.palette_ref()) {
        (true, Some(palette)) => palette,
        _ => &doc.color_mode_data,
    };
    sink.put(&(color_mode_data.len() as u32).to_be_bytes())?;
    sink.put(color_mode_data)?;
    let resources = crate::layer_comps::resources_for_output(
        doc,
        crate::global_light::with_document_global_light(
            doc,
            crate::guide_resources::output_resources(doc, mode_code),
        ),
    );
    let merged = doc.composite.merged_output();
    let resources = crate::measurement_scale_resource::resources_for_output(doc, resources);
    let (resources, skipped_xmp_mirror) = if let Some(version) = writer_version {
        crate::document_version::resources_for_versioned_write(&resources, merged, version)?
    } else if (compatibility_placeholder && !merged)
        || (merged && crate::compatibility::has_real_merged_data(&resources) == Some(false))
    {
        (
            crate::compatibility::set_real_merged_data(&resources, merged),
            false,
        )
    } else {
        (resources, false)
    };
    let resources = crate::variable_resources::output(doc, resources)?;
    sink.put(&(resources.len() as u32).to_be_bytes())?;
    sink.put(&resources)?;

    if let Some(cache) = cache {
        cache.fit_budget();
    }
    // The layer-and-mask section body is shared with the layered-TIFF writer.
    let signed_alpha = doc.composite.signed_merged_alpha();
    let merged_alpha = (writer_version.is_some()
        || signed_alpha
        || compatibility_placeholder && !doc.layers.is_empty())
        && merged
        && mode_code == MODE_RGB
        && doc.source.mode().is_none()
        && doc.composite.channels == 4
        && (doc.channels.is_empty() || signed_alpha);
    let used_layer_keys = write_layer_mask_section(sink, doc, psb, cache, merged_alpha)?;

    // Direct writers retain the layer-only representation. Interactive saves
    // can emit the Photoshop placeholder without composing the layer stack.
    if !merged {
        if let Some(cache) = cache {
            cache.prune(&used_layer_keys, None);
        }
        if compatibility_placeholder {
            if doc.layers.is_empty() || !doc.channels.is_empty() {
                return Err(PsdError::Invalid(
                    "placeholder would discard image data".into(),
                ));
            }
            crate::compatibility::put_placeholder(
                sink,
                doc.width as usize,
                doc.height as usize,
                out_color_channels,
                depth,
                psb,
            )?;
        }
        return Ok(skipped_xmp_mirror);
    }

    let width = doc.width as usize;
    let height = doc.height as usize;
    let cacheable_composite = cache.is_some()
        && doc.source.mode().is_none()
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
            sink.put(&bytes)?;
            cache.prune(&used_layer_keys, Some(key));
            return Ok(skipped_xmp_mirror);
        }
        sink.begin_capture(cache.capture_limit());
    }
    sink.put(&doc.composite_compression.to_code().to_be_bytes())?;
    // The Lab, CMYK, or Indexed output re-emits retained planes exactly when
    // they are unchanged, and otherwise re-encodes from the working RGB into a
    // temp buffer (Indexed never re-encodes: an edit already chose the RGB mode).
    // ponytail: those, Bitmap, flat Duotone/Multichannel and 16/32-bit planes are
    // built whole from the retained native stores, which are flat anyway; only
    // the 8-bit working planes are read a band at a time.
    let lab_composite = lab_mode.then(|| lab_composite_planes(doc, depth, plane));
    let cmyk_composite =
        cmyk_mode.then(|| crate::write_cmyk::cmyk_composite_planes(doc, depth, plane));
    let indexed_composite = indexed_mode.then(|| {
        composite_retained(doc, depth, 0)
            .expect("indexed_mode checked the composite plane")
            .to_bytes()
    });
    let flat_composite = flat_source.as_ref().map(|(_, _, planes)| planes);
    let planes: Vec<PlaneBytes> = if bitmap_mode {
        crate::write_bitmap::bitmap_planes(doc)?
            .into_iter()
            .map(PlaneBytes::from)
            .collect()
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
                // The working composite is read row by row in place.
                planes.push(native_plane_rows(
                    composite_retained(doc, depth, c).as_ref(),
                    &doc.composite.data,
                    c * height,
                    width,
                    height,
                    depth,
                )?);
                continue;
            };
            planes.push(
                native_plane(
                    composite_retained(doc, depth, c).as_ref(),
                    current,
                    width,
                    height,
                    depth,
                )?
                .into(),
            );
        }
        if crate::write_bitmap::is_bitmap_source(doc) {
            for expanded in crate::write_bitmap::fallback_extra_planes(doc)? {
                planes.push(PlaneBytes::Bytes(Cow::Owned(expanded)));
            }
        } else {
            for (i, channel) in doc.channels.iter().enumerate() {
                planes.push(native_plane_rows(
                    // A CMYK output stores four color planes before the extras,
                    // so the retained extras offset is the output color count.
                    composite_retained(doc, depth, out_color_channels + i).as_ref(),
                    &channel.data,
                    0,
                    width,
                    height,
                    depth,
                )?);
            }
        }
        planes
    };
    write_stream::write_payload(
        sink,
        doc.composite_compression,
        &planes,
        width,
        row_bytes(width, depth),
        height,
        depth,
        psb,
    )?;
    let captured = composite_key.as_ref().and_then(|_| sink.end_capture());
    if let (Some(cache), Some(key)) = (cache, composite_key.as_ref()) {
        if let Some(bytes) = captured {
            cache.put_composite(key.clone(), Arc::from(bytes));
        }
        cache.prune(&used_layer_keys, Some(key));
    } else if let Some(cache) = cache {
        cache.prune(&used_layer_keys, None);
    }
    Ok(skipped_xmp_mirror)
}
