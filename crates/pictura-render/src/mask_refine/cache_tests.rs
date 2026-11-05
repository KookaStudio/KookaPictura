use super::*;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

fn bounded<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, finished) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = work();
        let _ = done.send(());
        result
    });
    if let Err(RecvTimeoutError::Timeout) = finished.recv_timeout(Duration::from_secs(15)) {
        panic!("feather-cache lookup deadlocked the pool");
    }
    worker.join().expect("feather-cache lookup completes")
}

fn key() -> Key {
    let identity: Plane<u8> = vec![37].into();
    Key {
        source: identity.ident(),
        rect: [0, 0, 1, 1],
        clip: [0, 0, 1, 1],
        outside: 0,
        feather: 1.0f64.to_bits(),
    }
}

fn plane() -> Feathered {
    Feathered::new([0, 0, 1, 1], 37, 0)
}

#[test]
fn feather_cache_same_thread_reentry_completes() {
    bounded(|| {
        let key = key();
        let ran = std::sync::atomic::AtomicBool::new(false);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let (outer, inner) = pool.install(|| {
            rayon::join(
                || {
                    sample(key, 0, 0, || {
                        ran.store(
                            rayon::yield_now() == Some(rayon::Yield::Executed),
                            Ordering::SeqCst,
                        );
                        plane()
                    })
                },
                || sample(key, 0, 0, plane),
            )
        });
        assert!(ran.load(Ordering::SeqCst), "lookup ran inside its build");
        assert_eq!(outer, 37);
        assert_eq!(inner, outer);
        assert_eq!(sample(key, 0, 0, plane), 37);
    });
}

#[test]
fn feather_cache_stolen_half_completes() {
    bounded(|| {
        let key = key();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let (sent, received) = mpsc::channel();
        let mut inner = None;
        let mut workers = (None, None);
        let outer = pool.install(|| {
            shared_plane(key, || {
                let (first_worker, ()) = rayon::join(
                    move || {
                        let worker = rayon::current_thread_index();
                        received
                            .recv_timeout(Duration::from_secs(5))
                            .expect("stolen lookup completes while outer build is in flight");
                        worker
                    },
                    || {
                        workers.1 = rayon::current_thread_index();
                        inner = Some(shared_plane(key, plane));
                        sent.send(()).unwrap();
                    },
                );
                workers.0 = first_worker;
                plane()
            })
        });
        assert_ne!(workers.0, workers.1, "the lookup ran on the other worker");
        assert_eq!(outer.at(0, 0), inner.unwrap().at(0, 0));
        assert_eq!(outer.at(0, 0), 37);
    });
}
