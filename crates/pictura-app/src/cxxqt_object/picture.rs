//! The picture as currently shown.
//!
//! A document whose composite is authoritative (no layers, a non-RGB mode, a
//! retained native-depth or merged composite) keeps it stored, and the edit
//! transaction keeps it current. Every other document keeps no composite
//! (#1216): its picture is the display tile cache where that is current and
//! needs no colour conversion, and a region composite anywhere else.
//!
//! The stored or cached composite is *not* the picture in a few states, and
//! this module is the single place that decides so:
//!
//! - a paint or mask stroke is live: a layer stroke's tiles lag its
//!   unpresented regions and a mask stroke never repairs them;
//! - a lifted transform is live: the lift rewrote the layer stack without a
//!   refresh;
//! - a display-only preview is presented: its tiles are approximate;
//! - the working mode is not RGB, or the composite is not four channels: the
//!   stored composite then keeps the document's own planes, not RGBA;
//! - the document retains a native-depth composite: saving rewrites
//!   `doc.composite` from the native samples, which is not the 8-bit composite
//!   the consumers read;
//! - the composite's dimensions differ from the document's;
//! - a large Move commit's composite is still on its worker (#1157).
//!
//! Anything else is a bug in an edit path: it changed pixels without
//! reporting their damage.
//!
//! The picture here is the composited layer stack. A soloed mask and the Layers
//! panel's isolation are display overlays and do not change it.

use super::helpers_composite::crop_planar;
use super::state::PictureViewRust;
use pictura_core::{Document, PixelBuffer, PsdRect};
use pictura_render::{patch_buffer_region, TileSource};
use std::borrow::Cow;

/// The whole picture the canvas currently shows.
///
/// `Cow::Borrowed` is a stored authoritative composite. Anything else is
/// assembled by [`picture_region`] over the whole document.
///
/// ponytail: a whole picture is one document-sized buffer. Its callers are the
/// readers that need every pixel at once (raster export, Copy Merged, Color
/// Range, Refine Edge, the image operations); the region readers use
/// [`picture_region`].
pub(super) fn picture<'a>(doc: &'a Document, view: &PictureViewRust) -> Cow<'a, PixelBuffer> {
    if stored_is_current(doc, view) {
        return Cow::Borrowed(&doc.composite);
    }
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    Cow::Owned(picture_region(doc, view, whole))
}

/// `rect` of the picture as currently shown: straight working-space RGBA,
/// planar, transparent outside the document. Byte-identical to the same crop
/// of a full composite. Costs a region-sized allocation; the compositor runs
/// only for what neither the stored composite nor the current tiles hold.
pub(super) fn picture_region(doc: &Document, view: &PictureViewRust, rect: PsdRect) -> PixelBuffer {
    let (rw, rh) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
    let inside = PsdRect {
        top: rect.top.max(0),
        left: rect.left.max(0),
        bottom: rect.bottom.min(doc.height as i32),
        right: rect.right.min(doc.width as i32),
    };
    if inside.width() <= 0 || inside.height() <= 0 {
        return PixelBuffer::new(rw, rh, 4);
    }
    let pixels = if stored_is_current(doc, view) {
        crop_planar(
            &doc.composite,
            inside.left,
            inside.top,
            inside.width() as u32,
            inside.height() as u32,
        )
    } else if let Some(cached) = from_tiles(doc, view, inside) {
        cached
    } else {
        #[cfg(test)]
        if !doc.layers.is_empty() {
            COMPOSITES.with(|count| count.set(count.get() + 1));
        }
        pictura_render::composite_region(doc, inside)
    };
    if inside == rect {
        return pixels;
    }
    let mut out = PixelBuffer::new(rw, rh, 4);
    patch_buffer_region(
        &mut out,
        &pixels,
        inside.left - rect.left,
        inside.top - rect.top,
    );
    out
}

/// Tiles a Sample All Layers flood keeps: a flood revisits a tile's rows, and
/// 64 tiles are 16 MiB.
const SAMPLE_MEMO: usize = 64;

/// The picture as a Sample All Layers flood reads it: `fetch(tx, ty)` returns
/// the straight interleaved RGBA of one 256² level-0 tile, cropped from the
/// stored composite when that is current or composited for the tile alone.
/// The last [`SAMPLE_MEMO`] tiles are kept, so the flood never holds a whole
/// composite (#1216).
pub(super) fn sample_tiles<'a>(
    doc: &'a Document,
    view: &PictureViewRust,
) -> impl Fn(usize, usize) -> std::sync::Arc<[u8]> + Sync + 'a {
    sample_tiles_from(doc, stored_is_current(doc, view))
}

/// [`sample_tiles`] given whether the stored composite is current, read
/// beforehand, so the flood can run off the GUI thread (#1300).
pub(super) fn sample_tiles_from(
    doc: &Document,
    stored: bool,
) -> impl Fn(usize, usize) -> std::sync::Arc<[u8]> + Sync + '_ {
    let memo = std::sync::Mutex::new(std::collections::VecDeque::<(
        (usize, usize),
        std::sync::Arc<[u8]>,
    )>::new());
    move |tx, ty| {
        if let Some((_, bytes)) = memo
            .lock()
            .expect("the memo lock")
            .iter()
            .find(|(at, _)| *at == (tx, ty))
        {
            return std::sync::Arc::clone(bytes);
        }
        let side = pictura_core::TILE as i32;
        let rect = PsdRect {
            top: ty as i32 * side,
            left: tx as i32 * side,
            bottom: (ty as i32 * side + side).min(doc.height as i32),
            right: (tx as i32 * side + side).min(doc.width as i32),
        };
        let pixels = if stored {
            crop_planar(
                &doc.composite,
                rect.left,
                rect.top,
                rect.width().max(0) as u32,
                rect.height().max(0) as u32,
            )
        } else {
            pictura_render::composite_region(doc, rect)
        };
        let bytes: std::sync::Arc<[u8]> = pictura_render::interleave(&pixels).into();
        let mut memo = memo.lock().expect("the memo lock");
        if memo.len() == SAMPLE_MEMO {
            memo.pop_front();
        }
        memo.push_back(((tx, ty), std::sync::Arc::clone(&bytes)));
        bytes
    }
}

/// `rect` from the current level-0 display tiles, when the picture is exactly
/// what they hold: a layered RGB document with no stored composite, no colour
/// conversion between it and the display, no state that makes them stale,
/// and every tile under `rect` current. `None` otherwise.
fn from_tiles(doc: &Document, view: &PictureViewRust, rect: PsdRect) -> Option<PixelBuffer> {
    if doc.composite_is_authoritative() || doc.document_icc.is_some() || !tiles_are_current(view) {
        return None;
    }
    view.fit_tiles();
    let mut cache = view.tiles.cache.borrow_mut();
    let keys = cache.keys_in(TileSource::Composite, 0, rect);
    if !keys.iter().all(|&key| cache.contains(key)) {
        return None;
    }
    let (rw, rh) = (rect.width() as usize, rect.height() as usize);
    let plane = rw * rh;
    let mut data = vec![0u8; plane * 4];
    for key in keys {
        let tile = cache.tile_rect(key);
        let bytes = cache.get(key)?;
        let tw = tile.width() as usize;
        for y in tile.top.max(rect.top)..tile.bottom.min(rect.bottom) {
            for x in tile.left.max(rect.left)..tile.right.min(rect.right) {
                let s = ((y - tile.top) as usize * tw + (x - tile.left) as usize) * 4;
                let d = (y - rect.top) as usize * rw + (x - rect.left) as usize;
                for c in 0..4 {
                    data[c * plane + d] = bytes[s + c];
                }
            }
        }
    }
    Some(PixelBuffer {
        width: rw as u32,
        height: rh as u32,
        channels: 4,
        data: data.into(),
    })
}

/// Whether nothing live makes the stored composite or the tiles lag the
/// layer stack.
fn tiles_are_current(view: &PictureViewRust) -> bool {
    !view.is_painting()
        && !view.has_lifted_transform()
        && !view.display_preview_presented()
        && view.deferred_composite.is_none()
}

/// Whether the stored composite is the picture for this document state. Only
/// an authoritative composite is kept current; any other is a leftover (a
/// document as constructed) that no edit maintains.
pub(super) fn stored_is_current(doc: &Document, view: &PictureViewRust) -> bool {
    tiles_are_current(view)
        && doc.composite_is_authoritative()
        && doc.source_depth.is_none()
        && pictura_render::DocumentComposite::is_shape_valid(doc)
}

#[cfg(test)]
std::thread_local! {
    /// Region composites the accessor ran on this thread, for the tests that
    /// assert a consumer does not reach the compositor.
    static COMPOSITES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times the accessor composited on this thread so far.
#[cfg(test)]
pub(super) fn composites_so_far() -> u64 {
    COMPOSITES.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::document_session::{
        DocumentSession, MaskStroke, StrokeSession, TransformMode,
    };
    use crate::cxxqt_object::edit::Edit;
    use crate::cxxqt_object::helpers_composite::current_buffer;
    use crate::cxxqt_object::tests::pixel_layer;
    use crate::history::History;
    use pictura_core::{BitDepth, ColorMode, PsdRect};
    use pictura_paint::{Stroke, StrokeConfig, StrokeSample};
    use pictura_select::Selection;

    const SIDE: u32 = 16;

    fn doc_with_layer() -> Document {
        let rgba = [255u8, 0, 0, 255].repeat((SIDE * SIDE) as usize);
        Document::from_rgba("Layer 0", SIDE, SIDE, &rgba)
    }

    fn view_of(doc: Document) -> PictureViewRust {
        PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        }
    }

    fn doc_rect() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: SIDE as i32,
            right: SIDE as i32,
        }
    }

    fn served(doc: &Document, view: &PictureViewRust) -> PixelBuffer {
        picture(doc, view).into_owned()
    }

    /// Present the whole document at level 0, as the canvas would, so its
    /// tiles are cached.
    fn present(view: &PictureViewRust) {
        let (_, missing) = view.tile_crop(TileSource::Composite, 0, doc_rect());
        assert!(
            missing.is_empty(),
            "a small document builds its tiles at once"
        );
    }

    /// Whether the current picture is served without running the compositor,
    /// and equals a full composite.
    fn served_from_the_cache(view: &PictureViewRust) -> bool {
        let doc = view.session.doc.as_ref().expect("document");
        let before = composites_so_far();
        let region = picture_region(doc, view, doc_rect());
        assert_eq!(
            region,
            pictura_render::composite_rgba(doc),
            "the picture is exact"
        );
        composites_so_far() == before
    }

    /// Restore through history and present the result the way the view does.
    fn restore_and_present(
        view: &mut PictureViewRust,
        step: impl FnOnce(
            &mut History,
            &mut Document,
            &mut Option<Selection>,
            &mut crate::layer_target::LayerTarget,
        ) -> Option<crate::history::Restored>,
        legacy: impl FnOnce(&mut History) -> Option<crate::history::Snapshot>,
    ) {
        let restored = view
            .session
            .restore(&mut view.layer_target, step, legacy)
            .expect("a state to restore");
        let presentation = view.restore_presentation(restored, false);
        crate::cxxqt_object::presentation::present_core(view, &presentation);
    }

    #[test]
    fn picture_is_served_after_a_committed_edit() {
        // A whole-document edit leaves the tiles stale; once the canvas has
        // presented them, the picture is served from them.
        let mut view = view_of(doc_with_layer());
        view.apply_edit("Edit", Edit::Full);
        present(&view);
        assert!(served_from_the_cache(&view));

        // The regional path is the common one: it repairs the presented tiles.
        let mut view = view_of(doc_with_layer());
        present(&view);
        view.apply_edit("Edit", Edit::regions(vec![doc_rect()], true));
        assert!(served_from_the_cache(&view));
    }

    #[test]
    fn picture_is_served_after_a_preview() {
        let mut view = view_of(doc_with_layer());
        view.preview_edit(&Edit::Full);
        present(&view);
        assert!(served_from_the_cache(&view));

        // The direct panel-preview path: mutate the layer, then refresh the
        // changed region instead of going through the transaction.
        let mut view = view_of(doc_with_layer());
        present(&view);
        {
            let doc = view.session.doc.as_mut().expect("document");
            assert!(pictura_render::set_opacity_paths(doc, &["0"], 128) > 0);
        }
        view.refresh_regions(&[doc_rect()]);
        assert!(served_from_the_cache(&view));
    }

    #[test]
    fn picture_is_served_after_history_navigation() {
        let mut view = view_of(doc_with_layer());
        let target = view.layer_target.clone();
        view.session.record(&target, "Open");
        view.apply_edit("Edit", Edit::Full);

        present(&view);

        restore_and_present(&mut view, History::undo_live, History::undo);
        present(&view);
        assert!(served_from_the_cache(&view));
        restore_and_present(&mut view, History::redo_live, History::redo);
        present(&view);
        assert!(served_from_the_cache(&view));
        restore_and_present(
            &mut view,
            |history, doc, selection, target| history.jump_live(0, doc, selection, target),
            |history| history.jump(0),
        );
        present(&view);
        assert!(served_from_the_cache(&view));
    }

    #[test]
    fn a_display_preview_is_composited() {
        let mut view = view_of(doc_with_layer());
        view.presented_level = 0;
        view.presented_rect = doc_rect();
        view.reset_pyramid();
        view.begin_display_preview(
            vec!["0".to_string()],
            crate::cxxqt_object::display_preview::PreviewKind::Adjustment,
        );
        assert!(view.present_display_preview().is_some());
        let doc = view.session.doc.as_ref().expect("document");
        let before = composites_so_far();
        assert_eq!(served(doc, &view), current_buffer(doc, false));
        assert!(
            composites_so_far() > before,
            "the approximate tiles are not served"
        );
    }

    #[test]
    fn a_live_layer_stroke_is_composited() {
        let mut doc = doc_with_layer();
        let before = pictura_render::composite_rgba(&doc);
        let mut stroke =
            Stroke::begin_at(&doc, "0", StrokeConfig::default()).expect("a stroke begins");
        assert!(stroke.sample(
            &mut doc,
            StrokeSample {
                x: 8.0,
                y: 8.0,
                pressure: 1.0,
            }
        ));
        let mut view = view_of(doc);
        view.session.live.set_stroke_session(StrokeSession {
            stroke: Some(stroke),
            ..Default::default()
        });
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Owned(_)));
        assert_ne!(
            served(doc, &view).data,
            before.data,
            "the served frame has the dab"
        );
        assert_eq!(served(doc, &view), current_buffer(doc, false));
    }

    #[test]
    fn a_live_mask_stroke_is_composited() {
        let mut view = view_of(doc_with_layer());
        let mut mask_doc = Document::from_rgba("0", SIDE, SIDE, &[0u8; (SIDE * SIDE * 4) as usize]);
        let mut stroke =
            Stroke::begin_at(&mask_doc, "0", StrokeConfig::default()).expect("a stroke begins");
        assert!(stroke.sample(
            &mut mask_doc,
            StrokeSample {
                x: 8.0,
                y: 8.0,
                pressure: 1.0,
            }
        ));
        view.session.live.set_stroke_session(StrokeSession {
            mask_stroke: Some(MaskStroke {
                baseline: None,
                path: "0".to_string(),
                rect: doc_rect(),
                document: mask_doc,
                stroke,
            }),
            ..Default::default()
        });
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Owned(_)));
        assert_eq!(served(doc, &view), current_buffer(doc, false));
    }

    #[test]
    fn a_lifted_transform_is_composited() {
        let mut doc = doc_with_layer();
        doc.layers[0].opacity = 128;
        doc.composite = pictura_render::composite_rgba(&doc);
        let mut view = view_of(doc);
        view.session.selection = Some(Selection::all(SIDE, SIDE));
        let session = view
            .prepare_transform("0", TransformMode::Free)
            .expect("a lifted transform begins");
        view.session.live.set_transform(session);
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Owned(_)));
        assert_eq!(served(doc, &view), current_buffer(doc, false));
    }

    #[test]
    fn a_grayscale_document_is_composited() {
        let mut doc = doc_with_layer();
        doc.mode = ColorMode::Grayscale;
        doc.composite = PixelBuffer {
            width: SIDE,
            height: SIDE,
            channels: 1,
            data: vec![100u8; (SIDE * SIDE) as usize].into(),
        };
        let view = view_of(doc);
        let doc = view.session.doc.as_ref().expect("document");
        let served = served(doc, &view);
        assert!(matches!(picture(doc, &view), Cow::Owned(_)));
        assert_eq!(served.channels, 4);
        assert_eq!(served, current_buffer(doc, false));
    }

    #[test]
    fn a_retained_native_composite_is_composited() {
        let mut doc = doc_with_layer();
        doc.source_depth = Some(BitDepth::Sixteen);
        let view = view_of(doc);
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Owned(_)));
        assert_eq!(served(doc, &view), current_buffer(doc, false));
    }

    #[test]
    fn a_mismatched_composite_is_composited() {
        let mut doc = doc_with_layer();
        doc.composite = PixelBuffer {
            width: SIDE / 2,
            height: SIDE / 2,
            channels: 4,
            data: vec![0u8; (SIDE * SIDE) as usize].into(),
        };
        let view = view_of(doc);
        let doc = view.session.doc.as_ref().expect("document");
        let served = served(doc, &view);
        assert!(matches!(picture(doc, &view), Cow::Owned(_)));
        assert_eq!((served.width, served.height), (SIDE, SIDE));
        assert_eq!(served, current_buffer(doc, false));
    }

    #[test]
    fn a_layerless_document_keeps_its_embedded_composite() {
        let mut doc = Document::new(SIDE, SIDE, ColorMode::Rgb, BitDepth::Eight);
        doc.composite = PixelBuffer {
            width: SIDE,
            height: SIDE,
            channels: 4,
            data: vec![42u8; (SIDE * SIDE * 4) as usize].into(),
        };
        let view = view_of(doc);
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
        assert_eq!(served(doc, &view), doc.composite);
    }

    #[test]
    fn the_served_picture_equals_a_full_composite() {
        // A fresh document: the cache is a full composite.
        let fresh = view_of(doc_with_layer());
        let doc = fresh.session.doc.as_ref().unwrap();
        assert_eq!(served(doc, &fresh), pictura_render::composite_rgba(doc));

        // A previewed document: the cache includes the preview.
        let mut previewed = view_of(doc_with_layer());
        previewed.preview_edit(&Edit::Full);
        let doc = previewed.session.doc.as_ref().unwrap();
        assert_eq!(served(doc, &previewed), pictura_render::composite_rgba(doc));

        // A live stroke: the fallback composite, not the lagging cache.
        let mut stroke_view = view_of(doc_with_layer());
        let mut doc = doc_with_layer();
        let mut stroke = Stroke::begin_at(&doc, "0", StrokeConfig::default()).expect("stroke");
        assert!(stroke.sample(
            &mut doc,
            StrokeSample {
                x: 8.0,
                y: 8.0,
                pressure: 1.0,
            }
        ));
        stroke_view.session.doc = Some(doc);
        stroke_view.session.live.set_stroke_session(StrokeSession {
            stroke: Some(stroke),
            ..Default::default()
        });
        let doc = stroke_view.session.doc.as_ref().unwrap();
        assert_eq!(served(doc, &stroke_view), current_buffer(doc, false));
        assert_ne!(
            served(doc, &stroke_view),
            pictura_render::composite_rgba(&doc_with_layer())
        );

        // A lifted transform: the lift changed the composite, so the fallback
        // must be recomputed.
        let mut lifted_doc = doc_with_layer();
        lifted_doc.layers[0].opacity = 128;
        lifted_doc.composite = pictura_render::composite_rgba(&lifted_doc);
        let mut lifted = view_of(lifted_doc);
        lifted.session.selection = Some(Selection::all(SIDE, SIDE));
        let session = lifted
            .prepare_transform("0", TransformMode::Free)
            .expect("a lifted transform begins");
        lifted.session.live.set_transform(session);
        let doc = lifted.session.doc.as_ref().unwrap();
        assert_eq!(served(doc, &lifted), current_buffer(doc, false));
        assert_ne!(served(doc, &lifted), doc.composite);

        // A converted Grayscale document: a four-channel composite, not the
        // one-plane cache.
        let mut gray = doc_with_layer();
        gray.mode = ColorMode::Grayscale;
        gray.composite = PixelBuffer {
            width: SIDE,
            height: SIDE,
            channels: 1,
            data: vec![7u8; (SIDE * SIDE) as usize].into(),
        };
        let gray = view_of(gray);
        let doc = gray.session.doc.as_ref().unwrap();
        let served_gray = served(doc, &gray);
        assert_eq!(served_gray.channels, 4);
        assert_eq!(served_gray, current_buffer(doc, false));

        // A layerless document: the embedded composite, not an empty stack.
        let mut layerless = Document::new(SIDE, SIDE, ColorMode::Rgb, BitDepth::Eight);
        layerless.composite = PixelBuffer {
            width: SIDE,
            height: SIDE,
            channels: 4,
            data: vec![9u8; (SIDE * SIDE * 4) as usize].into(),
        };
        let layerless = view_of(layerless);
        let doc = layerless.session.doc.as_ref().unwrap();
        assert_eq!(served(doc, &layerless), doc.composite);

        // A mismatched composite: recomputed at the document's dimensions.
        let mut mismatched = doc_with_layer();
        mismatched.composite = PixelBuffer {
            width: SIDE / 2,
            height: SIDE / 2,
            channels: 4,
            data: vec![0u8; (SIDE * SIDE) as usize].into(),
        };
        let mismatched = view_of(mismatched);
        let doc = mismatched.session.doc.as_ref().unwrap();
        let served_mismatched = served(doc, &mismatched);
        assert_eq!(
            (served_mismatched.width, served_mismatched.height),
            (SIDE, SIDE)
        );
        assert_eq!(served_mismatched, current_buffer(doc, false));
    }

    /// Print-only before/after for the current-picture accessor: on a 4000²
    /// eight-layer document, `current_buffer` composites the whole stack while
    /// `picture` serves the maintained composite. Reports the median of three runs
    /// of each so the PR carries a measured win, not an estimate.
    #[test]
    #[ignore = "4000x4000 layered profile; run explicitly with --ignored --nocapture"]
    fn picture_serve_profile_4000() {
        use std::time::{Duration, Instant};

        let layers: Vec<_> = (0..8)
            .map(|i| pixel_layer(&format!("L{i}"), 4000, 4000, (i * 30, 60, 90)))
            .collect();
        let mut doc = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(layers);
        doc.composite = pictura_render::composite_rgba(&doc);
        let view = PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        };
        let doc = view.session.doc.as_ref().expect("document");

        let mut old = Vec::new();
        let mut new = Vec::new();
        for _ in 0..3 {
            let t = Instant::now();
            let buffer = current_buffer(doc, false);
            old.push(t.elapsed());
            std::hint::black_box(&buffer);

            let t = Instant::now();
            let served = picture(doc, &view);
            new.push(t.elapsed());
            std::hint::black_box(&served);
        }
        old.sort();
        new.sort();
        let median = |times: &[Duration]| times[times.len() / 2];
        println!(
            "picture_serve_profile 4000x4000 8 layers: current_buffer median {:.2?}, \
             picture median {:.2?}, ratio {:.0}x",
            median(&old),
            median(&new),
            median(&old).as_secs_f64() / median(&new).as_secs_f64().max(1e-9),
        );

        // The Sample All Layers consumer: the surface conversion is unchanged, so
        // the win is the composite it no longer runs.
        let mut old_surface = Vec::new();
        let mut new_surface = Vec::new();
        for _ in 0..3 {
            let t = Instant::now();
            let surface = pictura_paint::stamp::surface_from_composite(&current_buffer(doc, false));
            old_surface.push(t.elapsed());
            std::hint::black_box(&surface);

            let t = Instant::now();
            let surface = pictura_paint::stamp::surface_from_composite(&picture(doc, &view));
            new_surface.push(t.elapsed());
            std::hint::black_box(&surface);
        }
        old_surface.sort();
        new_surface.sort();
        println!(
            "picture_serve_profile sample-all-layers surface: before median {:.2?}, \
             after median {:.2?}, ratio {:.1}x",
            median(&old_surface),
            median(&new_surface),
            median(&old_surface).as_secs_f64() / median(&new_surface).as_secs_f64().max(1e-9),
        );
    }
}
