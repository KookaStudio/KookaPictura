//! What a save holds, for the memory gate (#1320).
//!
//! A save shares its copy of the document's tiles, composites the merged image
//! a band at a time into tiles the evictor may pack or spill, and streams its
//! file from one band of rows at a time. So it holds the tile tables, one
//! compositor band and the encoder's rows, plus the tiled composite as
//! spillable output; never the document twice. The formats that still build
//! the whole image add those buffers and their encoded file.

use pictura_core::memory::{self, Reservation};
use pictura_core::{BitDepth, ColorMode, Document};

use super::memory_gate::{self as gate, CommitCost};

/// The merged image a save composites a band at a time: one band's
/// compositor canvas and planes (#1216).
pub(in crate::cxxqt_object) const SAVE_BAND_BYTES: u64 = 160 * 1024 * 1024;

/// The encoder's rows: a band read and its encoded rows (8 MiB each, as
/// `pictura_codec`'s band encoders cap them), the deflater and the file
/// buffer.
const ENCODE_BYTES: u64 = 2 * (8 << 20) + (1 << 20);

fn canvas(doc: &Document) -> u64 {
    u64::from(doc.width) * u64::from(doc.height)
}

/// Bytes per sample of the output the retained store writes.
fn sample_bytes(doc: &Document) -> u64 {
    match doc.source.depth() {
        Some(BitDepth::Sixteen) => 2,
        Some(BitDepth::ThirtyTwo) => 4,
        _ => 1,
    }
}

/// The whole planes the PSD writer still builds (`ponytail:` in the codec):
/// a 16/32-bit composite refresh and native widening, and the composite of a
/// CMYK, Lab, Indexed or other source mode, one canvas of four planes each.
fn whole_plane_bytes(doc: &Document) -> u64 {
    let deep = sample_bytes(doc) > 1;
    let separated = doc.source.mode().is_some_and(|mode| mode != ColorMode::Rgb);
    if deep || separated {
        3 * 4 * canvas(doc) * sample_bytes(doc)
    } else {
        0
    }
}

/// A native PSD/PSB save: the job's copy (its tile tables), one compositor
/// band, the encoder's rows and RLE count tables, and any whole planes the
/// mode needs; the merged image's tiles are spillable.
pub(in crate::cxxqt_object) fn save_cost(doc: &Document) -> CommitCost {
    let counts = 4 * u64::from(doc.height) * (4 + doc.channels.len() as u64);
    let merged = if doc.composite_is_authoritative() {
        0
    } else {
        4 * canvas(doc)
    };
    CommitCost {
        working: crate::history::tile_table_bytes(doc)
            + SAVE_BAND_BYTES
            + ENCODE_BYTES
            + counts
            + whole_plane_bytes(doc),
        spillable: merged,
    }
}

/// An engine Save As of `kind`: the streamed formats hold the band and the
/// encoder's rows; the whole-image formats (`ponytail:` in the codec) also
/// hold the flat raster planes and their encoded file. The tiled composite is
/// spillable for both.
pub(in crate::cxxqt_object) fn raster_save_cost(
    doc: &Document,
    kind: pictura_codec::SaveKind,
    options: &pictura_codec::SaveOptions,
) -> CommitCost {
    let n = canvas(doc);
    let base = crate::history::tile_table_bytes(doc) + SAVE_BAND_BYTES + ENCODE_BYTES;
    let working = if pictura_codec::save_streams(kind, doc, options) {
        // TIFF JPEG hands the encoder one interleaved strip and gets the file.
        let jpeg = if kind == pictura_codec::SaveKind::Tiff && options.tiff_compression == 4 {
            8 * n
        } else {
            0
        };
        base + jpeg
    } else {
        let quantizes = kind == pictura_codec::SaveKind::Gif
            || (kind == pictura_codec::SaveKind::Bmp && options.bmp_depth <= 8);
        base + whole_raster_bytes(doc)
            + encoded_bound(kind, n)
            + if quantizes { QUANTIZE_BYTES * n } else { 0 }
    };
    CommitCost {
        working,
        spillable: 4 * n,
    }
}

/// The quantizer's working set and indices per pixel, fitted to the heap peak
/// of the GIF save (30 bytes a pixel in all at 4250², 8 of them the flat
/// composite and its RGBA).
const QUANTIZE_BYTES: u64 = 22;

/// The flat planes the codec's whole-image `Raster` builds: the composite
/// copy, RGBA, RGB and gray, and the CMYK or Lab separation.
fn whole_raster_bytes(doc: &Document) -> u64 {
    let n = canvas(doc);
    let separation = match doc.source.mode().unwrap_or(doc.mode) {
        ColorMode::Cmyk => 8 * n,
        ColorMode::Lab => 6 * n,
        _ => 0,
    };
    4 * n + 4 * n + 3 * n + n + separation
}

/// An upper bound of a whole-image format's encoded file: EPS and DCS write
/// hex or ASCII85 image data and a preview, the others at most the raw
/// samples and their compressor's growth.
fn encoded_bound(kind: pictura_codec::SaveKind, n: u64) -> u64 {
    match kind {
        pictura_codec::SaveKind::Eps | pictura_codec::SaveKind::Dcs => 12 * n,
        _ => 6 * n,
    }
}

/// A Qt JPEG or WebP save: the flat composite, its sRGB copy, the packed
/// RGBA and Qt's image of it, and the encoded file.
pub(in crate::cxxqt_object) fn qt_raster_cost(doc: &Document) -> CommitCost {
    CommitCost {
        working: crate::history::tile_table_bytes(doc) + 20 * canvas(doc),
        spillable: 0,
    }
}

/// A GIF save: the flat composite, its sRGB copy, the packed RGBA, the
/// quantizer's indices and working set, and the encoded file.
pub(in crate::cxxqt_object) fn gif_cost(doc: &Document) -> CommitCost {
    CommitCost {
        working: crate::history::tile_table_bytes(doc) + (10 + QUANTIZE_BYTES) * canvas(doc),
        spillable: 0,
    }
}

/// Reserve a save's working bytes for as long as the reservation lives, and
/// admit its spillable bytes against the room plus the scratch disks' free
/// space; `None` records a memory refusal.
pub(in crate::cxxqt_object) fn reserve_save(cost: &CommitCost) -> Option<Reservation<'static>> {
    let reservation = memory::governor().reserve(cost.working, "a save").ok()?;
    if let Some(room) = memory::governor().room() {
        if cost.spillable > room.saturating_add(gate::scratch_available()) {
            gate::refuse_scratch_full("a save", cost.spillable);
            return None;
        }
    }
    Some(reservation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::helpers::memory_gate::{
        clear, edit_refused_for_memory, set_cgroup_for_test, set_scratch_available_for_test,
    };
    use pictura_core::{Channel, LayerStack, Plane};

    const GIB: u64 = 1 << 30;

    /// M24's document: four full-canvas RGB layers and a Background at
    /// `side`², every plane tiled (uniform, so the test allocates nothing).
    fn large_layered(side: u32) -> Document {
        let s = side as usize;
        let layer = |name: &str| {
            let channel = |id| Channel {
                id,
                data: Plane::filled(s, s, 1, 128u8),
            };
            let mut layer = crate::cxxqt_object::tests::pixel_layer(name, 1, 1, (0, 0, 0));
            layer.rect.right = side as i32;
            layer.rect.bottom = side as i32;
            *layer.channels_mut() = vec![channel(0), channel(1), channel(2), channel(-1)];
            layer
        };
        let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(
            ["Background", "1", "2", "3", "4"]
                .into_iter()
                .map(layer)
                .collect(),
        );
        doc
    }

    // [smg_estimate]
    #[test]
    fn save_cost_does_not_scale_with_the_document() {
        let doc = large_layered(17000);
        let cost = save_cost(&doc);
        assert!(cost.working < 512 << 20, "working {} B", cost.working);
        // The previous estimate: twice the document's samples, the merged
        // image and a band.
        let samples = 5 * 4 * canvas(&doc);
        let previous = 2 * samples + samples.min(4 * canvas(&doc)) + SAVE_BAND_BYTES;
        assert!(previous > 10 * GIB, "previous {previous} B");
    }

    // [smg_admitted]
    #[test]
    fn a_two_gib_host_admits_a_large_save() {
        let doc = large_layered(17000);
        set_cgroup_for_test(2 * GIB as i64, (3 * GIB / 2) as i64);
        set_scratch_available_for_test(4 * GIB as i64);
        clear();
        let admitted = reserve_save(&save_cost(&doc)).is_some();
        set_cgroup_for_test(-1, 0);
        set_scratch_available_for_test(-1);
        assert!(admitted, "a 2 GiB host admits the save");
    }

    // [smg_refused]
    #[test]
    fn a_small_host_refuses_the_save() {
        let doc = large_layered(17000);
        let working = save_cost(&doc).working;
        // A 1 GiB cgroup leaves 768 MiB under its ceiling; use all but less
        // than the working bytes.
        set_cgroup_for_test(GIB as i64, (768 << 20) - (working / 2) as i64);
        set_scratch_available_for_test(4 * GIB as i64);
        clear();
        let refused = reserve_save(&save_cost(&doc)).is_none();
        let recorded = edit_refused_for_memory();
        set_cgroup_for_test(-1, 0);
        set_scratch_available_for_test(-1);
        assert!(refused && recorded, "refused as a memory refusal");
    }

    // [smg_whole_refused]
    #[test]
    fn a_whole_image_format_is_refused_first() {
        let doc = large_layered(4096);
        let options = pictura_codec::SaveOptions::default();
        let tiff = raster_save_cost(&doc, pictura_codec::SaveKind::Tiff, &options);
        let gif = raster_save_cost(&doc, pictura_codec::SaveKind::Gif, &options);
        assert!(tiff.working < gif.working);
        // Room halfway between the two working sets.
        let room = (tiff.working + gif.working) / 2;
        let limit = 4 * GIB;
        let ceiling = pictura_core::memory::less_headroom(limit);
        set_cgroup_for_test(limit as i64, (ceiling - room) as i64);
        set_scratch_available_for_test(64 * GIB as i64);
        clear();
        let tiff_admitted = reserve_save(&tiff).is_some();
        clear();
        let gif_admitted = reserve_save(&gif).is_some();
        let recorded = edit_refused_for_memory();
        set_cgroup_for_test(-1, 0);
        set_scratch_available_for_test(-1);
        assert!(tiff_admitted, "the streamed TIFF fits");
        assert!(!gif_admitted && recorded, "GIF is refused as memory");
    }
}
