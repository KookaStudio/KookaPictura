//! The picture as currently shown.
//!
//! The document session maintains `doc.composite` as the picture: the edit
//! transaction patches it regionally or stores it whole after every
//! pixel-changing commit and every presented preview, and history navigation
//! restores its bytes. A read-only consumer that only needs the picture
//! therefore does not have to run the compositor.
//!
//! The composite is *not* the picture in a few states, and this module is the
//! single place that decides so:
//!
//! - a paint or mask stroke is live: a layer stroke's composite lags its
//!   unpresented regions and a mask stroke never patches it;
//! - a lifted transform is live: the lift rewrote the layer stack without a
//!   recomposite;
//! - the working mode is not RGB, or the composite is not four channels: the
//!   composite then keeps the document's own planes, not RGBA;
//! - the document retains a native-depth composite: saving rewrites
//!   `doc.composite` from the native samples, which is not the 8-bit composite
//!   the consumers read;
//! - the composite's dimensions differ from the document's.
//!
//! Anything else is a bug in an edit path: it changed pixels without
//! refreshing the composite.
//!
//! The picture here is the composited layer stack. A soloed mask and the Layers
//! panel's isolation are display overlays and do not change it, exactly as the
//! old `current_buffer` consumers behaved.

use super::helpers_composite::current_buffer;
use super::state::PictureViewRust;
use pictura_core::{ColorMode, Document, PixelBuffer};
use std::borrow::Cow;

/// The picture the canvas currently shows.
///
/// `Cow::Borrowed` is the document's maintained composite — no allocation and
/// no compositor run. `Cow::Owned` is a fresh composite, returned only for the
/// states listed in the module documentation.
///
/// The returned picture borrows `doc`, so the caller cannot write the document
/// while it holds the picture. That is deliberate: cloning the composite would
/// make the next region patch copy the whole plane.
pub(super) fn picture<'a>(doc: &'a Document, view: &PictureViewRust) -> Cow<'a, PixelBuffer> {
    if is_current(doc, view) {
        return Cow::Borrowed(&doc.composite);
    }
    #[cfg(test)]
    COMPOSITES.with(|count| count.set(count.get() + 1));
    Cow::Owned(current_buffer(doc, view.gpu_compute))
}

/// Whether the maintained composite is the picture for this document state.
fn is_current(doc: &Document, view: &PictureViewRust) -> bool {
    !view.is_painting()
        && !view.has_lifted_transform()
        && doc.mode == ColorMode::Rgb
        && doc.source_depth.is_none()
        && doc.composite.channels == 4
        && doc.composite.width == doc.width
        && doc.composite.height == doc.height
}

#[cfg(test)]
std::thread_local! {
    /// Composites the accessor ran on this thread, for the tests that assert a
    /// consumer does not reach the compositor.
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
    use crate::cxxqt_object::tests::pixel_layer;
    use crate::history::History;
    use pictura_core::{BitDepth, PsdRect};
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

    #[test]
    fn picture_is_served_after_a_committed_edit() {
        let mut view = view_of(doc_with_layer());
        view.apply_edit("Edit", Edit::Full);
        {
            let doc = view.session.doc.as_ref().expect("document");
            assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
        }

        // The regional path is the common one: it patches the composite too.
        let mut view = view_of(doc_with_layer());
        view.apply_edit("Edit", Edit::regions(vec![doc_rect()], true));
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
    }

    #[test]
    fn picture_is_served_after_a_preview() {
        let mut view = view_of(doc_with_layer());
        view.preview_edit(&Edit::Full);
        {
            let doc = view.session.doc.as_ref().expect("document");
            assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
        }

        // The direct panel-preview path: mutate the layer, then refresh the
        // changed region instead of going through the transaction.
        let mut view = view_of(doc_with_layer());
        {
            let doc = view.session.doc.as_mut().expect("document");
            assert!(pictura_render::set_opacity_paths(doc, &["0"], 128) > 0);
        }
        view.refresh_regions(&[doc_rect()]);
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
    }

    #[test]
    fn picture_is_served_after_history_navigation() {
        let mut view = view_of(doc_with_layer());
        let target = view.layer_target.clone();
        view.session.record(&target, "Open");
        view.apply_edit("Edit", Edit::Full);

        assert!(view
            .session
            .restore(&mut view.layer_target, History::undo_live, History::undo)
            .is_some());
        {
            let doc = view.session.doc.as_ref().expect("document");
            assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
        }
        assert!(view
            .session
            .restore(&mut view.layer_target, History::redo_live, History::redo)
            .is_some());
        {
            let doc = view.session.doc.as_ref().expect("document");
            assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
        }
        assert!(view
            .session
            .restore(
                &mut view.layer_target,
                |history, doc, selection, target| history.jump_live(0, doc, selection, target),
                |history| history.jump(0)
            )
            .is_some());
        let doc = view.session.doc.as_ref().expect("document");
        assert!(matches!(picture(doc, &view), Cow::Borrowed(_)));
    }

    #[test]
    fn a_live_layer_stroke_is_composited() {
        let mut doc = doc_with_layer();
        let before = doc.composite.clone();
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
        assert_ne!(served(doc, &view), before, "the served frame has the dab");
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
        assert_ne!(served(doc, &stroke_view), doc.composite);

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
