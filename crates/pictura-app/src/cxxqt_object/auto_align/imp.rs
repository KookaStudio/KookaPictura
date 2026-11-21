use super::*;

use super::super::edit::Edit;
use super::super::helpers::{as_str_slice, list_of_strings};
use super::super::prepared_job::{Landed, PreparedJob};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use pictura_render::{
    apply_auto_align, can_auto_align, plan_auto_align, AutoAlignOptions, AutoAlignPlan,
    AutoAlignProjection, LayerWarp,
};

const STALE_MESSAGE: &str = "The document changed while Auto-Align Layers was running.";
const FAILED_MESSAGE: &str = "Auto-Align Layers could not finish.";

/// The running Auto-Align worker and the last run's notice.
#[derive(Default)]
pub(in crate::cxxqt_object) struct AutoAlignJob {
    job: PreparedJob<AutoAlignPlan>,
    notice: Option<String>,
}

impl AutoAlignJob {
    /// Drop any run, leaving its worker to stop on its own.
    pub(in crate::cxxqt_object) fn cancel(&mut self) {
        self.job.discard();
    }
}

pub(super) fn auto_align_can(view: &PictureView, paths: &QStringList) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let owned = list_of_strings(paths);
    !rust.modal_transform_blocks_edit() && can_auto_align(doc, &as_str_slice(&owned))
}

pub(super) fn auto_align_start(
    view: Pin<&mut PictureView>,
    paths: &QStringList,
    projection: i32,
    vignette: bool,
    geometric: bool,
) -> bool {
    if !auto_align_can(&view, paths) {
        return false;
    }
    let mut rust = view.rust_mut();
    let Some(doc) = rust.session.doc.snapshot() else {
        return false;
    };
    let projection =
        AutoAlignProjection::from_index(projection).unwrap_or(AutoAlignProjection::Auto);
    let lens = if projection == AutoAlignProjection::LensCorrection {
        super::super::filter_tools::lens_correction::auto_params(&doc, vignette, geometric)
    } else {
        None
    };
    let options = AutoAlignOptions {
        projection,
        vignette_removal: vignette,
        geometric_distortion: geometric,
        lens,
    };
    let owned = list_of_strings(paths);
    let revision = rust.session.content_revision;
    let started = rust
        .auto_align
        .job
        .start(doc, revision, move |doc, cancel, progress| {
            plan_auto_align(doc, &as_str_slice(&owned), options, cancel, progress).ok()
        });
    if started {
        rust.auto_align.notice = None;
    }
    started
}

pub(super) fn auto_align_running(view: &PictureView) -> bool {
    view.rust().auto_align.job.running()
}

pub(super) fn auto_align_progress(view: &PictureView) -> i32 {
    view.rust().auto_align.job.progress()
}

pub(super) fn auto_align_cancel(view: Pin<&mut PictureView>) {
    view.rust().auto_align.job.stop();
}

pub(super) fn auto_align_finish(mut view: Pin<&mut PictureView>) -> QStringList {
    let mut paths = Vec::new();
    let landed = {
        let mut rust = view.as_mut().rust_mut();
        let revision = rust.session.content_revision;
        rust.auto_align.job.land(revision)
    };
    match landed {
        Landed::Empty | Landed::Cancelled => {}
        Landed::Stale => {
            view.as_mut().rust_mut().auto_align.notice = Some(STALE_MESSAGE.to_string());
        }
        Landed::Failed => {
            view.as_mut().rust_mut().auto_align.notice = Some(FAILED_MESSAGE.to_string());
        }
        Landed::Ready(plan) => {
            let notice = plan.notice.clone();
            view.as_mut().edit("Auto-Align Layers", |rust| {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                let changes = plan.lens.is_some()
                    || plan
                        .warps
                        .iter()
                        .any(|w| !matches!(w, LayerWarp::Identity { .. }));
                if !changes {
                    return Edit::None;
                }
                paths = apply_auto_align(doc, plan);
                if paths.is_empty() {
                    return Edit::None;
                }
                rust.session.link_sets.clear();
                rust.session.link_remap = None;
                rust.session.solo_snapshot = None;
                rust.mask_edit_target = None;
                Edit::Structural { damage: None }
            });
            if let Some(notice) = notice {
                view.as_mut().rust_mut().auto_align.notice = Some(notice);
            }
        }
    }
    let mut list = QList::<QString>::default();
    for path in &paths {
        list.append(QString::from(path));
    }
    QStringList::from(&list)
}

pub(super) fn auto_align_notice(view: &PictureView) -> QString {
    QString::from(view.rust().auto_align.notice.as_deref().unwrap_or_default())
}

pub(super) fn auto_align_set_delay_for_test(view: Pin<&mut PictureView>, ms: i32) {
    view.rust_mut().auto_align.job.set_delay_for_test(ms);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::document_session::DocumentSession;
    use crate::cxxqt_object::PictureViewRust;
    use pictura_core::{BitDepth, ColorMode, Document};

    fn doc() -> Document {
        Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight)
    }

    #[test]
    fn replacing_the_document_discards_its_run() {
        let mut view = PictureViewRust {
            session: DocumentSession::with_document(doc()),
            ..Default::default()
        };
        view.auto_align.job.set_delay_for_test(30_000);
        assert!(view
            .auto_align
            .job
            .start(doc(), 0, |_, _, _| Some(AutoAlignPlan::default())));

        view.clear_document_scoped_state();

        assert_eq!(view.auto_align.job.land(0), Landed::Empty);
        view.auto_align.job.set_delay_for_test(0);
        assert!(view
            .auto_align
            .job
            .start(doc(), 1, |_, _, _| Some(AutoAlignPlan::default())));
        assert_eq!(
            view.auto_align.job.land(1),
            Landed::Ready(AutoAlignPlan::default())
        );
    }
}
