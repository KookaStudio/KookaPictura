use super::*;

use super::super::edit::Edit;
use super::super::helpers::{as_str_slice, list_of_strings};
use super::super::prepared_job::{Landed, PreparedJob};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use pictura_render::{
    apply_auto_blend, can_auto_blend, plan_auto_blend, AutoBlendOptions, AutoBlendPlan,
    BlendObjective,
};

const STALE_MESSAGE: &str = "The document changed while Auto-Blend Layers was running.";
const FAILED_MESSAGE: &str = "Auto-Blend Layers could not finish.";
const NO_MERGED_MESSAGE: &str =
    "These layers are too large for a merged layer, so Auto-Blend Layers added masks only.";

/// The running Auto-Blend worker and the last run's error.
#[derive(Default)]
pub(in crate::cxxqt_object) struct AutoBlendJob {
    job: PreparedJob<AutoBlendPlan>,
    error: Option<String>,
}

impl AutoBlendJob {
    /// Drop any run, leaving its worker to stop on its own.
    pub(in crate::cxxqt_object) fn cancel(&mut self) {
        self.job.discard();
    }
}

pub(super) fn auto_blend_can(view: &PictureView, paths: &QStringList) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let owned = list_of_strings(paths);
    !rust.modal_transform_blocks_edit() && can_auto_blend(doc, &as_str_slice(&owned))
}

pub(super) fn auto_blend_start(
    view: Pin<&mut PictureView>,
    paths: &QStringList,
    stack_images: bool,
    seamless: bool,
    fill_transparent: bool,
) -> bool {
    if !auto_blend_can(&view, paths) {
        return false;
    }
    let mut rust = view.rust_mut();
    let Some(doc) = rust.session.doc.snapshot() else {
        return false;
    };
    let options = AutoBlendOptions {
        objective: if stack_images {
            BlendObjective::StackImages
        } else {
            BlendObjective::Panorama
        },
        seamless,
        fill_transparent,
    };
    let owned = list_of_strings(paths);
    let revision = rust.session.content_revision;
    let started = rust
        .auto_blend
        .job
        .start(doc, revision, move |doc, cancel, progress| {
            plan_auto_blend(doc, &as_str_slice(&owned), options, cancel, progress).ok()
        });
    if started {
        rust.auto_blend.error = None;
    }
    started
}

pub(super) fn auto_blend_running(view: &PictureView) -> bool {
    view.rust().auto_blend.job.running()
}

pub(super) fn auto_blend_progress(view: &PictureView) -> i32 {
    view.rust().auto_blend.job.progress()
}

pub(super) fn auto_blend_cancel(view: Pin<&mut PictureView>) {
    view.rust().auto_blend.job.stop();
}

pub(super) fn auto_blend_finish(mut view: Pin<&mut PictureView>) -> QStringList {
    let mut paths = Vec::new();
    let landed = {
        let mut rust = view.as_mut().rust_mut();
        let revision = rust.session.content_revision;
        rust.auto_blend.job.land(revision)
    };
    match landed {
        Landed::Empty | Landed::Cancelled => {}
        Landed::Stale => {
            view.as_mut().rust_mut().auto_blend.error = Some(STALE_MESSAGE.to_string());
        }
        Landed::Failed => {
            view.as_mut().rust_mut().auto_blend.error = Some(FAILED_MESSAGE.to_string());
        }
        Landed::Ready(plan) => {
            let masks_only = !plan.layers.is_empty() && plan.merged.is_none();
            view.as_mut().edit("Auto-Blend Layers", |rust| {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                if plan.layers.is_empty() {
                    return Edit::None;
                }
                paths = apply_auto_blend(doc, plan);
                rust.session.link_sets.clear();
                rust.session.link_remap = None;
                rust.session.solo_snapshot = None;
                rust.mask_edit_target = None;
                Edit::Structural { damage: None }
            });
            if masks_only {
                view.as_mut().rust_mut().auto_blend.error = Some(NO_MERGED_MESSAGE.to_string());
            }
        }
    }
    let mut list = QList::<QString>::default();
    for path in &paths {
        list.append(QString::from(path));
    }
    QStringList::from(&list)
}

pub(super) fn auto_blend_error(view: &PictureView) -> QString {
    QString::from(view.rust().auto_blend.error.as_deref().unwrap_or_default())
}

pub(super) fn auto_blend_set_delay_for_test(view: Pin<&mut PictureView>, ms: i32) {
    view.rust_mut().auto_blend.job.set_delay_for_test(ms);
}
