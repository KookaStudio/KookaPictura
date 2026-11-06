//! Engine-cold profile: a fresh child process per sample, a generated
//! nested/clipped/masked/styled scene, and a line-oriented report.
//!
//! Run with
//! `cargo test -p pictura-render --test cold_engine_profile -- --ignored --nocapture`.
//! The parent writes the generated PSD before spawning any timed child.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerMask, LayerStack, PsdRect, SmartObject,
    SmartObjectKind,
};
use pictura_render::cold_profile::{Cohort, Report, PRIMARY, REGION, REPEATED_FULL};
use pictura_render::{composite, CompositeRequest};

const CHILD: &str = "cold_engine_child";
const SAMPLES: usize = 3;

fn full(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn pixel(name: &str, r: PsdRect, rgb: [u8; 3], alpha: u8) -> Layer {
    let n = (r.width().max(0) as usize) * (r.height().max(0) as usize);
    Layer {
        name: name.into(),
        rect: r,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: vec![
            Channel {
                id: 0,
                data: vec![rgb[0]; n].into(),
            },
            Channel {
                id: 1,
                data: vec![rgb[1]; n].into(),
            },
            Channel {
                id: 2,
                data: vec![rgb[2]; n].into(),
            },
            Channel {
                id: -1,
                data: vec![alpha; n].into(),
            },
        ],
        ..Default::default()
    }
}

fn embedded_source() -> SmartObject {
    let mut payload = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    payload.layers =
        LayerStack::from_layers(vec![pixel("payload", full(2, 2), [20, 200, 90], 255)]);
    let bytes = pictura_codec::write_psd(&payload).expect("payload writes");
    let mut source = SmartObject::default();
    source.filename = "source.psd".into();
    source.kind = SmartObjectKind::Embedded;
    source.set_payload(bytes);
    source
}

fn scene() -> Document {
    let (w, h) = (1200, 800);
    let background = pixel("Background", full(w, h), [245, 245, 240], 255);
    let group = Layer {
        name: "group".into(),
        rect: full(w, h),
        is_group: true,
        children: LayerStack::from_layers(vec![
            pixel("group-a", rect(40, 40, 600, 400), [30, 90, 160], 255),
            pixel("group-b", rect(80, 80, 640, 440), [200, 120, 40], 180),
        ]),
        ..Default::default()
    };
    let base = pixel("base", rect(100, 120, 700, 620), [255, 255, 255], 255);
    let mut clipped = pixel("clipped", full(w, h), [10, 200, 80], 255);
    clipped.clipping = true;
    let mut masked = pixel("masked", rect(200, 200, 900, 700), [90, 40, 180], 255);
    let mask_rect = masked.rect;
    let mask_len = (mask_rect.width().max(0) as usize) * (mask_rect.height().max(0) as usize);
    let mut mask = LayerMask {
        rect: mask_rect,
        default_color: 0,
        data: Some(vec![128; mask_len].into()),
        ..Default::default()
    };
    mask.set_feather(24.0);
    masked.mask = Some(mask);
    let mut styled = pixel("styled", rect(300, 160, 1000, 600), [210, 70, 70], 255);
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    let smart = Layer {
        name: "smart".into(),
        rect: rect(500, 300, 900, 700),
        smart_object: Some(embedded_source()),
        ..Default::default()
    };
    let mut hidden = pixel("hidden", full(w, h), [255, 0, 255], 255);
    hidden.visible = false;

    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        background, group, base, clipped, masked, styled, smart, hidden,
    ]);
    doc
}

fn write_input(path: &Path) {
    let bytes = pictura_codec::write_psd(&scene()).expect("generated PSD writes");
    std::fs::write(path, bytes).expect("generated PSD");
}

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pictura-cold-engine-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn run_child(input: &Path, report: &Path, metrics: bool) -> std::process::ExitStatus {
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .arg("--exact")
        .arg(CHILD)
        .arg("--ignored")
        .arg("--nocapture")
        .env("PICTURA_COLD_CHILD", "1")
        .env("PICTURA_COLD_INPUT", input)
        .env("PICTURA_PROFILE_REPORT", report)
        .env("PICTURA_METRICS", if metrics { "1" } else { "0" })
        .env("RAYON_NUM_THREADS", "4")
        .spawn()
        .expect("spawn profile child");
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            return status;
        }
        if started.elapsed() > Duration::from_secs(300) {
            child.kill().ok();
            panic!("profile child timed out");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn cohort() -> Cohort {
    Cohort {
        mode: "engine".to_string(),
        input: "generated-nested-v1".to_string(),
        edit_recipe: "none".to_string(),
        machine: std::env::var("PICTURA_PROFILE_MACHINE").unwrap_or_else(|_| "unspecified".into()),
        build: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
        threads: std::env::var("RAYON_NUM_THREADS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(rayon::current_num_threads) as u32,
        requested_backend: std::env::var("PICTURA_COLD_BACKEND").unwrap_or_else(|_| "cpu".into()),
        executed_backend: "cpu".to_string(),
        fallback: None,
        metrics: "off".to_string(),
    }
}

fn median(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn digest(buffer: &pictura_core::PixelBuffer) -> String {
    format!("{:016x}", pictura_testkit::hash_bytes(buffer.data.as_ref()))
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test cold_engine_profile -- --ignored --nocapture"]
fn cold_engine_profile() {
    if std::env::var_os("PICTURA_COLD_CHILD").is_some() {
        return;
    }
    let dir = temp_dir("parent");
    let input = dir.join("input.psd");
    write_input(&input);

    for sample in 0..SAMPLES {
        let report = dir.join(format!("off-{sample}.report"));
        let status = run_child(&input, &report, false);
        assert!(status.success(), "metrics-off child failed: {status}");
        let parsed = Report::read(&report).expect("read metrics-off report");
        assert_eq!(parsed.cohort.metrics, "off");
        assert!(parsed.metrics[PRIMARY] > 0);
    }
    let attribution = dir.join("on.report");
    let status = run_child(&input, &attribution, true);
    assert!(status.success(), "metrics-on child failed: {status}");
    let parsed = Report::read(&attribution).expect("read metrics-on report");
    assert!(!parsed.phases.is_empty(), "attribution records phases");

    let mut reports = Vec::new();
    for sample in 0..SAMPLES {
        reports.push(Report::read(&dir.join(format!("off-{sample}.report"))).expect("report"));
    }
    eprintln!(
        "cold_engine_profile samples={} primary={:?} phases={:?}",
        reports.len(),
        reports
            .iter()
            .map(|r| r.metrics[PRIMARY])
            .collect::<Vec<_>>(),
        parsed.phases.keys().collect::<Vec<_>>()
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
#[ignore = "profile child: launched by cold_engine_profile"]
fn cold_engine_child() {
    if std::env::var_os("PICTURA_COLD_CHILD").is_none() {
        return;
    }
    let input = std::env::var_os("PICTURA_COLD_INPUT").expect("PICTURA_COLD_INPUT");
    let output = std::env::var_os("PICTURA_PROFILE_REPORT").expect("PICTURA_PROFILE_REPORT");
    let backend = std::env::var("PICTURA_COLD_BACKEND").unwrap_or_else(|_| "cpu".into());
    let gpu = backend == "gpu";

    let began = Instant::now();
    let bytes = std::fs::read(&input).expect("read input");
    let doc = pictura_codec::read_psd(&bytes).expect("decode input");
    let decode = began.elapsed().as_nanos() as u64;

    let began = Instant::now();
    let (first, executed) = composite(CompositeRequest::whole(&doc, gpu));
    let first_full = began.elapsed().as_nanos() as u64;
    assert!(!first.data.is_empty(), "first full produced pixels");

    let mut repeated = Vec::new();
    for _ in 0..3 {
        let began = Instant::now();
        let (buffer, _) = composite(CompositeRequest::whole(&doc, gpu));
        repeated.push(began.elapsed().as_nanos() as u64);
        assert_eq!(buffer.data, first.data);
    }
    let mut regions = Vec::new();
    let region = rect(100, 100, 500, 400);
    for _ in 0..3 {
        let began = Instant::now();
        let (buffer, _) = composite(CompositeRequest::region(&doc, region, gpu));
        regions.push(began.elapsed().as_nanos() as u64);
        assert!(!buffer.data.is_empty());
    }

    let phases = pictura_render::metrics::take_phases();
    let mut report = Report::new(cohort());
    report.set_metric(PRIMARY, decode.saturating_add(first_full));
    report.set_metric("decode", decode);
    report.set_metric("first_full", first_full);
    report.set_metric(REPEATED_FULL, median(&repeated));
    report.set_metric(REGION, median(&regions));
    report.cohort.executed_backend = match executed {
        pictura_render::Backend::Gpu => "gpu".to_string(),
        pictura_render::Backend::Cpu => "cpu".to_string(),
    };
    if gpu && !pictura_render::gpu_available() {
        report.cohort.fallback = Some("unavailable".to_string());
    }
    report.cohort.metrics = if pictura_render::metrics::enabled() {
        "on".to_string()
    } else {
        "off".to_string()
    };
    for (label, phase) in phases {
        report.set_phase(&label, phase);
    }
    report.set_digest("output", digest(&first));
    report
        .write(Path::new(&output))
        .expect("write profile report");
}

#[test]
fn cold_engine_child_failure_is_reported() {
    let dir = temp_dir("failure");
    let missing = dir.join("missing.psd");
    let report = dir.join("failure.report");
    let status = run_child(&missing, &report, false);
    assert!(!status.success(), "a missing input must fail the child");
    std::fs::remove_dir_all(dir).ok();
}
