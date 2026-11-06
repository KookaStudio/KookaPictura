//! Application cold profile: production open, a real full edit, repeated
//! fixed-direction edits, and a warm region.
//!
//! Run with
//! `cargo test -p pictura_app --lib first_full_edit_profile -- --ignored --nocapture`.
//! Set `PICTURA_PROFILE_PSD` to use a local document; otherwise a generated
//! fixture is used.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::{pixel_layer, PictureViewRust};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::impl_core::{
    last_backend, refresh_calls, reset_last_backend, reset_refresh_calls,
};
use crate::cxxqt_object::metrics;
use crate::cxxqt_object::open_jobs::{apply_opened, decode_psd};
use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerMask, LayerStack, PsdRect};
use pictura_render::cold_profile::{Cohort, Report, PRIMARY, REGION, REPEATED_FULL};

const CHILD: &str = "cxxqt_object::tests_profiles::cold::first_full_edit_child";
const SAMPLES: usize = 3;

fn full(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn scene(w: u32, h: u32) -> Document {
    let background = pixel_layer("Background", w, h, (240, 240, 240));
    let group = Layer {
        name: "group".into(),
        rect: full(w, h),
        is_group: true,
        children: LayerStack::from_layers(vec![
            pixel_layer("group-a", w, h, (30, 90, 160)),
            pixel_layer("group-b", w, h, (200, 120, 40)),
        ]),
        ..Default::default()
    };
    let mut base = pixel_layer("base", w, h, (255, 255, 255));
    base.name = "base".to_string();
    let mut clipped = pixel_layer("clipped", w, h, (10, 200, 80));
    clipped.clipping = true;
    let mut masked = pixel_layer("masked", w, h, (90, 40, 180));
    masked.mask = Some(LayerMask {
        rect: full(w, h),
        default_color: 0,
        data: Some(vec![128; (w * h) as usize].into()),
        ..Default::default()
    });
    let mut styled = pixel_layer("styled", w, h, (210, 70, 70));
    styled.opacity = 200;
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    let mut hidden = pixel_layer("hidden", w, h, (255, 0, 255));
    hidden.visible = false;
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        background, group, base, clipped, masked, styled, hidden,
    ]);
    doc
}

fn write_input(path: &Path, w: u32, h: u32) {
    let bytes = pictura_codec::write_psd(&scene(w, h)).expect("generated PSD writes");
    std::fs::write(path, bytes).expect("generated PSD");
}

fn temp_dir(label: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("pictura-cold-app-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

/// The topmost visible, unlocked pixel layer with opacity below 255 and area.
fn compatible_target(doc: &Document) -> Result<usize, String> {
    doc.layers
        .iter()
        .enumerate()
        .rev()
        .find(|(_, layer)| {
            layer.visible
                && !layer.is_group
                && !layer.clipping
                && layer.lock.bits() == 0
                && layer.opacity < 255
                && layer.rect.width() > 0
                && layer.rect.height() > 0
                && layer.adjustment.is_none()
        })
        .map(|(index, _)| index)
        .ok_or_else(|| "requested profile input has no compatible edit target".to_string())
}

/// Resolve the optional local input without recording it in an error.
fn resolve_input(explicit: Option<&Path>, generated: &Path) -> Result<PathBuf, String> {
    match explicit {
        None => Ok(generated.to_path_buf()),
        Some(path) if path.is_file() => Ok(path.to_path_buf()),
        Some(_) => Err("requested profile input is missing".to_string()),
    }
}

fn set_opacity(view: &mut PictureViewRust, index: usize, value: u8) {
    view.session
        .doc
        .as_mut()
        .expect("open document")
        .layers
        .get_mut(index)
        .expect("edit target")
        .opacity = value;
}

fn open_and_edit(path: &Path, gpu: bool, metrics_on: bool) -> (PictureViewRust, usize) {
    metrics::set_for_this_thread(metrics_on);
    let started = Instant::now();
    let opened = decode_psd(
        path.to_str().expect("utf-8 path"),
        pictura_codec::Policy::Preserve,
        gpu,
        &|_, _| {},
    )
    .expect("production decode");
    let mut view = PictureViewRust {
        gpu_compute: gpu,
        ..Default::default()
    };
    apply_opened(&mut view, opened);
    let opening = started.elapsed();
    assert!(!opening.is_zero());

    let index = compatible_target(view.session.doc.as_ref().expect("document")).expect("target");
    let original = view
        .session
        .doc
        .as_ref()
        .unwrap()
        .layers
        .get(index)
        .unwrap()
        .opacity;
    let alternate = if original == 255 { 128 } else { 255 };
    reset_refresh_calls();
    reset_last_backend();
    set_opacity(&mut view, index, alternate);
    let edit = Instant::now();
    let _ = view.apply_edit("Opacity", Edit::Full);
    view.reset_pyramid();
    let first_edit = edit.elapsed();
    assert_eq!(refresh_calls(), 1, "exactly one full composite");
    assert!(!first_edit.is_zero());
    (view, index)
}

fn digest(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn cohort(gpu: bool, metrics_on: bool) -> Cohort {
    Cohort {
        mode: "application".to_string(),
        input: "generated-app-v1".to_string(),
        edit_recipe: "opacity-a-to-b".to_string(),
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
        requested_backend: if gpu { "gpu" } else { "cpu" }.to_string(),
        executed_backend: "cpu".to_string(),
        fallback: None,
        metrics: if metrics_on { "on" } else { "off" }.to_string(),
    }
}

fn median(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn wait_for(mut child: std::process::Child) -> std::process::ExitStatus {
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

fn run_child(input: &Path, report: &Path, gpu: bool, metrics_on: bool) -> std::process::ExitStatus {
    let child = Command::new(std::env::current_exe().expect("test binary"))
        .arg("--exact")
        .arg(CHILD)
        .arg("--ignored")
        .arg("--nocapture")
        .env("PICTURA_COLD_CHILD", "1")
        .env("PICTURA_COLD_INPUT", input)
        .env("PICTURA_PROFILE_REPORT", report)
        .env("PICTURA_COLD_GPU", if gpu { "1" } else { "0" })
        .env("PICTURA_METRICS", if metrics_on { "1" } else { "0" })
        .env("RAYON_NUM_THREADS", "4")
        .spawn()
        .expect("spawn profile child");
    wait_for(child)
}

fn run_save_child(input: &Path, output: &Path, metrics_on: bool) -> std::process::ExitStatus {
    std::fs::create_dir_all(output).expect("create save output dir");
    let child = Command::new(std::env::current_exe().expect("test binary"))
        .arg("--exact")
        .arg("cxxqt_object::tests_profiles::cold::profile_save_child")
        .arg("--ignored")
        .arg("--nocapture")
        .env("PICTURA_COLD_CHILD", "1")
        .env("PICTURA_COLD_INPUT", input)
        .env("PICTURA_COLD_OUTPUT_DIR", output)
        .env("PICTURA_METRICS", if metrics_on { "1" } else { "0" })
        .spawn()
        .expect("spawn save child");
    wait_for(child)
}

#[test]
fn first_full_edit_follows_production_open() {
    let dir = temp_dir("sequence");
    let input = dir.join("input.psd");
    write_input(&input, 96, 64);
    let (view, _) = open_and_edit(&input, false, false);
    assert!(!view.session.doc.as_ref().unwrap().composite.data.is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn profile_local_input_errors_are_redacted() {
    let dir = temp_dir("input");
    let generated = dir.join("generated.psd");
    let missing = dir.join("sentinel-name.psd");
    assert_eq!(
        resolve_input(None, &generated).expect("generated fallback"),
        generated
    );
    let error = resolve_input(Some(&missing), &generated).expect_err("missing must fail");
    assert!(!error.contains("sentinel-name"), "redacted: {error}");
    assert!(!error.contains(dir.to_str().unwrap()), "redacted: {error}");

    let mut doc = scene(96, 64);
    for layer in doc.layers.iter_mut() {
        layer.opacity = 255;
    }
    let error = compatible_target(&doc).expect_err("no compatible target");
    assert!(!error.is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn profile_metrics_preserve_open_edit_and_saved_samples() {
    let dir = temp_dir("exact");
    let input = dir.join("input.psd");
    write_input(&input, 96, 64);
    let off = dir.join("off");
    let on = dir.join("on");
    assert!(run_save_child(&input, &off, false).success());
    assert!(run_save_child(&input, &on, true).success());
    assert_eq!(
        std::fs::read(off.join("composite.bin")).unwrap(),
        std::fs::read(on.join("composite.bin")).unwrap(),
        "composite is identical with metrics on/off"
    );
    assert_eq!(
        std::fs::read(off.join("saved.psd")).unwrap(),
        std::fs::read(on.join("saved.psd")).unwrap(),
        "saved samples are identical with metrics on/off"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
#[ignore = "profile: cargo test -p pictura_app --lib first_full_edit_profile -- --ignored --nocapture"]
fn first_full_edit_profile() {
    if std::env::var_os("PICTURA_COLD_CHILD").is_some() {
        return;
    }
    let dir = temp_dir("parent");
    let generated = dir.join("generated.psd");
    write_input(&generated, 1200, 800);
    let explicit = std::env::var_os("PICTURA_PROFILE_PSD").map(PathBuf::from);
    let input = resolve_input(explicit.as_deref(), &generated).expect("profile input");

    for sample in 0..SAMPLES {
        let report = dir.join(format!("off-{sample}.report"));
        let status = run_child(&input, &report, false, false);
        assert!(status.success(), "metrics-off child failed: {status}");
        let parsed = Report::read(&report).expect("read report");
        assert_eq!(parsed.cohort.metrics, "off");
        assert!(parsed.metrics[PRIMARY] > 0);
    }
    let attribution = dir.join("on.report");
    let status = run_child(&input, &attribution, false, true);
    assert!(status.success(), "metrics-on child failed: {status}");
    let parsed = Report::read(&attribution).expect("read attribution report");
    assert!(parsed.metrics.contains_key("first_edit_composite"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
#[ignore = "profile child: launched by first_full_edit_profile"]
fn first_full_edit_child() {
    if std::env::var_os("PICTURA_COLD_CHILD").is_none() {
        return;
    }
    let input = PathBuf::from(std::env::var_os("PICTURA_COLD_INPUT").expect("PICTURA_COLD_INPUT"));
    let output =
        PathBuf::from(std::env::var_os("PICTURA_PROFILE_REPORT").expect("PICTURA_PROFILE_REPORT"));
    let gpu = std::env::var("PICTURA_COLD_GPU").as_deref() == Ok("1");
    let metrics_on = std::env::var("PICTURA_METRICS").as_deref() == Ok("1");

    metrics::set_for_this_thread(metrics_on);
    let started = Instant::now();
    let opened = decode_psd(
        input.to_str().expect("utf-8 path"),
        pictura_codec::Policy::Preserve,
        gpu,
        &|_, _| {},
    )
    .expect("production decode");
    let opening_composite = metrics::phase_ns("opening_composite");
    let mut view = PictureViewRust {
        gpu_compute: gpu,
        ..Default::default()
    };
    apply_opened(&mut view, opened);
    let opening = started.elapsed().as_nanos() as u64;

    let index = compatible_target(view.session.doc.as_ref().expect("document")).expect("target");
    let original = view
        .session
        .doc
        .as_ref()
        .unwrap()
        .layers
        .get(index)
        .unwrap()
        .opacity;
    let alternate = if original == 255 { 128 } else { 255 };
    let before = view.session.doc.as_ref().unwrap().composite.data.to_vec();
    reset_refresh_calls();
    reset_last_backend();
    metrics::set_for_this_thread(metrics_on);
    set_opacity(&mut view, index, alternate);
    let started = Instant::now();
    let _ = view.apply_edit("Opacity", Edit::Full);
    view.reset_pyramid();
    let first_edit = started.elapsed().as_nanos() as u64;
    let first_edit_composite = metrics::phase_ns("recomposite_full_composite");
    assert_eq!(refresh_calls(), 1);
    let after = view.session.doc.as_ref().unwrap().composite.data.to_vec();
    assert_ne!(before, after, "the first edit changes pixels");

    let mut repeated = Vec::new();
    for _ in 0..3 {
        set_opacity(&mut view, index, original);
        let _ = view.apply_edit("Reset", Edit::Full);
        view.reset_pyramid();
        set_opacity(&mut view, index, alternate);
        let started = Instant::now();
        let _ = view.apply_edit("Opacity", Edit::Full);
        view.reset_pyramid();
        repeated.push(started.elapsed().as_nanos() as u64);
    }

    let region = view
        .session
        .doc
        .as_ref()
        .unwrap()
        .layers
        .get(index)
        .unwrap()
        .rect;
    let mut regions = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        view.refresh_regions(&[region]);
        regions.push(started.elapsed().as_nanos() as u64);
    }

    let mut report = Report::new(cohort(gpu, metrics_on));
    report.set_metric(PRIMARY, opening.saturating_add(first_edit));
    report.set_metric("opening", opening);
    report.set_metric("first_edit", first_edit);
    if let Some(ns) = opening_composite {
        report.set_metric("opening_composite", ns);
    }
    if let Some(ns) = first_edit_composite {
        report.set_metric("first_edit_composite", ns);
    }
    report.set_metric(REPEATED_FULL, median(&repeated));
    report.set_metric(REGION, median(&regions));
    report.cohort.executed_backend = match last_backend() {
        Some(pictura_render::Backend::Gpu) => "gpu",
        _ => "cpu",
    }
    .to_string();
    if gpu && !pictura_render::gpu_available() {
        report.cohort.fallback = Some("unavailable".to_string());
    }
    report.set_digest("output", digest(&after));
    report.write(&output).expect("write report");
}

#[test]
#[ignore = "profile child: launched by profile_metrics_preserve_open_edit_and_saved_samples"]
fn profile_save_child() {
    if std::env::var_os("PICTURA_COLD_CHILD").is_none() {
        return;
    }
    let input = PathBuf::from(std::env::var_os("PICTURA_COLD_INPUT").expect("PICTURA_COLD_INPUT"));
    let output = PathBuf::from(
        std::env::var_os("PICTURA_COLD_OUTPUT_DIR").expect("PICTURA_COLD_OUTPUT_DIR"),
    );
    let metrics_on = std::env::var("PICTURA_METRICS").as_deref() == Ok("1");
    let (view, _) = open_and_edit(&input, false, metrics_on);
    let composite = view.session.doc.as_ref().unwrap().composite.data.to_vec();
    let saved = pictura_codec::write_psd(view.session.doc.as_ref().unwrap()).expect("save");
    std::fs::write(output.join("composite.bin"), composite).expect("write composite");
    std::fs::write(output.join("saved.psd"), saved).expect("write saved");
}

#[test]
fn first_full_edit_child_failure_is_reported() {
    let dir = temp_dir("failure");
    let missing = dir.join("missing.psd");
    let report = dir.join("failure.report");
    let status = run_child(&missing, &report, false, false);
    assert!(!status.success(), "a missing input must fail the child");
    std::fs::remove_dir_all(dir).ok();
}
