//! A plan computed on a worker from a document snapshot and landed as one
//! edit (#1399).
//!
//! Auto-Align and Auto-Blend share this lifecycle: hold the document clone and
//! the content revision captured at start, run the plan off the GUI thread with
//! cancellation and progress, then join it once and refuse a result the
//! document no longer supports. The plan's computation and application stay
//! with the caller.

use pictura_core::Document;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// What landing a held run produced.
#[derive(Debug, PartialEq)]
pub(in crate::cxxqt_object) enum Landed<P> {
    /// No run was held, or it already landed.
    Empty,
    /// A plan computed for the document's current revision.
    Ready(P),
    /// The document changed while the worker ran.
    Stale,
    /// The run was asked to stop, or its worker returned no plan.
    Cancelled,
    /// The worker panicked.
    Failed,
}

struct Run<P> {
    worker: Option<JoinHandle<Option<P>>>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicU32>,
    revision: u64,
}

impl<P> Drop for Run<P> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// One held worker and the protocol that lands its plan.
pub(in crate::cxxqt_object) struct PreparedJob<P> {
    run: Option<Run<P>>,
    delay: Duration,
}

impl<P> Default for PreparedJob<P> {
    fn default() -> Self {
        Self {
            run: None,
            delay: Duration::ZERO,
        }
    }
}

impl<P> PreparedJob<P> {
    /// Whether the held worker is still computing.
    pub(in crate::cxxqt_object) fn running(&self) -> bool {
        self.run
            .as_ref()
            .is_some_and(|run| run.worker.as_ref().is_some_and(|w| !w.is_finished()))
    }

    /// The held run's progress, 0 to 1000; zero when no run is held.
    pub(in crate::cxxqt_object) fn progress(&self) -> i32 {
        self.run
            .as_ref()
            .map_or(0, |run| run.progress.load(Ordering::Relaxed) as i32)
    }

    /// Hold the worker for `ms` before its plan starts, so a test can act
    /// mid-run.
    pub(in crate::cxxqt_object) fn set_delay_for_test(&mut self, ms: i32) {
        self.delay = Duration::from_millis(ms.max(0) as u64);
    }

    /// Ask the worker to stop; landing then commits nothing.
    pub(in crate::cxxqt_object) fn stop(&self) {
        if let Some(run) = &self.run {
            run.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Drop the held run, leaving its worker to stop and exit on its own.
    pub(in crate::cxxqt_object) fn discard(&mut self) {
        self.run = None;
    }
}

impl<P: Send + 'static> PreparedJob<P> {
    /// Start a run over `doc`, captured at `revision`. `plan` runs on the
    /// worker and returns `None` when it was cancelled. False when a run is
    /// already held.
    pub(in crate::cxxqt_object) fn start(
        &mut self,
        doc: Document,
        revision: u64,
        plan: impl FnOnce(&Document, &AtomicBool, &AtomicU32) -> Option<P> + Send + 'static,
    ) -> bool {
        if self.run.is_some() {
            return false;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicU32::new(0));
        let delay = self.delay;
        let worker = {
            let (cancel, progress) = (cancel.clone(), progress.clone());
            std::thread::spawn(move || {
                let held = Instant::now();
                while held.elapsed() < delay && !cancel.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                plan(&doc, &cancel, &progress)
            })
        };
        self.run = Some(Run {
            worker: Some(worker),
            cancel,
            progress,
            revision,
        });
        true
    }

    /// Join the held worker once and classify its plan for `revision`.
    pub(in crate::cxxqt_object) fn land(&mut self, revision: u64) -> Landed<P> {
        let Some(mut run) = self.run.take() else {
            return Landed::Empty;
        };
        let Some(worker) = run.worker.take() else {
            // Unreachable: `land` takes the whole run, so its worker is always
            // present.
            return Landed::Failed;
        };
        match worker.join() {
            Err(payload) => {
                std::mem::forget(payload);
                Landed::Failed
            }
            Ok(_) if run.cancel.load(Ordering::Relaxed) => Landed::Cancelled,
            Ok(None) => Landed::Cancelled,
            Ok(Some(_)) if run.revision != revision => Landed::Stale,
            Ok(Some(plan)) => Landed::Ready(plan),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode};

    fn doc() -> Document {
        Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight)
    }

    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let started = Instant::now();
        while !done() {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "timed out waiting for {what}"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_second_start_is_refused_until_the_first_lands() {
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 1, |_, _, _| Some(1)));
        assert!(!job.start(doc(), 2, |_, _, _| Some(2)));
        wait_until("the first worker to finish", || !job.running());
        assert!(
            !job.start(doc(), 2, |_, _, _| Some(2)),
            "a finished but unlanded run still holds the slot"
        );
        assert_eq!(job.land(1), Landed::Ready(1));

        assert!(job.start(doc(), 3, |_, _, _| Some(3)));
        assert_eq!(
            job.land(2),
            Landed::Stale,
            "a restarted run at the old revision"
        );
        assert!(job.start(doc(), 4, |_, _, _| Some(4)));
        assert_eq!(job.land(4), Landed::Ready(4));
    }

    #[test]
    fn a_run_reports_running_and_progress() {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let release = Arc::new(AtomicBool::new(false));
        let worker_release = release.clone();
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 7, move |_, cancel, progress| {
            progress.store(500, Ordering::Relaxed);
            let _ = started_tx.send(());
            while !worker_release.load(Ordering::Relaxed) && !cancel.load(Ordering::Relaxed) {
                std::thread::yield_now();
            }
            Some(9)
        }));
        started_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the worker started");
        assert!(job.running());
        assert_eq!(job.progress(), 500);

        release.store(true, Ordering::Relaxed);
        wait_until("the worker to finish", || !job.running());
        assert_eq!(job.progress(), 500, "progress holds until landing");
        assert_eq!(job.land(7), Landed::Ready(9));
        assert_eq!(job.progress(), 0);
        assert!(!job.running());
    }

    #[test]
    fn a_finished_plan_lands_once() {
        let (finished_tx, finished_rx) = std::sync::mpsc::channel();
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 0, move |_, _, _| {
            let _ = finished_tx.send(());
            Some(5)
        }));
        finished_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the worker finished before the first poll");
        wait_until("the worker to finish", || !job.running());
        assert_eq!(
            job.land(0),
            Landed::Ready(5),
            "completion before the first poll"
        );
        job.stop();
        assert_eq!(job.progress(), 0);
        assert_eq!(job.land(0), Landed::Empty, "a landed plan is consumed once");
    }

    #[test]
    fn a_stale_plan_is_dropped() {
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 4, |_, _, _| Some(7)));
        assert_eq!(job.land(5), Landed::Stale);

        assert!(job.start(doc(), 5, |_, _, _| Some(8)));
        assert_eq!(job.land(5), Landed::Ready(8));
        assert!(job.start(doc(), 5, |_, _, _| None));
        assert_eq!(job.land(5), Landed::Cancelled);
    }

    #[test]
    fn a_cancelled_plan_lands_nothing() {
        let ran = Arc::new(AtomicBool::new(false));
        let seen = ran.clone();
        let mut job = PreparedJob::<u8>::default();
        job.set_delay_for_test(30_000);
        assert!(job.start(doc(), 1, move |_, _, _| {
            seen.store(true, Ordering::Relaxed);
            Some(6)
        }));
        job.stop();
        assert_eq!(
            job.land(2),
            Landed::Cancelled,
            "a stop wins over a stale revision"
        );
        assert!(ran.load(Ordering::Relaxed), "the worker ran and was joined");

        let completed = Arc::new(AtomicBool::new(false));
        let completed_seen = completed.clone();
        job.set_delay_for_test(0);
        assert!(job.start(doc(), 3, move |_, _, _| {
            completed_seen.store(true, Ordering::Relaxed);
            Some(7)
        }));
        wait_until("the worker to finish", || !job.running());
        job.stop();
        assert_eq!(
            job.land(3),
            Landed::Cancelled,
            "a stop after the worker finished still commits nothing"
        );
        assert!(completed.load(Ordering::Relaxed));
    }

    #[test]
    fn a_worker_panic_lands_nothing() {
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 0, |_, _, _| -> Option<u8> {
            panic!("injected worker panic")
        }));
        assert_eq!(job.land(0), Landed::Failed);
        assert_eq!(job.land(0), Landed::Empty);
    }

    #[test]
    fn dropping_a_run_asks_its_worker_to_stop() {
        let release = Arc::new(AtomicBool::new(false));
        let worker_release = release.clone();
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 0, move |_, _, _| {
            while !worker_release.load(Ordering::Relaxed) {
                std::thread::yield_now();
            }
            None
        }));
        let cancel = job.run.as_ref().expect("a held run").cancel.clone();
        let (dropped_tx, dropped_rx) = std::sync::mpsc::channel();
        let helper = std::thread::spawn(move || {
            drop(job);
            let _ = dropped_tx.send(());
        });
        let returned = dropped_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        release.store(true, Ordering::Relaxed);
        helper.join().expect("the helper thread finished");
        assert!(returned, "dropping the owner must not join its worker");
        assert!(
            cancel.load(Ordering::Relaxed),
            "dropping the owner asked the worker to stop"
        );
    }

    #[test]
    fn discarding_a_run_allows_a_new_start() {
        let release = Arc::new(AtomicBool::new(false));
        let worker_release = release.clone();
        let mut job = PreparedJob::<u8>::default();
        assert!(job.start(doc(), 1, move |_, _, _| {
            while !worker_release.load(Ordering::Relaxed) {
                std::thread::yield_now();
            }
            None
        }));
        let old = job.run.as_ref().expect("a held run").cancel.clone();
        let (job_tx, job_rx) = std::sync::mpsc::channel();
        let helper = std::thread::spawn(move || {
            job.discard();
            let _ = job_tx.send(job);
        });
        let first = job_rx.recv_timeout(Duration::from_secs(2));
        let returned = first.is_ok();
        release.store(true, Ordering::Relaxed);
        let mut job = match first {
            Ok(job) => job,
            Err(_) => job_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("the discarded job came back"),
        };
        helper.join().expect("the helper thread finished");
        assert!(returned, "discarding must not join its worker");
        assert!(
            old.load(Ordering::Relaxed),
            "discarding asked the worker to stop"
        );
        assert_eq!(job.land(1), Landed::Empty);

        assert!(job.start(doc(), 2, |_, _, _| Some(4)));
        assert_eq!(job.land(2), Landed::Ready(4));
    }
}
