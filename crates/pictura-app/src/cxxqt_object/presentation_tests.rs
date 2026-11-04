//! The presentation order: bookkeeping, then regions, then one rebuild, then
//! at most one notification. These tests need no QObject.

use super::{plan, present_core, Dirty, PresentStep, Presentation, Rebuild};
use crate::cxxqt_object::tests::pixel_layer;
use crate::cxxqt_object::PictureViewRust;
use crate::history::{History, Restored};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn doc_with_layer() -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 4, 4, (255, 0, 0))]);
    doc.composite = pictura_render::composite_rgba(&doc);
    doc
}

fn view() -> PictureViewRust {
    PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(
            doc_with_layer(),
        ),
        ..Default::default()
    }
}

#[test]
fn bookkeeping_precedes_blit() {
    let presentation = Presentation {
        regions: vec![rect(0, 0, 2, 2)],
        dirty: Dirty::Set(true),
        revision: true,
        ..Presentation::none()
    };
    let steps = plan(&presentation);
    assert_eq!(
        steps,
        vec![PresentStep::Bookkeep, PresentStep::Blit(rect(0, 0, 2, 2)),]
    );
}

#[test]
fn regions_precede_notify() {
    let presentation = Presentation {
        regions: vec![rect(0, 0, 2, 2), rect(2, 2, 4, 4)],
        notify: true,
        ..Presentation::none()
    };
    assert_eq!(
        plan(&presentation),
        vec![
            PresentStep::Blit(rect(0, 0, 2, 2)),
            PresentStep::Blit(rect(2, 2, 4, 4)),
            PresentStep::Notify,
        ]
    );
}

#[test]
fn one_notify_per_presentation() {
    for count in 0..3 {
        let presentation = Presentation {
            regions: (0..count).map(|i| rect(i, i, i + 1, i + 1)).collect(),
            notify: true,
            ..Presentation::none()
        };
        let notifies = plan(&presentation)
            .into_iter()
            .filter(|step| *step == PresentStep::Notify)
            .count();
        assert_eq!(notifies, 1, "{count} regions");
    }
}

#[test]
fn silent_region() {
    let presentation = Presentation::regions(vec![rect(0, 0, 2, 2)], false);
    assert_eq!(
        plan(&presentation),
        vec![PresentStep::Blit(rect(0, 0, 2, 2))]
    );
}

#[test]
fn notify_without_regions() {
    let presentation = Presentation::notify();
    assert_eq!(plan(&presentation), vec![PresentStep::Notify]);
}

#[test]
fn rebuild_once() {
    let mut view = view();
    let before_frame = view.frame_revision;
    let presentation = Presentation::rebuild(Rebuild::Sync);
    let steps = present_core(&mut view, &presentation);
    assert_eq!(
        steps
            .iter()
            .filter(|step| **step == PresentStep::Rebuild)
            .count(),
        1
    );
    assert_eq!(
        steps
            .iter()
            .filter(|step| **step == PresentStep::Notify)
            .count(),
        1
    );
    assert_eq!(view.frame_revision, before_frame + 1, "one display rebuild");
}

#[test]
fn deferred_rebuild_plan() {
    let presentation = Presentation::rebuild(Rebuild::Deferred);
    assert_eq!(
        plan(&presentation),
        vec![PresentStep::Rebuild, PresentStep::Notify]
    );
}

#[test]
fn bounded_does_not_rebuild() {
    let presentation = Presentation::regions(vec![rect(0, 0, 2, 2)], false);
    assert!(!plan(&presentation).contains(&PresentStep::Rebuild));
}

#[test]
fn rebuild_always_notifies() {
    let presentation = Presentation {
        rebuild: Rebuild::Sync,
        notify: false,
        ..Presentation::none()
    };
    assert_eq!(
        plan(&presentation),
        vec![PresentStep::Rebuild, PresentStep::Notify]
    );
}

#[test]
fn presenter_applies_bookkeeping() {
    let mut view = view();
    let presentation = Presentation {
        dirty: Dirty::Set(true),
        revision: true,
        ..Presentation::none()
    };
    present_core(&mut view, &presentation);
    assert!(view.session.dirty);
    assert_eq!(view.session.content_revision, 1);
}

#[test]
fn bounded_edit_equivalent() {
    let mut view = view();
    let presentation = view.apply_edit(
        "Edit",
        crate::cxxqt_object::edit::Edit::regions(vec![rect(0, 0, 2, 2)], false),
    );
    assert_eq!(presentation.regions, vec![rect(0, 0, 2, 2)]);
    assert_eq!(presentation.rebuild, Rebuild::None);
    assert!(!presentation.notify);
}

#[test]
fn unbounded_edit_equivalent() {
    let mut view = view();
    let presentation = view.apply_edit("Edit", crate::cxxqt_object::edit::Edit::Full);
    assert!(presentation.regions.is_empty());
    assert_eq!(presentation.rebuild, Rebuild::Sync);
    assert!(presentation.notify);
}

/// Assert the fixed presentation order: bookkeeping first, every blit before
/// the notify, at most one notify, and a rebuild always notifies.
fn assert_steps(steps: &[PresentStep], presentation: &Presentation) {
    let mut saw_blit = false;
    let mut saw_notify = false;
    let mut notifies = 0;
    for step in steps {
        match step {
            PresentStep::Bookkeep => {
                assert!(
                    !saw_blit && !saw_notify,
                    "bookkeeping must precede every blit and notify: {steps:?}"
                );
            }
            PresentStep::Blit(_) => {
                assert!(!saw_notify, "a blit must precede the notify: {steps:?}");
                saw_blit = true;
            }
            PresentStep::Rebuild => {}
            PresentStep::Notify => {
                assert!(!saw_notify, "at most one notify: {steps:?}");
                saw_notify = true;
                notifies += 1;
            }
        }
    }
    if presentation.notify || presentation.rebuild != Rebuild::None {
        assert_eq!(notifies, 1, "the presentation notifies once: {steps:?}");
    } else {
        assert_eq!(notifies, 0, "the presentation stays silent: {steps:?}");
    }
    if presentation.rebuild != Rebuild::None {
        assert_eq!(notifies, 1, "a rebuild always notifies: {steps:?}");
    }
}

#[test]
fn one_value_for_every_producer() {
    let mut edit_view = view();
    let edit = edit_view.apply_edit("Edit", crate::cxxqt_object::edit::Edit::Meta);
    assert_eq!(
        edit.declaration,
        Some(crate::cxxqt_object::edit::Edit::Meta)
    );
    let steps = present_core(&mut edit_view, &edit);
    assert_steps(&steps, &edit);

    let mut cancel_view = view();
    cancel_view
        .session
        .live
        .set_mode_preview(Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight));
    let cancel = cancel_view.cancel_active_sessions();
    assert!(cancel.has_display_change());
    let steps = present_core(&mut cancel_view, &cancel);
    assert_steps(&steps, &cancel);

    let mut restore_view = view();
    let restore = restore_view.restore_presentation(Restored::Everywhere, false);
    assert!(restore.notify);
    let steps = present_core(&mut restore_view, &restore);
    assert_steps(&steps, &restore);

    let mut filter_view = view();
    let filter = crate::cxxqt_object::filter_tools::jobs::test_commit_finish(&mut filter_view)
        .expect("a filter commit presentation");
    let steps = present_core(&mut filter_view, &filter);
    assert_steps(&steps, &filter);

    let mut adjust_view = view();
    let adjust = crate::cxxqt_object::image_adjust::jobs::test_preview_poll(&mut adjust_view)
        .expect("an adjust preview presentation");
    let steps = present_core(&mut adjust_view, &adjust);
    assert_steps(&steps, &adjust);
}

#[test]
fn every_rebuild_and_region_shape_orders_the_steps() {
    for rebuild in [Rebuild::None, Rebuild::Sync, Rebuild::Deferred] {
        for regions in 0..4 {
            for notify in [false, true] {
                let presentation = Presentation {
                    regions: (0..regions).map(|i| rect(i, i, i + 1, i + 1)).collect(),
                    rebuild,
                    notify,
                    dirty: Dirty::Set(true),
                    revision: true,
                    declaration: None,
                };
                let steps = plan(&presentation);
                assert_steps(&steps, &presentation);
            }
        }
    }
}

#[test]
fn move_commit_keeps_revision() {
    let mut view = view();
    let presentation = view.apply_edit_move("Move", crate::cxxqt_object::edit::Edit::Full);
    assert!(!presentation.revision);
    present_core(&mut view, &presentation);
    assert_eq!(view.session.content_revision, 0);
}

#[test]
fn restore_derives_dirty() {
    let mut view = view();
    let mut target = LayerTarget::only("0");
    view.layer_target = target.clone();
    assert!(view.session.record(&target, "First"));
    view.session.mark_saved();
    assert!(view.session.record(&target, "Second"));
    assert!(view.session.derived_dirty());

    let restored = view
        .session
        .restore(&mut target, History::undo_live, History::undo)
        .expect("a state to undo");
    assert!(!view.session.derived_dirty());
    let presentation = view.restore_presentation(restored, view.session.derived_dirty());
    assert_eq!(presentation.dirty, Dirty::Set(false));
    assert!(presentation.revision);
    present_core(&mut view, &presentation);
    assert!(
        !view.session.dirty,
        "the presenter applies the derived clean value"
    );

    let restored = view
        .session
        .restore(&mut target, History::redo_live, History::redo)
        .expect("a state to redo");
    let presentation = view.restore_presentation(restored, view.session.derived_dirty());
    present_core(&mut view, &presentation);
    assert!(view.session.dirty, "redo leaves the saved state");
}
