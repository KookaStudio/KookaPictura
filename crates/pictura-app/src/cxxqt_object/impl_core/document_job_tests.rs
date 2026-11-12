//! Whole-document commands as cancellable jobs (#1278).

use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::tests::{pixel_layer, present_all};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, LayerStack};
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

fn wait(view: &PictureViewRust) {
    let started = Instant::now();
    while view
        .document_jobs
        .job
        .as_ref()
        .and_then(|job| job.worker.as_ref())
        .is_some_and(|worker| !worker.is_finished())
    {
        assert!(started.elapsed().as_secs() < 30, "the job never finished");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// What `document_job_finish` does, without the QObject's presentation.
fn land(view: &mut PictureViewRust) -> bool {
    let Some(done) = view.take_document_job() else {
        return view.note_document_op(None);
    };
    view.session.doc = Some(done.doc);
    let edit = (done.land)(view, &done.outcome);
    view.apply_edit(&done.label, edit);
    view.note_document_op(Some(done.outcome))
}

fn rotate(view: &mut PictureViewRust) -> bool {
    view.start_document_job(
        "Rotate",
        JobBar::Counted,
        |doc| {
            pictura_render::rotate_document(doc, 1)
                .ok()
                .map(|()| Outcome::default())
        },
        Box::new(|_, _| Edit::Full),
    )
}

fn merge_down(view: &mut PictureViewRust) -> bool {
    view.start_document_job(
        "Merge Down",
        JobBar::Counted,
        |doc| {
            let merged =
                pictura_render::merge_scope(doc, pictura_render::MergeScope::Down("1")).ok()?;
            Some(Outcome {
                path: merged.path,
                count: merged.replaced,
            })
        },
        Box::new(|rust, outcome| {
            let target = rust.merge_target(&outcome.path);
            rust.set_layer_target(target);
            Edit::Structural { damage: None }
        }),
    )
}

fn composite(view: &PictureViewRust) -> pictura_core::PixelBuffer {
    pictura_render::composite_rgba(view.session.doc.as_ref().expect("document"))
}

// [lop_whole_image_job]
#[test]
fn only_an_armed_command_on_a_large_document_leaves_the_gui_thread() {
    let mut small = view(64);
    small.document_jobs.armed = true;
    assert!(
        !small.document_job_wanted(),
        "a small document runs at once"
    );

    let side = (DOCUMENT_JOB_PIXELS as f64).sqrt().ceil() as u32;
    let mut large = PictureViewRust {
        session: DocumentSession::with_document(Document::new(
            side,
            side,
            ColorMode::Rgb,
            BitDepth::Eight,
        )),
        ..Default::default()
    };
    assert!(!large.document_job_wanted(), "an unarmed call runs at once");
    large.document_jobs.armed = true;
    assert!(large.document_job_wanted());
}

// [lop_whole_image_job]
#[test]
fn a_document_job_lands_one_history_state() {
    let mut view = view(64);
    let (count, before) = (view.session.history.count(), composite(&view));
    assert!(rotate(&mut view));
    assert!(view.document_job_pending());
    assert!(
        composite(&view) == before,
        "the document is unchanged until the job lands"
    );
    wait(&view);
    let progress = view
        .document_jobs
        .job
        .as_ref()
        .expect("job")
        .progress
        .clone();
    assert!(land(&mut view), "the job applied");
    assert_eq!(progress.permille(), 1000);
    assert!(!view.document_job_pending());
    assert_eq!(view.session.history.count(), count + 1);
    assert_eq!(view.session.history.label(count), "Rotate");
    assert!(composite(&view) != before, "the rotation landed");
}

// [lop_whole_image_cancel]
#[test]
fn a_cancelled_document_job_changes_nothing() {
    let mut view = view(64);
    let (count, index, before) = (
        view.session.history.count(),
        view.session.history.index(),
        composite(&view),
    );
    view.document_jobs.delay_for_test = 30_000;
    assert!(rotate(&mut view));
    view.document_jobs
        .job
        .as_ref()
        .expect("a job")
        .progress
        .cancel();
    assert!(!land(&mut view), "a cancelled job lands nothing");
    assert!(!view.document_job_pending());
    assert!(view.last_outcome().is_none());
    assert_eq!(view.session.history.count(), count);
    assert_eq!(view.session.history.index(), index);
    assert!(composite(&view) == before, "the composite changed");
}

// [lop_merge_job]
#[test]
fn a_merge_job_reports_the_merged_layer() {
    let mut view = view(64);
    let count = view.session.history.count();
    assert!(merge_down(&mut view));
    wait(&view);
    assert!(land(&mut view));
    let outcome = view.last_outcome().cloned().expect("an outcome");
    assert_eq!(outcome.count, 2, "two layers were merged");
    assert_eq!(view.layer_target.single(), Some(outcome.path.as_str()));
    assert_eq!(view.session.history.count(), count + 1);
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(doc.layers.len(), 1);
}

// [lop_whole_image_cancel]
#[test]
fn a_closed_document_drops_its_job() {
    let mut view = view(64);
    view.document_jobs.delay_for_test = 30_000;
    assert!(rotate(&mut view));
    let progress = view
        .document_jobs
        .job
        .as_ref()
        .expect("job")
        .progress
        .clone();
    view.cancel_move_composite();
    assert!(!view.document_job_pending());
    assert!(progress.cancelled(), "the worker was told to stop");
}
