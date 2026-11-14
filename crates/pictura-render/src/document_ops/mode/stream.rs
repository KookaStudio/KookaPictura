//! The RGB source of a conversion, streamed (#1274): a layered RGB document
//! keeps no stored composite (#1216), so a conversion composites it one band
//! at a time straight into the converted document's stores instead of first
//! storing a whole RGBA composite and copying it into flat planes. Each pass
//! writes what the whole-plane path writes, byte for byte.

use pictura_codec::{ConversionOptions, Profile};
use pictura_core::{
    BitDepth, ColorMode, Document, PixelBuffer, Plane, PsdRect, Samples, SourcePlanes,
};

use super::planes::{encode_source_mode, luma_u8};

/// Rows per band. A multiple of 8, so a band's samples keep their plane
/// index modulo 8 and the ordered dither of a profile conversion is the
/// whole plane's.
const ROWS: u32 = pictura_core::TILE as u32;

/// Whether `doc` is RGB and keeps no stored composite, so a conversion
/// streams its source.
pub(super) fn streams_source(doc: &Document) -> bool {
    doc.mode == ColorMode::Rgb
        && !doc.layers.is_empty()
        && !crate::DocumentComposite::is_stored(doc)
}

/// Call `f(top, rgb, alpha)` for each band of the composite, top to bottom:
/// the band's three colour planes back to back and its alpha plane.
fn for_each_band(doc: &Document, mut f: impl FnMut(usize, &[u8], &[u8])) {
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    crate::stream_composite(doc, whole, ROWS, |offset, band: PixelBuffer| {
        let n = band.width as usize * band.height as usize;
        let mut rgba = vec![0u8; 4 * n];
        band.data.read_into(0, &mut rgba);
        f(offset as usize, &rgba[..3 * n], &rgba[3 * n..]);
    });
}

/// The composite as one gray plane through [`luma_u8`], as
/// `planes::reduce_to_gray` derives it from a stored composite.
pub(super) fn gray_composite(doc: &Document) -> PixelBuffer {
    let (w, h) = (doc.width as usize, doc.height as usize);
    let mut gray = Plane::filled(w, h, 1, 0u8);
    for_each_band(doc, |top, rgb, _| {
        let n = rgb.len() / 3;
        let rows: Vec<u8> = (0..n)
            .map(|i| luma_u8(rgb[i], rgb[n + i], rgb[2 * n + i]))
            .collect();
        gray.write_rect_from(0..w, top..top + n / w.max(1), w, &rows);
        gray.settle();
    });
    PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 1,
        data: gray,
    }
}

/// The composite encoded in `mode` (CMYK or Lab): the 8-bit source store and
/// the RGBA composite it displays as, as `planes::encode_document` builds them
/// from a stored composite.
pub(super) fn encoded_composite(
    doc: &Document,
    mode: ColorMode,
    cmyk_profile: Option<&Profile>,
    options: ConversionOptions,
) -> (PixelBuffer, SourcePlanes) {
    let (w, h) = (doc.width as usize, doc.height as usize);
    let plane = w * h;
    let mut shown = Plane::filled(w, h, 4, 0u8);
    let mut store: Vec<u8> = Vec::new();
    for_each_band(doc, |top, rgb, alpha| {
        let n = alpha.len();
        let rows = top..top + n / w.max(1);
        let (encoded, display) = encode_source_mode(mode, rgb, cmyk_profile, options);
        let planes = encoded.len() / n.max(1);
        if store.is_empty() {
            store = vec![0; planes * plane];
        }
        for c in 0..planes {
            let at = c * plane + top * w;
            store[at..at + n].copy_from_slice(&encoded[c * n..(c + 1) * n]);
        }
        for c in 0..3 {
            let ys = c * h + rows.start..c * h + rows.end;
            shown.write_rect_from(0..w, ys, w, &display[c * n..(c + 1) * n]);
        }
        shown.write_rect_from(0..w, 3 * h + rows.start..3 * h + rows.end, w, alpha);
        shown.settle();
    });
    with_store(doc, shown, store)
}

/// The RGBA composite and its 8-bit RGB store, the step before widening, as
/// `materialize_composite` and `planes::ensure_stores` build them.
pub(super) fn composite_with_store(doc: &Document) -> (PixelBuffer, SourcePlanes) {
    let (w, h) = (doc.width as usize, doc.height as usize);
    let plane = w * h;
    let mut composite = Plane::filled(w, h, 4, 0u8);
    let mut store = vec![0u8; 3 * plane];
    for_each_band(doc, |top, rgb, alpha| {
        let n = alpha.len();
        let rows = top..top + n / w.max(1);
        for c in 0..3 {
            let at = c * plane + top * w;
            store[at..at + n].copy_from_slice(&rgb[c * n..(c + 1) * n]);
            let ys = c * h + rows.start..c * h + rows.end;
            composite.write_rect_from(0..w, ys, w, &rgb[c * n..(c + 1) * n]);
        }
        composite.write_rect_from(0..w, 3 * h + rows.start..3 * h + rows.end, w, alpha);
        composite.settle();
    });
    with_store(doc, composite, store)
}

/// A four-plane composite of `doc`'s size and its 8-bit source store.
fn with_store(doc: &Document, composite: Plane<u8>, store: Vec<u8>) -> (PixelBuffer, SourcePlanes) {
    (
        PixelBuffer {
            width: doc.width,
            height: doc.height,
            channels: 4,
            data: composite,
        },
        SourcePlanes {
            depth: BitDepth::Eight,
            width: doc.width,
            height: doc.height,
            samples: Samples::U8(store.into()),
        },
    )
}
