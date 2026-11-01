use super::*;

use super::super::edit::Edit;
use super::super::helpers::{as_str_slice, list_of_strings};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use pictura_render::{
    apply_auto_blend, can_auto_blend, plan_auto_blend, AutoBlendCancelled, AutoBlendOptions,
    AutoBlendPlan, BlendObjective,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

type Outcome = Result<AutoBlendPlan, AutoBlendCancelled>;

const STALE_MESSAGE: &str = "The document changed while Auto-Blend Layers was running.";
const FAILED_MESSAGE: &str = "Auto-Blend Layers could not finish.";

struct Run {
    worker: Option<JoinHandle<Outcome>>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicU32>,
    revision: u64,
}

impl Drop for Run {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// The running Auto-Blend worker and the last run's error.
#[derive(Default)]
pub(in crate::cxxqt_object) struct AutoBlendJob {
    run: Option<Run>,
    error: Option<String>,
    test_delay: Duration,
}

impl AutoBlendJob {
    /// Drop any run, leaving its worker to stop on its own.
    pub(in crate::cxxqt_object) fn cancel(&mut self) {
        self.run = None;
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
    if view.rust().auto_blend.run.is_some() || !auto_blend_can(&view, paths) {
        return false;
    }
    let mut rust = view.rust_mut();
    let Some(doc) = rust.session.doc.clone() else {
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
    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicU32::new(0));
    let delay = rust.auto_blend.test_delay;
    let worker = {
        let (cancel, progress) = (cancel.clone(), progress.clone());
        std::thread::spawn(move || {
            let held = Instant::now();
            while held.elapsed() < delay && !cancel.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(1));
            }
            plan_auto_blend(&doc, &as_str_slice(&owned), options, &cancel, &progress)
        })
    };
    let revision = rust.session.content_revision;
    rust.auto_blend.error = None;
    rust.auto_blend.run = Some(Run {
        worker: Some(worker),
        cancel,
        progress,
        revision,
    });
    true
}

pub(super) fn auto_blend_running(view: &PictureView) -> bool {
    view.rust()
        .auto_blend
        .run
        .as_ref()
        .is_some_and(|run| run.worker.as_ref().is_some_and(|w| !w.is_finished()))
}

pub(super) fn auto_blend_progress(view: &PictureView) -> i32 {
    view.rust()
        .auto_blend
        .run
        .as_ref()
        .map_or(0, |run| run.progress.load(Ordering::Relaxed) as i32)
}

pub(super) fn auto_blend_cancel(view: Pin<&mut PictureView>) {
    if let Some(run) = view.rust().auto_blend.run.as_ref() {
        run.cancel.store(true, Ordering::Relaxed);
    }
}

/// The plan a finished run may commit, or the error it leaves behind. A
/// cancelled run yields neither.
fn landed(mut run: Run, revision: u64) -> Result<Option<AutoBlendPlan>, &'static str> {
    let outcome = run.worker.take().map(JoinHandle::join);
    match outcome {
        Some(Ok(Ok(plan))) if run.revision == revision => Ok(Some(plan)),
        Some(Ok(Ok(_))) => Err(STALE_MESSAGE),
        Some(Ok(Err(AutoBlendCancelled))) => Ok(None),
        _ => Err(FAILED_MESSAGE),
    }
}

pub(super) fn auto_blend_finish(mut view: Pin<&mut PictureView>) -> QStringList {
    let mut paths = Vec::new();
    let (run, revision) = {
        let mut rust = view.as_mut().rust_mut();
        (rust.auto_blend.run.take(), rust.session.content_revision)
    };
    let Some(run) = run else {
        return QStringList::default();
    };
    match landed(run, revision) {
        Err(message) => view.as_mut().rust_mut().auto_blend.error = Some(message.to_string()),
        Ok(None) => {}
        Ok(Some(plan)) => {
            view.as_mut().edit("Auto-Blend Layers", |rust| {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                if plan.layers.is_empty() {
                    return Edit::None;
                }
                paths = apply_auto_blend(doc, plan);
                rust.link_sets.clear();
                rust.solo_snapshot = None;
                rust.mask_edit_target = None;
                Edit::Structural { damage: None }
            });
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
    view.rust_mut().auto_blend.test_delay = Duration::from_millis(ms.max(0) as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(outcome: Outcome, revision: u64) -> Run {
        Run {
            worker: Some(std::thread::spawn(move || outcome)),
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicU32::new(0)),
            revision,
        }
    }

    #[test]
    fn a_stale_plan_is_dropped() {
        let plan = AutoBlendPlan::default();
        assert_eq!(landed(run(Ok(plan.clone()), 4), 5), Err(STALE_MESSAGE));
        assert_eq!(landed(run(Ok(plan.clone()), 5), 5), Ok(Some(plan)));
        assert_eq!(landed(run(Err(AutoBlendCancelled), 5), 5), Ok(None));
    }

    #[test]
    fn dropping_a_run_asks_its_worker_to_stop() {
        let held = run(Ok(AutoBlendPlan::default()), 0);
        let cancel = held.cancel.clone();
        drop(held);
        assert!(cancel.load(Ordering::Relaxed));
    }
}
