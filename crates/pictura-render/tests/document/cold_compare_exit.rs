//! The comparator executable's process-exit contract.

use std::path::{Path, PathBuf};
use std::process::Command;

use pictura_render::cold_profile::{Cohort, Report, PRIMARY, REGION, REPEATED_FULL};

fn cohort() -> Cohort {
    Cohort {
        mode: "engine".to_string(),
        input: "generated-nested-v1".to_string(),
        edit_recipe: "opacity-a-to-b".to_string(),
        machine: "test".to_string(),
        build: "debug".to_string(),
        threads: 4,
        requested_backend: "cpu".to_string(),
        executed_backend: "cpu".to_string(),
        fallback: None,
        metrics: "off".to_string(),
    }
}

fn report(primary: u64, repeated_full: u64, region: u64) -> Report {
    let mut report = Report::new(cohort());
    report.set_metric(PRIMARY, primary);
    report.set_metric(REPEATED_FULL, repeated_full);
    report.set_metric(REGION, region);
    report
}

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pictura-cold-compare-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_reports(dir: &Path, values: &[(u64, u64, u64)]) {
    for (index, (primary, repeated, region)) in values.iter().enumerate() {
        report(*primary, *repeated, *region)
            .write(&dir.join(format!("{index:02}.report")))
            .expect("write report");
    }
}

fn run(baseline: &Path, candidate: &Path) -> i32 {
    let status = Command::new(env!("CARGO_BIN_EXE_cold_compare"))
        .arg(baseline)
        .arg(candidate)
        .status()
        .expect("run cold_compare");
    status.code().expect("exit code")
}

#[test]
fn cold_compare_exit_codes() {
    let root = temp_dir("exit");

    let baseline = root.join("baseline");
    let improved = root.join("improved");
    let regressed = root.join("regressed");
    let inconclusive = root.join("inconclusive");
    let insufficient = root.join("insufficient");
    let invalid = root.join("invalid");
    for dir in [
        &baseline,
        &improved,
        &regressed,
        &inconclusive,
        &insufficient,
        &invalid,
    ] {
        std::fs::create_dir_all(dir).expect("create cohort dir");
    }

    write_reports(&baseline, &[(1000, 500, 200); 3]);
    write_reports(&improved, &[(100, 500, 200); 3]);
    write_reports(&regressed, &[(1000, 600, 200); 3]);
    write_reports(&inconclusive, &[(1000, 500, 200); 3]);
    write_reports(&insufficient, &[(1000, 500, 200); 2]);
    write_reports(&invalid, &[(100, 500, 200); 3]);
    let mut mismatched = report(100, 500, 200);
    mismatched.cohort.machine = "other".to_string();
    mismatched
        .write(&invalid.join("02.report"))
        .expect("write mismatched report");

    assert_eq!(run(&baseline, &improved), 0, "improved");
    assert_eq!(run(&baseline, &regressed), 2, "regressed");
    assert_eq!(run(&baseline, &inconclusive), 3, "inconclusive");
    assert_eq!(run(&baseline, &insufficient), 4, "insufficient");
    assert_eq!(run(&baseline, &invalid), 5, "invalid");

    let usage = Command::new(env!("CARGO_BIN_EXE_cold_compare"))
        .status()
        .expect("run without args");
    assert_eq!(usage.code(), Some(1), "usage");

    std::fs::remove_dir_all(root).ok();
}
