//! Progress and cancellation for long engine operations run as an app job.
//!
//! The caller wraps the engine call in [`run`], which installs the token for the
//! calling thread. An engine loop reads [`current`] once, on that thread, before
//! it fans out, and reports per row through the returned token. With no job
//! installed, `current` is `None` and the loop is unchanged.
//!
//! ponytail: the token is thread-scoped, so an engine that reaches a reporting
//! loop from inside its own rayon task must capture [`current`] first and
//! re-install it there with [`scope`].

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// A job's cancel flag and its 0–1000 progress.
///
/// The work is split into equal stages (one per layer, say). A layer-level
/// engine call opens a stage with [`JobProgress::begin_stage`], declaring its
/// units (output rows), and its loops [`JobProgress::advance`] through them.
#[derive(Debug, Default)]
pub struct JobProgress {
    cancel: AtomicBool,
    stages: AtomicU64,
    stage: AtomicU64,
    opened: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
    shown: AtomicU32,
}

/// The job was cancelled before its work completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

impl JobProgress {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Split the work into `n` equal stages; one by default.
    pub fn set_stages(&self, n: u64) {
        self.stages.store(n.max(1), Ordering::Relaxed);
    }

    /// Open the next stage, of `units` work. Call it on the job's thread
    /// before the stage's loops fan out.
    pub fn begin_stage(&self, units: u64) {
        if self.opened.swap(true, Ordering::Relaxed) {
            self.stage.fetch_add(1, Ordering::Relaxed);
        }
        self.done.store(0, Ordering::Relaxed);
        self.total.store(units, Ordering::Relaxed);
    }

    /// Record `units` of the current stage's work as finished.
    pub fn advance(&self, units: u64) {
        let done = self.done.fetch_add(units, Ordering::Relaxed) + units;
        let total = self.total.load(Ordering::Relaxed).max(1);
        let stages = self.stages.load(Ordering::Relaxed).max(1);
        let stage = self.stage.load(Ordering::Relaxed).min(stages - 1);
        let permille = (stage * 1000 + done.min(total) * 1000 / total) / stages;
        // Only `finish` reports 1000; an overrun or a short stage never lowers it.
        self.shown
            .fetch_max((permille as u32).min(999), Ordering::Relaxed);
    }

    /// Mark the work complete.
    pub fn finish(&self) {
        self.shown.store(1000, Ordering::Relaxed);
    }

    /// Progress so far, 0 to 1000; it never decreases.
    pub fn permille(&self) -> u32 {
        self.shown.load(Ordering::Relaxed)
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<JobProgress>>> = const { RefCell::new(None) };
}

/// The job installed on this thread, if any.
pub fn current() -> Option<Arc<JobProgress>> {
    CURRENT.with(|c| c.borrow().clone())
}

/// Run `f` with `job` installed on this thread, restoring the previous one.
pub fn scope<T>(job: &Arc<JobProgress>, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<Arc<JobProgress>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|c| *c.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(CURRENT.with(|c| c.borrow_mut().replace(job.clone())));
    f()
}

/// Run `f` under `job`; `Err(Cancelled)` when the job was cancelled, whose
/// result may be incomplete and must be dropped.
pub fn run<T>(job: &Arc<JobProgress>, f: impl FnOnce() -> T) -> Result<T, Cancelled> {
    let value = scope(job, f);
    if job.cancelled() {
        return Err(Cancelled);
    }
    job.finish();
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_never_decreases() {
        let job = JobProgress::new();
        job.set_stages(2);
        let mut seen = Vec::new();
        job.begin_stage(10);
        for _ in 0..12 {
            job.advance(1);
            seen.push(job.permille());
        }
        assert_eq!(*seen.last().unwrap(), 500, "an overrun stops at its stage");
        job.begin_stage(4);
        for _ in 0..4 {
            job.advance(1);
            seen.push(job.permille());
        }
        // A third, undeclared stage holds the value.
        job.begin_stage(8);
        job.advance(1);
        seen.push(job.permille());
        assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
        assert_eq!(*seen.last().unwrap(), 999, "only finish reports 1000");
        assert_eq!(run(&job, || ()), Ok(()));
        assert_eq!(job.permille(), 1000);
    }

    #[test]
    fn a_cancelled_run_reports_cancelled() {
        let job = JobProgress::new();
        let seen = run(&job, || {
            let inner = current().expect("installed");
            inner.cancel();
            7
        });
        assert_eq!(seen, Err(Cancelled));
        assert!(job.permille() < 1000);
        assert!(current().is_none(), "the scope is restored");
    }

    #[test]
    fn scopes_nest_and_restore() {
        let (outer, inner) = (JobProgress::new(), JobProgress::new());
        scope(&outer, || {
            scope(&inner, || assert!(Arc::ptr_eq(&current().unwrap(), &inner)));
            assert!(Arc::ptr_eq(&current().unwrap(), &outer));
        });
        assert!(current().is_none());
    }
}
