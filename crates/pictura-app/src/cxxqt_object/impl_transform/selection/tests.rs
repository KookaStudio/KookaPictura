//! Transform Selection's own checks: begin, commit, cancel, the mask contract,
//! the mapped preview contour, and the repeat target.

use super::super::super::document_session::DocumentSession;
use super::super::super::document_session::TransformTarget;
use super::super::super::state::PictureViewRust;
use super::super::super::tests::pixel_layer;
use crate::cxxqt_object::document_session::TransformMode;
use crate::history::History;
use crate::layer_target::LayerTarget;
use pictura_core::warp::{Warp, WarpStyle};
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PixelBuffer, PsdRect};
use pictura_select::Selection;

fn doc() -> Document {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("px", 16, 16, (255, 0, 0))]);
    doc.composite
        .replace(PixelBuffer {
            width: 16,
            height: 16,
            channels: 4,
            data: vec![0u8; 16 * 16 * 4].into(),
        })
        .expect("valid composite");
    doc
}

fn view() -> PictureViewRust {
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc()),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view.session
        .set_selection(Some(Selection::rect(16, 16, 3, 3, 8, 6)));
    view
}

fn scale_session(view: &mut PictureViewRust, scale: f64) {
    let session = view.session.live.transform_mut().expect("a live session");
    session.scale_x = scale;
    session.scale_y = scale;
}

fn bounds(view: &PictureViewRust) -> Option<PsdRect> {
    view.session.selection.as_ref().and_then(Selection::bounds)
}

#[test]
fn selection_transform_requires_a_selection() {
    let mut view = view();
    view.session.set_selection(None);
    assert!(!view.begin_selection_transform());
    view.session.set_selection(Some(Selection::none(16, 16)));
    assert!(!view.begin_selection_transform());
}

#[test]
fn selection_transform_commit_replaces_the_mask() {
    let mut view = view();
    let before = bounds(&view).expect("a selection");
    assert!(view.begin_selection_transform());
    scale_session(&mut view, 2.0);
    assert!(view.commit_selection_transform());
    let after = bounds(&view).expect("the transformed border");
    assert!((after.width() - before.width() * 2).abs() <= 1);
    assert!((after.height() - before.height() * 2).abs() <= 1);
    assert!(!view.selection_transform_live());
}

#[test]
fn selection_transform_cancel_keeps_the_mask() {
    let mut view = view();
    let before = view.session.selection.clone();
    assert!(view.begin_selection_transform());
    scale_session(&mut view, 3.0);
    let _ = view.session.live.take_transform();
    assert_eq!(view.session.selection, before);
}

#[test]
fn selection_transform_identity_records_nothing() {
    let mut view = view();
    let before = view.session.selection.clone();
    let history = view.session.history.count();
    assert!(view.begin_selection_transform());
    assert!(!view.commit_selection_transform());
    assert_eq!(view.session.selection, before);
    assert_eq!(view.session.history.count(), history);
}

#[test]
fn selection_transform_keeps_layer_bytes() {
    let mut view = view();
    let before: Vec<Vec<u8>> = view
        .session
        .doc
        .as_ref()
        .unwrap()
        .layers
        .iter()
        .flat_map(|layer| layer.channels().iter().map(|channel| channel.data.to_vec()))
        .collect();
    assert!(view.begin_selection_transform());
    scale_session(&mut view, 1.5);
    assert!(view.commit_selection_transform());
    let after: Vec<Vec<u8>> = view
        .session
        .doc
        .as_ref()
        .unwrap()
        .layers
        .iter()
        .flat_map(|layer| layer.channels().iter().map(|channel| channel.data.to_vec()))
        .collect();
    assert_eq!(after, before);
}

#[test]
fn selection_transform_undo_restores_the_mask() {
    let mut view = view();
    let pre = view.session.selection.clone();
    assert!(view.begin_selection_transform());
    scale_session(&mut view, 2.0);
    let mut doc = view.session.doc.take().expect("the document");
    let mut target = LayerTarget::default();
    let mut history = History::default();
    history.capture_live(&mut doc, &view.session.selection, &target, "Open");
    assert!(view.commit_selection_transform());
    history.capture_live(
        &mut doc,
        &view.session.selection,
        &target,
        "Transform Selection",
    );
    let restored = history.undo_live(&mut doc, &mut view.session.selection, &mut target);
    assert!(restored.is_some());
    assert_eq!(view.session.selection, pre);
    view.session.doc.set(Some(doc));
}

#[test]
fn selection_transform_contour_follows_the_box() {
    let mut view = view();
    assert!(view.begin_selection_transform());
    let identity = view.selection_transform_contour();
    assert!(!identity.is_empty());
    scale_session(&mut view, 2.0);
    let scaled = view.selection_transform_contour();
    let extent = |loops: &[f64]| {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut i = 0;
        while i < loops.len() {
            let n = loops[i] as usize;
            i += 1;
            for _ in 0..n {
                if i + 1 >= loops.len() {
                    break;
                }
                min = min.min(loops[i]);
                max = max.max(loops[i]);
                i += 2;
            }
        }
        (min, max)
    };
    let (before_lo, before_hi) = extent(&identity);
    let (after_lo, after_hi) = extent(&scaled);
    assert!(after_hi - after_lo > (before_hi - before_lo) * 1.5);
}

#[test]
fn selection_transform_last_target_enables_again() {
    let mut view = view();
    assert!(view.begin_selection_transform());
    scale_session(&mut view, 1.5);
    assert!(view.commit_selection_transform());
    let repeat = view.last_transform.expect("a repeatable border transform");
    assert_eq!(repeat.target, TransformTarget::Selection);

    assert!(view.begin_selection_transform());
    {
        let session = view.session.live.transform_mut().expect("a live session");
        session.mode = TransformMode::Distort;
        session.quad = Some([(3.0, 3.0), (12.0, 4.0), (11.0, 12.0), (2.0, 11.0)]);
    }
    assert!(view.commit_selection_transform());
    assert!(
        view.last_transform.is_none(),
        "a projective commit is not repeatable"
    );
}

#[test]
fn selection_transform_warp_preview_and_commit() {
    let mut view = view();
    assert!(view.begin_selection_transform());
    {
        let session = view.session.live.transform_mut().expect("a live session");
        let rect = session.orig_rect;
        session.warp = Some(Warp::preset(
            WarpStyle::Arc,
            50.0,
            [
                f64::from(rect.left),
                f64::from(rect.top),
                f64::from(rect.right),
                f64::from(rect.bottom),
            ],
        ));
    }
    assert!(!view.selection_transform_contour().is_empty());
    let before = view.session.selection.clone();
    assert!(view.commit_selection_transform());
    assert_ne!(view.session.selection, before);
    assert!(view.last_transform.is_none());
}

#[test]
fn selection_transform_warp_commit() {
    let mut view = view();
    assert!(view.begin_selection_transform());
    {
        let session = view.session.live.transform_mut().expect("a live session");
        let rect = session.orig_rect;
        session.warp = Some(Warp::preset(
            WarpStyle::Wave,
            40.0,
            [
                f64::from(rect.left),
                f64::from(rect.top),
                f64::from(rect.right),
                f64::from(rect.bottom),
            ],
        ));
    }
    let before = view.session.selection.clone();
    assert!(view.commit_selection_transform());
    assert_ne!(view.session.selection, before);
}
