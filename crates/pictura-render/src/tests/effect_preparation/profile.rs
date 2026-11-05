use super::*;
use std::time::Instant;

fn generated() -> Document {
    let (w, h) = (1200, 800);
    let mut layers = Vec::new();
    for index in 0..2 {
        let mut layer = solid(
            "profile",
            rect(70, 80 + index * 120, 730, 1000 + index * 70),
            (90, 160, 210),
            255,
            BlendMode::Normal,
            255,
        );
        for (key, value) in [
            ("bevel.on", 1.0),
            ("bevel.style", 2.0),
            ("bevel.size", 100.0),
            ("bevel.depth", 1000.0),
            ("innerGlow.on", 1.0),
        ] {
            crate::set_layer_style_value(&mut layer, key, value);
        }
        layers.push(layer);
    }
    doc(w, h, layers)
}

fn peak_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .and_then(|line| line.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

fn read_reference(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).expect("supply an existing independently generated original CPU reference")
}

#[test]
fn profile_reference_requires_existing_bytes() {
    let identity: pictura_core::Plane<u8> = vec![0].into();
    let dir = std::env::temp_dir().join(format!(
        "pictura-effect-reference-{}-{}",
        std::process::id(),
        identity.ident()
    ));
    std::fs::create_dir(&dir).expect("create reference test directory");
    let path = dir.join("original.rgba");
    let missing = std::panic::catch_unwind(|| read_reference(&path));
    assert!(missing.is_err(), "missing baseline must fail");
    assert!(!path.exists(), "profiling must not generate a baseline");
    let expected = [3, 19, 71, 255, 0, 8, 27, 100];
    std::fs::write(&path, expected).unwrap();
    assert_eq!(read_reference(&path), expected);
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
#[ignore = "profiling: run --ignored --nocapture --test-threads=1"]
fn banded_effect_preparation_profile() {
    let _cache = crate::layer_source::cache_test_guard();
    let d = if let Some(input) = std::env::var_os("PICTURA_EFFECT_PROFILE_INPUT") {
        let bytes =
            std::fs::read(input).unwrap_or_else(|_| panic!("cannot read supplied profiling input"));
        pictura_codec::read_psd(&bytes)
            .unwrap_or_else(|_| panic!("cannot decode supplied profiling input"))
    } else {
        generated()
    };
    assert!(crate::composite_region::banded(&d, d.height));
    let workers = std::env::var("PICTURA_EFFECT_PROFILE_WORKERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let repeats = std::env::var("PICTURA_EFFECT_PROFILE_REPEATS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3usize);
    assert!(repeats > 0);
    let pool = pool(workers);
    let reference = std::env::var_os("PICTURA_EFFECT_PROFILE_REFERENCE")
        .map(|path| read_reference(std::path::Path::new(&path)));
    let limit = std::env::var("PICTURA_EFFECT_PROFILE_MAX_MS")
        .ok()
        .map(|value| value.parse::<f64>().expect("numeric time limit"));
    let modes: &[bool] = match std::env::var("PICTURA_EFFECT_PROFILE_MODE").as_deref() {
        Ok("serial") => &[false],
        Ok("parallel") => &[true],
        _ => &[false, true],
    };
    let mut expected = None;
    let mut cold_samples = [Vec::new(), Vec::new()];
    for &parallel in modes {
        for sample in 0..repeats {
            crate::purge_source_cache();
            let guard = Guard::new(&d, vec![]);
            let begin = Instant::now();
            let cold = pool.install(|| render(&d, parallel));
            let cold_time = begin.elapsed();
            cold_samples[usize::from(parallel)].push(cold_time.as_secs_f64());
            let records = guard.probe.snapshot();
            drop(guard);
            let plan_time = records
                .iter()
                .find(|r| matches!(r.event, Event::Band))
                .unwrap()
                .time;
            for (index, start) in records
                .iter()
                .filter(|r| matches!(r.event, Event::BuildStart(_)))
                .enumerate()
            {
                let end = records
                    .iter()
                    .find(|r| {
                        r.layer == start.layer
                            && matches!(r.event, Event::BuildEnd)
                            && r.time >= start.time
                    })
                    .unwrap();
                eprintln!("effect_build mode={parallel} sample={sample} index={index} start_ms={:.3} end_ms={:.3}", start.time.as_secs_f64() * 1000.0, end.time.as_secs_f64() * 1000.0);
                let mut previous = start.time;
                for record in records.iter().filter(|record| {
                    record.layer == start.layer
                        && record.time >= start.time
                        && record.time <= end.time
                }) {
                    if let Event::Stage(stage) = record.event {
                        eprintln!(
                            "effect_stage index={index} stage={stage} ms={:.3}",
                            (record.time - previous).as_secs_f64() * 1000.0
                        );
                        previous = record.time;
                    }
                }
            }
            let begin = Instant::now();
            let warm = pool.install(|| render(&d, parallel));
            let warm_time = begin.elapsed();
            assert_eq!(cold.data, warm.data);
            if let Some(reference) = &reference {
                assert_eq!(
                    cold.data.as_ref(),
                    reference.as_slice(),
                    "original CPU bytes"
                );
            }
            if let Some(expected) = &expected {
                assert_eq!(&cold.data, expected);
            } else {
                expected = Some(cold.data.clone());
            }
            eprintln!("effect_profile mode={parallel} sample={sample} workers={workers} size={}x{} cold_ms={:.3} plan_ms={:.3} warm_ms={:.3} peak_kib={} hash={:016x}", d.width, d.height, cold_time.as_secs_f64() * 1000.0, plan_time.as_secs_f64() * 1000.0, warm_time.as_secs_f64() * 1000.0, peak_kib(), pictura_testkit::hash_bytes(cold.data.as_ref()));
            if let Some(limit) = limit {
                assert!(
                    cold_time.as_secs_f64() * 1000.0 <= limit,
                    "cold composite exceeds {limit} ms"
                );
            }
        }
    }
    if cold_samples.iter().all(|samples| !samples.is_empty()) {
        for samples in &mut cold_samples {
            samples.sort_by(f64::total_cmp);
        }
        let serial = cold_samples[0][cold_samples[0].len() / 2];
        let parallel = cold_samples[1][cold_samples[1].len() / 2];
        eprintln!(
            "effect_speedup cold={:.3} workers={workers}",
            serial / parallel
        );
    }
}
