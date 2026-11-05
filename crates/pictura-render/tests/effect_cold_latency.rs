//! Workstation acceptance against the production library, without unit-test
//! counters in mask sampling. Inputs are supplied locally through environment.

use std::time::{Duration, Instant};

#[test]
#[ignore = "workstation profiling: supply PICTURA_EFFECT_PROFILE_INPUT and PICTURA_EFFECT_PROFILE_REFERENCE; run --ignored --nocapture"]
fn effect_cold_latency() {
    let input = std::env::var_os("PICTURA_EFFECT_PROFILE_INPUT").expect("supply profiling input");
    let reference = std::env::var_os("PICTURA_EFFECT_PROFILE_REFERENCE")
        .expect("supply original CPU reference");
    let bytes = std::fs::read(input).expect("read supplied input");
    let doc = pictura_codec::read_psd(&bytes).expect("decode supplied input");
    let reference = std::fs::read(reference).expect("read original CPU reference");
    let workers = std::env::var("PICTURA_EFFECT_PROFILE_WORKERS")
        .ok()
        .map(|value| value.parse::<usize>().expect("worker count"))
        .unwrap_or(16);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .unwrap();
    let mut samples = Vec::new();
    for sample in 0..3 {
        pictura_render::purge_source_cache();
        let began = Instant::now();
        let cold = pool.install(|| pictura_render::composite_rgba(&doc));
        let elapsed = began.elapsed();
        let began = Instant::now();
        let warm = pool.install(|| pictura_render::composite_rgba(&doc));
        let warm_elapsed = began.elapsed();
        assert_eq!(
            cold.data.as_ref(),
            reference.as_slice(),
            "original CPU output"
        );
        assert_eq!(cold.data, warm.data);
        let peak = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| {
                status
                    .lines()
                    .find(|line| line.starts_with("VmHWM:"))
                    .and_then(|line| line.split_whitespace().nth(1)?.parse::<u64>().ok())
            })
            .unwrap_or(0);
        eprintln!("production_effect_profile sample={sample} workers={workers} size={}x{} cold_ms={:.3} warm_ms={:.3} peak_kib={peak} hash={:016x}", doc.width, doc.height, elapsed.as_secs_f64() * 1000.0, warm_elapsed.as_secs_f64() * 1000.0, pictura_testkit::hash_bytes(cold.data.as_ref()));
        samples.push(elapsed);
    }
    let first = samples[0];
    samples.sort();
    let median = samples[samples.len() / 2];
    eprintln!(
        "production_effect_acceptance first_ms={:.3} median_ms={:.3}",
        first.as_secs_f64() * 1000.0,
        median.as_secs_f64() * 1000.0
    );
    assert!(
        first <= Duration::from_millis(1000),
        "first cold composite exceeds 1000 ms"
    );
    assert!(
        median <= Duration::from_millis(1000),
        "median cold composite exceeds 1000 ms"
    );
}
