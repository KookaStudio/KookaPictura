//! Opt-in cold CPU composite measurement. Supply PICTURA_BENCH_PSD and
//! PICTURA_BENCH_PROFILE=test, then run
//! `cargo test -p pictura-render --test bevel_cold_composite_profile -- --ignored --nocapture`.

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

#[test]
#[ignore = "profiling: set PICTURA_BENCH_PSD and run --ignored --nocapture"]
fn bevel_cold_composite_profile() {
    let path = std::env::var_os("PICTURA_BENCH_PSD").expect("set PICTURA_BENCH_PSD");
    let profile = std::env::var("PICTURA_BENCH_PROFILE")
        .expect("set PICTURA_BENCH_PROFILE to the Cargo profile used to build this test");
    let started = Instant::now();
    let bytes = std::fs::read(path).expect("read benchmark input");
    let read = started.elapsed();
    let started = Instant::now();
    let doc = pictura_codec::read_psd(&bytes).expect("decode benchmark input");
    let decode = started.elapsed();
    let workers = rayon::current_num_threads();
    let started = Instant::now();
    let out = black_box(pictura_render::composite_rgba(&doc));
    let composite = started.elapsed();
    assert_eq!(
        (out.width, out.height, out.channels),
        (doc.width, doc.height, 4)
    );
    let digest = pictura_testkit::hash_bytes(out.data.as_ref());
    eprintln!(
        "cold CPU composite: profile={profile} release_assertions={} workers={workers} read={read:?} decode={decode:?} composite={composite:?} hash={digest:016x}",
        !cfg!(debug_assertions),
    );
    assert!(
        composite < Duration::from_secs(4),
        "cold composite {composite:?} exceeds 4 seconds"
    );
}
