//! Whole-document commands as cancellable jobs (#1278): Image Size, Canvas
//! Size, Rotate/Flip, mode and depth conversion, Merge and Flatten.
//!
//! The shell arms the view around the command's bridge call. An armed command
//! on a large document clones the document (its planes are shared), runs the
//! engine on a worker, and leaves the job for the shell's progress window to
//! land; the document and history change only when it lands. Any other call
//! (the API, a replay, a small document) runs at once, as before.

use super::super::edit::Edit;
use super::super::qobject::PictureView;
use super::super::{qobject, PictureViewRust};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::job::{self, JobProgress};
use pictura_core::Document;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// The document size from which an armed command leaves the GUI thread.
pub(in crate::cxxqt_object) const DOCUMENT_JOB_PIXELS: u64 = 4_000_000;

/// How a command's progress window reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) enum JobBar {
    /// The engine counts its rows, layers or bands.
    Counted,
    /// The engine makes several passes it cannot weigh; the bar is busy.
    Busy,
}

/// What an applied command reports back: a merged layer's path and the count
/// of layers it replaced, or a fill's changed rect (#1300); empty for the
/// other commands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct Outcome {
    pub(in crate::cxxqt_object) path: String,
    pub(in crate::cxxqt_object) count: usize,
    pub(in crate::cxxqt_object) rect: Option<pictura_core::PsdRect>,
}

/// What lands a finished engine: its outcome in, the history edit out.
type Land = Box<dyn FnOnce(&mut PictureViewRust, &Outcome) -> Edit>;

/// A whole-document engine running on a worker.
pub(in crate::cxxqt_object) struct DocumentJob {
    label: String,
    bar: JobBar,
    progress: Arc<JobProgress>,
    worker: Option<JoinHandle<Option<(Document, Outcome)>>>,
    land: Option<Land>,
}

impl Drop for DocumentJob {
    fn drop(&mut self) {
        self.progress.cancel();
    }
}

/// The view's document-job state.
#[derive(Default)]
pub(in crate::cxxqt_object) struct DocumentJobs {
    job: Option<DocumentJob>,
    /// Set by the shell around a command it can show a progress window for.
    armed: bool,
    /// Whether the last command applied (at once, or when its job landed).
    applied: bool,
    /// The last applied command's outcome.
    result: Outcome,
    /// Test seam: hold a finished engine for this many milliseconds.
    pub(in crate::cxxqt_object) delay_for_test: u64,
}

impl PictureViewRust {
    /// Whether an armed command on this document should run as a job.
    pub(in crate::cxxqt_object) fn document_job_wanted(&self) -> bool {
        self.document_jobs.armed
            && self.session.doc.as_ref().is_some_and(|doc| {
                u64::from(doc.width) * u64::from(doc.height) >= DOCUMENT_JOB_PIXELS
            })
    }

    /// Whether a document job is running or waiting to land.
    pub(in crate::cxxqt_object) fn document_job_pending(&self) -> bool {
        self.document_jobs.job.is_some()
    }

    /// The last applied command's outcome, when it applied.
    pub(in crate::cxxqt_object) fn last_outcome(&self) -> Option<&Outcome> {
        self.document_jobs
            .applied
            .then_some(&self.document_jobs.result)
    }

    /// Drop a document job: its document is going away.
    pub(in crate::cxxqt_object) fn drop_document_job(&mut self) {
        self.document_jobs.job = None;
    }
}

impl qobject::PictureView {
    /// Run a whole-document command: `op` edits the document and returns its
    /// outcome, `None` for a refusal that leaves it unchanged; `land`
    /// finishes the view state on success and names the history edit. On an
    /// armed view and a large document `op` runs on a worker and this returns
    /// true once it has started; otherwise it runs at once and this returns
    /// whether it applied.
    pub(in crate::cxxqt_object) fn document_op(
        mut self: Pin<&mut Self>,
        label: &str,
        bar: JobBar,
        op: impl FnOnce(&mut Document) -> Option<Outcome> + Send + 'static,
        land: impl FnOnce(&mut PictureViewRust, &Outcome) -> Edit + 'static,
    ) -> bool {
        if self.rust().document_job_wanted() {
            return self
                .as_mut()
                .rust_mut()
                .start_document_job(label, bar, op, Box::new(land));
        }
        let mut outcome = None;
        self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let Some(done) = op(doc) else {
                return Edit::None;
            };
            let edit = land(rust, &done);
            outcome = Some(done);
            edit
        });
        self.as_mut().rust_mut().note_document_op(outcome)
    }

    /// Land a finished document job: one history state on success, nothing
    /// after a cancel or a refusal.
    pub(in crate::cxxqt_object) fn document_job_finish(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().document_job_pending() {
            return self.rust().document_jobs.applied;
        }
        let Some(done) = self.as_mut().rust_mut().take_document_job() else {
            return self.as_mut().rust_mut().note_document_op(None);
        };
        let FinishedJob {
            label,
            doc,
            outcome,
            land,
        } = done;
        self.as_mut().edit(&label, |rust| {
            rust.session.doc = Some(doc);
            land(rust, &outcome)
        });
        self.as_mut().rust_mut().note_document_op(Some(outcome))
    }
}

/// A document job whose engine finished: what its landing installs.
struct FinishedJob {
    label: String,
    doc: Document,
    outcome: Outcome,
    land: Land,
}

impl PictureViewRust {
    fn start_document_job(
        &mut self,
        label: &str,
        bar: JobBar,
        op: impl FnOnce(&mut Document) -> Option<Outcome> + Send + 'static,
        land: Land,
    ) -> bool {
        let Some(mut doc) = self.session.doc.clone() else {
            return false;
        };
        let progress = JobProgress::new();
        let delay = self.document_jobs.delay_for_test;
        let worker = {
            let progress = progress.clone();
            std::thread::spawn(move || {
                job::run(&progress, || {
                    let outcome = op(&mut doc);
                    hold_for_test(&progress, delay);
                    outcome
                })
                .ok()
                .flatten()
                .map(|outcome| (doc, outcome))
            })
        };
        self.document_jobs.job = Some(DocumentJob {
            label: label.to_string(),
            bar,
            progress,
            worker: Some(worker),
            land: Some(land),
        });
        true
    }

    /// Join the document job; `None` after a cancel or a refusal, which
    /// leave the document and history as they were.
    fn take_document_job(&mut self) -> Option<FinishedJob> {
        let mut job = self.document_jobs.job.take()?;
        if job.progress.cancelled() {
            // ponytail: an engine that cannot stop early (a tone map) finishes
            // on its detached worker and its result is dropped.
            return None;
        }
        let (doc, outcome) = job.worker.take()?.join().ok().flatten()?;
        Some(FinishedJob {
            label: job.label.clone(),
            doc,
            outcome,
            land: job.land.take()?,
        })
    }

    /// Record whether a command applied, for the shell's job window; returns
    /// it.
    pub(in crate::cxxqt_object) fn note_document_op(&mut self, outcome: Option<Outcome>) -> bool {
        let jobs = &mut self.document_jobs;
        jobs.applied = outcome.is_some();
        jobs.result = outcome.unwrap_or_default();
        jobs.applied
    }
}

fn hold_for_test(progress: &JobProgress, ms: u64) {
    let held = std::time::Instant::now();
    while held.elapsed() < Duration::from_millis(ms) && !progress.cancelled() {
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub(in crate::cxxqt_object) fn document_job_arm(mut view: Pin<&mut PictureView>, armed: bool) {
    let mut rust = view.as_mut().rust_mut();
    rust.document_jobs.armed = armed;
    if armed {
        rust.document_jobs.applied = false;
        rust.document_jobs.result = Outcome::default();
    }
}

pub(in crate::cxxqt_object) fn document_job_pending(view: &PictureView) -> bool {
    view.rust().document_job_pending()
}

pub(in crate::cxxqt_object) fn document_job_running(view: &PictureView) -> bool {
    view.rust().document_jobs.job.as_ref().is_some_and(|job| {
        !job.progress.cancelled() && job.worker.as_ref().is_some_and(|w| !w.is_finished())
    })
}

/// 0 to 1000, or -1 for a busy bar.
pub(in crate::cxxqt_object) fn document_job_progress(view: &PictureView) -> i32 {
    match view.rust().document_jobs.job.as_ref() {
        Some(job) if job.bar == JobBar::Busy => -1,
        Some(job) => job.progress.permille() as i32,
        None => 0,
    }
}

pub(in crate::cxxqt_object) fn document_job_cancel(view: Pin<&mut PictureView>) {
    if let Some(job) = view.rust().document_jobs.job.as_ref() {
        job.progress.cancel();
    }
}

pub(in crate::cxxqt_object) fn document_job_finish(view: Pin<&mut PictureView>) -> bool {
    view.document_job_finish()
}

pub(in crate::cxxqt_object) fn document_op_result(view: &PictureView) -> QString {
    QString::from(view.rust().document_jobs.result.path.as_str())
}

pub(in crate::cxxqt_object) fn document_job_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().document_jobs.delay_for_test = ms;
}

#[cfg(test)]
#[path = "document_job_tests.rs"]
mod tests;
