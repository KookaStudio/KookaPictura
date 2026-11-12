//! The cancellable transform commit job (#1278).

use super::*;
use crate::cxxqt_object::document_session::{DocumentSession, TransformMode};
use crate::cxxqt_object::tests::{pixel_layer, present_all};
use pictura_core::{BitDepth, ColorMode, LayerStack};
use pictura_render::{Interpolation, LayerTransform};
use std::time::Instant;

fn view(side: u32) -> PictureViewRust {
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("under", side, side, (120, 170, 220)),
        pixel_layer("over", side / 2, side / 2, (220, 90, 40)),
    ]);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    present_all(&view);
    view.layer_target = LayerTarget::only("1");
    view.apply_edit("Open", Edit::Record);
    view
}

fn rotate(path: &str, lifted: bool) -> TransformSpec {
    let path = path.to_string();
    let target = path.clone();
    TransformSpec {
        paths: vec![path],
        orig_rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 32,
            right: 32,
        },
        apply: Arc::new(move |doc: &mut Document| {
            let turned = pictura_render::transform_layer_with(
                doc,
                &target,
                LayerTransform {
                    scale_x: 1.0,
                    scale_y: 1.0,
                    angle_radians: 0.3,
                    dx: 2.0,
                    dy: 1.0,
                },
                Interpolation::Bilinear,
            );
            turned && (!lifted || pictura_render::merge_lifted(doc, &target))
        }),
    }
}

fn start_session(view: &mut PictureViewRust) -> String {
    let session = view
        .prepare_transform("1", TransformMode::Free)
        .expect("a transform session");
    let path = session.path.clone();
    view.session.live.set_transform(session);
    path
}

fn wait(view: &PictureViewRust) {
    let started = Instant::now();
    while view.transform_commit_running() {
        assert!(started.elapsed().as_secs() < 30, "the job never finished");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn composite(view: &PictureViewRust) -> pictura_core::PixelBuffer {
    pictura_render::composite_rgba(view.session.doc.as_ref().expect("document"))
}

fn cancel(view: &mut PictureViewRust) {
    view.deferred_transform
        .as_ref()
        .expect("a job")
        .progress
        .cancel();
    wait(view);
}

#[test]
fn transform_commit_lands_one_history_state() {
    let mut view = view(64);
    let path = start_session(&mut view);
    let count = view.session.history.count();
    assert!(view.defer_transform("Free Transform", rotate(&path, false), false, None));
    assert!(view.transform_pending());
    wait(&view);
    let progress = view
        .deferred_transform
        .as_ref()
        .expect("job")
        .progress
        .clone();
    let (label, edit) = view.land_transform().expect("the job landed");
    assert_eq!(progress.permille(), 1000);
    view.apply_transform(&label, edit, false);
    assert_eq!(view.session.history.count(), count + 1);
    assert_eq!(view.session.history.label(count), "Free Transform");
    assert!(view.session.live.transform().is_none(), "the session ended");
    assert!(!view.transform_pending());
}

#[test]
fn transform_commit_cancel_changes_nothing() {
    let mut view = view(64);
    let path = start_session(&mut view);
    let (count, index, before) = (
        view.session.history.count(),
        view.session.history.index(),
        composite(&view),
    );
    view.transform_delay_for_test = 30_000;
    assert!(view.defer_transform("Free Transform", rotate(&path, false), false, None));
    cancel(&mut view);
    assert!(
        view.land_transform().is_none(),
        "a cancelled job lands nothing"
    );
    assert_eq!(view.session.history.count(), count);
    assert_eq!(view.session.history.index(), index);
    assert!(composite(&view) == before, "the composite changed");
}

#[test]
fn transform_commit_cancel_resumes_the_session() {
    let mut view = view(64);
    let path = start_session(&mut view);
    let quad = view
        .session
        .live
        .transform()
        .map(|s| (s.path.clone(), s.orig_rect));
    view.transform_delay_for_test = 30_000;
    assert!(view.defer_transform("Free Transform", rotate(&path, false), false, None));
    cancel(&mut view);
    assert!(view.land_transform().is_none());
    assert!(!view.transform_pending());
    let resumed = view
        .session
        .live
        .transform()
        .map(|s| (s.path.clone(), s.orig_rect));
    assert_eq!(resumed, quad, "the same session is still active");
    // A second commit of the resumed session lands.
    view.transform_delay_for_test = 0;
    assert!(view.defer_transform("Free Transform", rotate(&path, false), false, None));
    wait(&view);
    assert!(view.land_transform().is_some());
}

#[test]
fn lifted_transform_commit_runs_as_a_job() {
    let mut view = view(64);
    view.session
        .set_selection(Some(pictura_select::Selection::rect(64, 64, 4, 4, 16, 16)));
    let path = start_session(&mut view);
    let lifted = |view: &PictureViewRust| {
        view.session
            .live
            .transform()
            .is_some_and(|s| s.lifted.is_some())
    };
    assert!(lifted(&view), "the selection was lifted");
    let (count, before) = (view.session.history.count(), composite(&view));
    view.transform_delay_for_test = 30_000;
    assert!(view.defer_transform("Free Transform", rotate(&path, true), true, None));
    cancel(&mut view);
    assert!(view.land_transform().is_none());
    assert!(lifted(&view), "the lifted session resumes");
    assert!(composite(&view) == before);
    assert!(view.session.selection.is_some());

    view.transform_delay_for_test = 0;
    assert!(view.defer_transform("Free Transform", rotate(&path, true), true, None));
    wait(&view);
    let (label, edit) = view.land_transform().expect("landed");
    view.apply_transform(&label, edit, false);
    assert_eq!(view.session.history.count(), count + 1);
    assert!(view.session.live.transform().is_none());
    assert!(
        view.session.selection.is_none(),
        "the selection border is dropped"
    );
    let doc = view.session.doc.as_ref().expect("document");
    assert!(
        pictura_render::resolve_path(doc, &path).is_none(),
        "the lifted layer was merged back"
    );
}
