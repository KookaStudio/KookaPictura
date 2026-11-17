//! Heap guards for the whole-picture readers (#1274): each reads the picture
//! of a 4250² layered document, which keeps no stored composite, and must not
//! build a canvas-sized composite. Its peak stays below its own output plus
//! half of one RGBA composite: a whole composite does not fit in that, while a
//! band does, with the compositor's f32 band canvas (16 bytes a pixel, about a
//! quarter composite at 4250² for one 256-row band) and the reader's per-band
//! copies. The ignored `*_full` twins print the peak at 17000².

use super::*;
use crate::cxxqt_object::picture::{
    coverage_bounds, picture_rgba, preview_picture, sample_tiles, tile_source,
};
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};

/// One RGBA composite at `side`².
fn composite(side: u32) -> usize {
    side as usize * side as usize * 4
}

/// Fail when `run` peaks at or above `output + composite(GUARD_SIDE) / 2`.
fn bounded<T>(
    name: &str,
    output: usize,
    setup: impl FnOnce() -> T + Send,
    run: impl FnOnce(&mut T) + Send,
) {
    if !alone_in_process(name) {
        return;
    }
    let (peak, _) = measure(setup, run);
    let limit = output + composite(GUARD_SIDE) / 2;
    println!(
        "{name} at {GUARD_SIDE}²: heap peak {:.1} MiB ({peak} B), limit {:.1} MiB",
        mib(peak),
        mib(limit)
    );
    assert!(
        peak < limit,
        "{name}: heap peak {peak} B reaches its output {output} B + half a composite"
    );
}

fn export(view: &mut PictureViewRust) {
    let view = &*view;
    let doc = view.session.doc.as_ref().expect("document");
    std::hint::black_box(picture_rgba(doc, view, true));
}

fn trim(view: &mut PictureViewRust) {
    let view = &*view;
    let doc = view.session.doc.as_ref().expect("document");
    std::hint::black_box(coverage_bounds(doc, view));
}

fn similar(view: &mut PictureViewRust) {
    let view = &*view;
    let doc = view.session.doc.as_ref().expect("document");
    let sel = view.session.selection.as_ref().expect("a selection");
    let fetch = sample_tiles(doc, view);
    let grown = pictura_select::similar_from(sel, tile_source(doc, &fetch), 8);
    std::hint::black_box(grown.expect("a selection"));
}

fn magnetic(view: &mut PictureViewRust) {
    assert!(view.begin_magnetic(10));
}

fn preview(view: &mut PictureViewRust) {
    let view = &*view;
    let doc = view.session.doc.as_ref().expect("document");
    std::hint::black_box(preview_picture(doc, view, 256).into_owned());
}

fn gray(doc: &mut Document) {
    pictura_render::convert_mode(doc, ColorMode::Grayscale).expect("converts");
}

/// A `side`² layered RGB document with two small layers and, as the app keeps
/// one, no stored composite: the conversion's layer planes cost nothing, so
/// its peak is the composite it reads.
fn small_layers(side: u32) -> Document {
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    let layer = |name: &str, left: i32, top: i32, rgb| {
        let mut layer = crate::cxxqt_object::tests::pixel_layer(name, 64, 64, rgb);
        layer.rect = PsdRect {
            top,
            left,
            bottom: top + 64,
            right: left + 64,
        };
        layer
    };
    doc.layers = LayerStack::from_layers(vec![
        layer("a", 100, 100, (200, 40, 10)),
        layer("b", side as i32 / 2, side as i32 * 2 / 3, (10, 40, 200)),
    ]);
    // A layered RGB document keeps no stored composite in the app (#1216).
    doc.composite
        .replace(pictura_core::PixelBuffer::new(0, 0, 4))
        .expect("valid composite");
    doc
}

#[test]
fn export_heap_holds_no_composite() {
    let output = composite(GUARD_SIDE);
    bounded("export_rgba", output, || with_square(GUARD_SIDE), export);
}

#[test]
fn trim_heap_holds_no_composite() {
    bounded("trim_bounds", 0, || with_square(GUARD_SIDE), trim);
}

#[test]
fn similar_heap_holds_no_composite() {
    let output = composite(GUARD_SIDE) / 4;
    bounded("similar", output, || with_marquee(GUARD_SIDE), similar);
}

/// A `side`² layered document whose two layers are canvas-sized and dense
/// (every pixel varies), so a copy of their planes would show in a peak.
fn dense_layers(side: u32) -> PictureViewRust {
    use crate::cxxqt_object::picture::readers_fixture::{layered, view_of};
    view_of(layered(side, side, false))
}

/// On a document with dense canvas-sized layers, so the Magnetic Lasso's
/// document clone would show if it copied their planes.
#[test]
fn magnetic_begin_heap_holds_no_composite() {
    bounded("magnetic_begin", 0, || dense_layers(GUARD_SIDE), magnetic);
}

#[test]
fn preview_heap_holds_no_composite() {
    bounded("preview_level", 0, || with_square(GUARD_SIDE), preview);
}

#[test]
fn gray_conversion_heap_holds_no_composite() {
    let output = composite(GUARD_SIDE) / 4;
    bounded("gray_conversion", output, || small_layers(GUARD_SIDE), gray);
}

#[test]
#[ignore = "17000x17000 reader heap peaks; run explicitly with --run-ignored only"]
fn readers_heap_peak_full() {
    report("export_rgba", || with_square(FULL_SIDE), export);
    report("trim_bounds", || with_square(FULL_SIDE), trim);
    report("similar", || with_marquee(FULL_SIDE), similar);
    report("magnetic_begin", || dense_layers(FULL_SIDE), magnetic);
    report("preview_level", || with_square(FULL_SIDE), preview);
    report("gray_conversion", || small_layers(FULL_SIDE), gray);
}
