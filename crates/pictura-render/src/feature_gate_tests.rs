//! The `gpu` feature contract: what the manifest declares, and what a build
//! with and without it can prove about the public surface.
//!
//! The feature state is a compile-time fact, so the two surface tests are gated
//! on it; the manifest tests read the sibling manifests the workspace ships.

use std::path::PathBuf;

fn manifest(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// The `[features]` table of a manifest, as `name = [deps]` lines.
fn features(text: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut in_table = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_table = trimmed == "[features]";
            continue;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, value)) = trimmed.split_once('=') {
            out.push((name.trim(), value.trim()));
        }
    }
    out
}

fn dependency_lines(text: &str, name: &str) -> Vec<String> {
    let lines: Vec<String> = text
        .lines()
        .filter(|line| line.trim_start().starts_with(&format!("{name} = ")))
        .map(str::to_string)
        .collect();
    assert!(!lines.is_empty(), "no dependency line for {name}");
    lines
}

#[test]
fn renderer_manifest_gates_wgpu_behind_the_gpu_feature() {
    let text = manifest("Cargo.toml");
    let features = features(&text);
    let names: Vec<&str> = features.iter().map(|(name, _)| *name).collect();
    assert!(
        !names.contains(&"default"),
        "the default feature set must stay empty: {names:?}"
    );
    let gpu = features
        .iter()
        .find(|(name, _)| *name == "gpu")
        .map(|(_, value)| *value)
        .expect("the gpu feature is declared");
    assert!(gpu.contains("dep:wgpu"), "gpu enables wgpu: {gpu}");
    assert!(gpu.contains("dep:pollster"), "gpu enables pollster: {gpu}");
    for name in ["wgpu", "pollster"] {
        for line in dependency_lines(&text, name) {
            assert!(
                line.contains("optional = true"),
                "{name} must be optional in every target table: {line}"
            );
        }
    }
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_backend_is_one_type_behind_the_feature() {
    let _ = crate::probe_backend(crate::GpuBackend::Cpu);
    let from_gpu: crate::gpu::Backend = crate::Backend::Cpu;
    let from_root: crate::Backend = from_gpu;
    assert_eq!(from_root, crate::Backend::Cpu);
    let _ = crate::SOFTWARE_GPU;
}

#[cfg(not(feature = "gpu"))]
#[test]
fn gpu_enabled_request_falls_back_to_the_cpu_without_the_feature() {
    use pictura_core::{BitDepth, ColorMode, Document};

    let doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let (active, backend) = crate::composite(crate::CompositeRequest::whole(&doc, true));
    assert_eq!(backend, crate::Backend::Cpu);
    assert_eq!(active, crate::composite_rgba(&doc));
}

#[test]
fn app_manifest_enables_the_gpu_feature() {
    let text = manifest("../pictura-app/Cargo.toml");
    let body = text
        .split_once("[dependencies]")
        .map(|(_, rest)| rest)
        .expect("the app manifest has a dependencies table");
    let line = body
        .lines()
        .find(|line| line.trim_start().starts_with("pictura-render = "))
        .expect("pictura-render is a dependency");
    assert!(
        line.contains("features = [") && line.contains("\"gpu\""),
        "the app dependency must enable gpu: {line}"
    );
    // The dev-dependency stays oracle-only: the `view_oracle` guard pins that
    // line, and Cargo unifies it with the dependency above for test builds.
    let dev = text
        .split_once("[dev-dependencies]")
        .map(|(_, rest)| rest)
        .expect("the app manifest has a dev-dependencies table");
    let dev_line = dev
        .lines()
        .find(|line| line.trim_start().starts_with("pictura-render = "))
        .expect("pictura-render is a dev-dependency");
    assert!(
        dev_line.contains("\"test-oracle\"") && !dev_line.contains("\"gpu\""),
        "the dev-dependency must stay oracle-only: {dev_line}"
    );
}

#[test]
fn software_gpu_implies_gpu() {
    let text = manifest("Cargo.toml");
    let features = features(&text);
    let software = features
        .iter()
        .find(|(name, _)| *name == "software-gpu")
        .map(|(_, value)| *value)
        .expect("the software-gpu feature is declared");
    assert!(
        software.contains("\"gpu\""),
        "software-gpu must imply gpu: {software}"
    );
}
