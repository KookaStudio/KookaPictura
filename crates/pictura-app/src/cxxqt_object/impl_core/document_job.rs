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
use super::save_jobs::ffi::DocumentJobResult;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::job::{self, JobProgress};
use pictura_core::Document;
use pictura_error::{guard, Error, ErrorCode, OpStatus};
use std::panic::AssertUnwindSafe;
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

/// What runs after a landed command has been applied and captured. The edit
/// transaction clears the Fade record before it applies, so a command that
/// installs one (the Magic Eraser) does it here.
type After = Box<dyn FnOnce(&mut PictureViewRust)>;

type WorkerResult = Result<Option<(Document, Outcome)>, Error>;
type SourceState = (u64, u64, u64, Option<u64>);

/// A whole-document engine running on a worker.
pub(in crate::cxxqt_object) struct DocumentJob {
    label: String,
    bar: JobBar,
    progress: Arc<JobProgress>,
    worker: Option<JoinHandle<WorkerResult>>,
    land: Option<Land>,
    after: Option<After>,
    source_state: SourceState,
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
    status: OpStatus,
    attempt: u64,
    panic_for_test: bool,
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
        if self.document_jobs.job.take().is_some() {
            self.note_document_failure(ErrorCode::Cancelled);
        }
    }

    fn document_source_state(&self) -> SourceState {
        (
            self.session.document_generation,
            self.session.content_revision,
            self.non_preview_revision,
            self.session.history.current_id(),
        )
    }

    fn begin_document_attempt(&mut self) {
        self.drop_document_job();
        self.document_jobs.attempt = self.document_jobs.attempt.wrapping_add(1);
        self.note_document_op(None);
    }

    fn note_document_failure(&mut self, code: ErrorCode) -> bool {
        self.note_document_op(None);
        self.document_jobs.status = code.status();
        false
    }
}

impl qobject::PictureView {
    pub(in crate::cxxqt_object) fn start_source_job(
        mut self: Pin<&mut Self>,
        label: &str,
        op: impl FnOnce(&mut Document) -> Option<Outcome> + Send + 'static,
    ) -> bool {
        if self.rust().document_job_pending() || self.rust().session.live.transform().is_some() {
            return self.as_mut().rust_mut().note_document_op(None);
        }
        let attempt = self.rust().document_jobs.attempt;
        self.as_mut().settle_pending_composite();
        if self.rust().document_jobs.attempt != attempt {
            return false;
        }
        let mut rust = self.as_mut().rust_mut();
        rust.begin_document_attempt();
        rust.start_document_job(
            label,
            JobBar::Busy,
            move |doc| Ok(op(doc)),
            Box::new(|_, outcome| {
                if outcome.count == 0 {
                    Edit::None
                } else {
                    Edit::Full
                }
            }),
            None,
        )
    }

    /// Run a whole-document command: `op` edits the document and returns its
    /// outcome, no edit, or a typed refusal; `land`
    /// finishes the view state on success and names the history edit. On an
    /// armed view and a large document `op` runs on a worker and this returns
    /// true once it has started; otherwise it runs at once and this returns
    /// whether it applied.
    pub(in crate::cxxqt_object) fn document_op(
        self: Pin<&mut Self>,
        label: &str,
        bar: JobBar,
        op: impl FnOnce(&mut Document) -> Result<Option<Outcome>, Error> + Send + 'static,
        land: impl FnOnce(&mut PictureViewRust, &Outcome) -> Edit + 'static,
    ) -> bool {
        self.document_op_with(label, bar, op, land, None)
    }

    /// [`Self::document_op`] with `after`, run once the edit has been applied
    /// and captured (a Magic Eraser's Fade record).
    pub(in crate::cxxqt_object) fn document_op_after(
        self: Pin<&mut Self>,
        label: &str,
        bar: JobBar,
        op: impl FnOnce(&mut Document) -> Result<Option<Outcome>, Error> + Send + 'static,
        land: impl FnOnce(&mut PictureViewRust, &Outcome) -> Edit + 'static,
        after: impl FnOnce(&mut PictureViewRust) + 'static,
    ) -> bool {
        self.document_op_with(label, bar, op, land, Some(Box::new(after)))
    }

    fn document_op_with(
        mut self: Pin<&mut Self>,
        label: &str,
        bar: JobBar,
        op: impl FnOnce(&mut Document) -> Result<Option<Outcome>, Error> + Send + 'static,
        land: impl FnOnce(&mut PictureViewRust, &Outcome) -> Edit + 'static,
        after: Option<After>,
    ) -> bool {
        if self.rust().session.read_only() {
            return self.document_preflight_refusal(ErrorCode::Capability);
        }
        let attempt = self.rust().document_jobs.attempt;
        self.as_mut().settle_pending_composite();
        if self.rust().document_jobs.attempt != attempt {
            return false;
        }
        if self.rust().document_job_wanted() {
            return self.as_mut().rust_mut().start_document_job(
                label,
                bar,
                op,
                Box::new(land),
                after,
            );
        }

        if !self.rust().document_jobs.armed {
            self.as_mut().rust_mut().begin_document_attempt();
        }
        let attempt = self.rust().document_jobs.attempt;
        let mut staged = Ok(None);
        self.as_mut().edit(label, |rust| {
            if rust.document_jobs.attempt != attempt {
                return Edit::None;
            }
            let Some(mut doc) = rust.session.doc.snapshot() else {
                staged = Err(Error::new(ErrorCode::InvalidArg, "no editable document"));
                return Edit::None;
            };
            let panic_for_test = std::mem::take(&mut rust.document_jobs.panic_for_test);
            // The engine can mutate only this snapshot until it returns successfully.
            staged = guard(AssertUnwindSafe(|| {
                run_document_engine(&mut doc, op, panic_for_test)
            }));
            if let Ok(Some(done)) = &staged {
                rust.session.doc.set(Some(doc));
                land(rust, done)
            } else {
                Edit::None
            }
        });
        if self.rust().document_jobs.attempt != attempt {
            return false;
        }
        if staged.as_ref().is_ok_and(Option::is_some) {
            if let Some(after) = after {
                after(&mut self.as_mut().rust_mut());
            }
        }
        if self.rust().document_jobs.attempt != attempt {
            return false;
        }
        let mut rust = self.as_mut().rust_mut();
        match staged {
            Ok(outcome) => rust.note_document_op(outcome),
            Err(error) => rust.note_document_failure(error.code()),
        }
    }

    pub(in crate::cxxqt_object) fn document_preflight_refusal(
        mut self: Pin<&mut Self>,
        code: ErrorCode,
    ) -> bool {
        let mut rust = self.as_mut().rust_mut();
        if !rust.document_jobs.armed {
            rust.begin_document_attempt();
        }
        rust.note_document_failure(code)
    }

    /// Land a finished document job: one history state on success, nothing
    /// after a cancel or a refusal.
    pub(in crate::cxxqt_object) fn document_job_finish(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().document_job_pending() {
            return self.rust().document_jobs.applied;
        }
        let done = match self.as_mut().rust_mut().take_document_job() {
            Ok(Some(done)) => done,
            Ok(None) => return self.as_mut().rust_mut().note_document_op(None),
            Err(code) => return self.as_mut().rust_mut().note_document_failure(code),
        };
        let FinishedJob {
            label,
            doc,
            outcome,
            land,
            after,
            source_state,
            attempt,
        } = done;
        let mut landed = false;
        self.as_mut().edit(&label, |rust| {
            if rust.document_jobs.attempt != attempt || rust.document_source_state() != source_state
            {
                return Edit::None;
            }
            rust.session.doc.set(Some(doc));
            landed = true;
            land(rust, &outcome)
        });
        if !landed {
            let mut rust = self.as_mut().rust_mut();
            if rust.document_jobs.attempt == attempt {
                rust.note_document_failure(ErrorCode::Cancelled);
            }
            return false;
        }
        if self.rust().document_jobs.attempt != attempt {
            return false;
        }
        if let Some(after) = after {
            after(&mut self.as_mut().rust_mut());
        }
        if self.rust().document_jobs.attempt != attempt {
            return false;
        }
        self.as_mut().rust_mut().note_document_op(Some(outcome))
    }
}

/// A document job whose engine finished: what its landing installs.
struct FinishedJob {
    label: String,
    doc: Document,
    outcome: Outcome,
    land: Land,
    after: Option<After>,
    source_state: SourceState,
    attempt: u64,
}

impl PictureViewRust {
    fn start_document_job(
        &mut self,
        label: &str,
        bar: JobBar,
        op: impl FnOnce(&mut Document) -> Result<Option<Outcome>, Error> + Send + 'static,
        land: Land,
        after: Option<After>,
    ) -> bool {
        let Some(mut doc) = self.session.doc.snapshot() else {
            return self.note_document_failure(ErrorCode::InvalidArg);
        };
        let progress = JobProgress::new();
        let delay = self.document_jobs.delay_for_test;
        let panic_for_test = std::mem::take(&mut self.document_jobs.panic_for_test);
        let worker = {
            let progress = progress.clone();
            std::thread::Builder::new().spawn(move || {
                let outcome = job::run(&progress, || {
                    // The snapshot and progress token are exclusive to this stage.
                    let outcome = guard(AssertUnwindSafe(|| {
                        run_document_engine(&mut doc, op, panic_for_test)
                    }));
                    hold_for_test(&progress, delay);
                    outcome
                })
                .map_err(|_| Error::new(ErrorCode::Cancelled, "the operation was cancelled"))??;
                Ok(outcome.map(|outcome| (doc, outcome)))
            })
        };
        let Ok(worker) = worker else {
            return self.note_document_failure(ErrorCode::Host);
        };
        self.document_jobs.job = Some(DocumentJob {
            label: label.to_string(),
            bar,
            progress,
            worker: Some(worker),
            land: Some(land),
            after,
            source_state: self.document_source_state(),
        });
        true
    }

    fn take_document_job(&mut self) -> Result<Option<FinishedJob>, ErrorCode> {
        let mut job = self.document_jobs.job.take().ok_or(ErrorCode::Host)?;
        if job.source_state != self.document_source_state() || job.progress.cancelled() {
            // An engine that cannot stop early finishes on its detached worker.
            return Err(ErrorCode::Cancelled);
        }
        let result = job
            .worker
            .take()
            .ok_or(ErrorCode::Host)?
            .join()
            .map_err(|payload| {
                std::mem::forget(payload);
                ErrorCode::Panic
            })?;
        let Some((doc, outcome)) = result.map_err(|error| error.code())? else {
            return Ok(None);
        };
        Ok(Some(FinishedJob {
            label: job.label.clone(),
            doc,
            outcome,
            land: job.land.take().ok_or(ErrorCode::Host)?,
            after: job.after.take(),
            source_state: job.source_state,
            attempt: self.document_jobs.attempt,
        }))
    }

    /// Record whether a command applied, for the shell's job window; returns
    /// it.
    pub(in crate::cxxqt_object) fn note_document_op(&mut self, outcome: Option<Outcome>) -> bool {
        let jobs = &mut self.document_jobs;
        jobs.status = ErrorCode::Ok.status();
        jobs.applied = outcome.is_some();
        jobs.result = outcome.unwrap_or_default();
        jobs.applied
    }
}

pub(in crate::cxxqt_object) fn document_engine_error(error: pictura_ops::OpsError) -> Error {
    let code = match &error {
        pictura_ops::OpsError::Unsupported(_) => ErrorCode::Capability,
        pictura_ops::OpsError::InvalidParams(_) => ErrorCode::InvalidArg,
        pictura_ops::OpsError::Memory(_) => ErrorCode::Host,
    };
    Error::with_source(code, error.to_string(), error)
}

fn run_document_engine(
    doc: &mut Document,
    op: impl FnOnce(&mut Document) -> Result<Option<Outcome>, Error>,
    panic_for_test: bool,
) -> Result<Option<Outcome>, Error> {
    let outcome = op(doc)?;
    if panic_for_test {
        panic!("injected panic after staging a document operation");
    }
    Ok(outcome)
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
        rust.begin_document_attempt();
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

pub(in crate::cxxqt_object) fn document_job_status(view: &PictureView) -> u32 {
    view.rust().document_jobs.status.raw()
}

pub(in crate::cxxqt_object) fn document_job_attempt(view: &PictureView) -> u64 {
    view.rust().document_jobs.attempt
}

pub(in crate::cxxqt_object) fn document_job_finish_for(
    mut view: Pin<&mut PictureView>,
    attempt: u64,
) -> DocumentJobResult {
    if document_job_attempt(view.as_ref().get_ref()) != attempt {
        return DocumentJobResult {
            code: ErrorCode::Cancelled.status().raw(),
            applied: false,
        };
    }
    let applied = view.as_mut().document_job_finish();
    if document_job_attempt(view.as_ref().get_ref()) != attempt {
        return DocumentJobResult {
            code: ErrorCode::Cancelled.status().raw(),
            applied: false,
        };
    }
    DocumentJobResult {
        code: document_job_status(view.as_ref().get_ref()),
        applied,
    }
}

pub(in crate::cxxqt_object) fn document_job_panic_for_test(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().document_jobs.panic_for_test = true;
}

pub(in crate::cxxqt_object) fn document_job_edit_source_for_test(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    let Some(layer) = rust
        .session
        .doc
        .as_mut()
        .and_then(|doc| doc.layers.last_mut())
    else {
        return;
    };
    layer.name = "Edited while worker ran".to_owned();
    rust.non_preview_revision = rust.non_preview_revision.wrapping_add(1);
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
