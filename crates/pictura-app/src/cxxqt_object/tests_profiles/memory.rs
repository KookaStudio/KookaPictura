//! Heap-peak guards for the memory benchmark's scenarios M1–M12 (#1210).
//!
//! Each scenario repeats the bridge's work on a `PictureViewRust` (the Qt
//! object cannot be built in a Rust test) and measures the heap it adds over
//! its setup through the binary's counting allocator, `crate::HEAP`. The
//! `*_heap_peak` guards run at 4250² and fail above the recorded baseline ×
//! 1.10, so a later change cannot quietly raise a peak; the ignored
//! `*_heap_peak_full` tests print the peak at 17000².
//!
//! ponytail: the scenarios repeat the bridge by hand, so an allocation a bridge
//! change adds outside the engine calls repeated here goes unseen. The process
//! benchmark (`scripts/profile-tools.py --scenarios memory`) is the end-to-end
//! check. M13 (pan and zoom) is view-only and has no engine path.

mod scenarios;

use super::super::state::PictureViewRust;
use scenarios as sc;

const GUARD_SIDE: u32 = 4250;
const FULL_SIDE: u32 = 17000;
/// Rayon threads for every measurement: a fixed count of per-thread buffers,
/// so the peak does not depend on the machine's cores.
const THREADS: usize = 4;

/// The heap each scenario adds over its setup at 4250², in bytes: the larger
/// of three runs on the reference machine (docs/dev/benchmarks/
/// memory-baseline.json) at the harness's commit, which changes no engine
/// code. Every scenario but snapshot_restore repeats to the byte; that one
/// varied by 3.6 %. Lower an entry when a change saves memory.
///
/// A change that makes a *setup* retain less raises its delta without raising
/// the absolute peak, because the delta is measured from the setup's heap.
/// `edit_fill`, `bucket_in_selection` and `undo_redo` are that case since level
/// 0 shares the composite's plane (#1152): their absolute peaks are unchanged
/// or lower while their setups hold one 4250² RGBA plane less. Record the
/// measured delta and keep the absolute peak in mind.
const BASELINE: &[(&str, usize)] = &[
    ("new_document", 180_644_790),
    ("add_layer", 90_313_011),
    ("edit_fill", 15_047_480),
    ("bucket_in_selection", 16_298_976),
    ("bucket_whole_layer", 218_450_072),
    ("brush_stroke", 110_827_969),
    ("undo_redo", 264_624_776),
    ("snapshot_restore", 482_699_518),
    ("move_layer", 5_815_545),
    ("free_transform", 36_187_521),
    ("save_psd", 291_159_444),
    ("open_psd", 521_690_071),
];

/// The peak heap `run` adds over what `setup` left, with both in one pool of
/// [`THREADS`] threads. The pool exists before the reset, so its own
/// bookkeeping is not counted.
fn measure<T>(setup: impl FnOnce() -> T + Send, run: impl FnOnce(&mut T) + Send) -> usize {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(THREADS)
        .build()
        .expect("a measurement pool");
    pool.install(|| {
        let mut state = setup();
        let base = crate::HEAP.current();
        crate::HEAP.reset();
        run(&mut state);
        let peak = crate::HEAP.peak().saturating_sub(base);
        drop(state);
        peak
    })
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// The counter is process-wide, so a guard is exact only alone in its
/// process: nextest runs each test in its own; plain `cargo test` shares one.
fn alone_in_process(name: &str) -> bool {
    let alone = std::env::var_os("NEXTEST").is_some();
    if !alone {
        println!("SKIP {name}: the heap guards need one test per process (cargo nextest)");
    }
    alone
}

fn guard<T>(name: &str, setup: impl FnOnce() -> T + Send, run: impl FnOnce(&mut T) + Send) {
    if !alone_in_process(name) {
        return;
    }
    let baseline = BASELINE
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, bytes)| *bytes)
        .expect("a recorded baseline");
    let peak = measure(setup, run);
    let limit = baseline + baseline / 10;
    println!(
        "{name} at {GUARD_SIDE}²: heap peak {:.1} MiB ({peak} B), baseline {:.1} MiB",
        mib(peak),
        mib(baseline)
    );
    assert!(
        peak <= limit,
        "{name}: heap peak {peak} B is above the baseline {baseline} B x 1.10 ({limit} B); \
         a setup that retains less raises this delta, so check the scenario's absolute peak too"
    );
}

fn report<T>(name: &str, setup: impl FnOnce() -> T + Send, run: impl FnOnce(&mut T) + Send) {
    if !alone_in_process(name) {
        return;
    }
    let peak = measure(setup, run);
    println!(
        "{name} at {FULL_SIDE}²: heap peak {:.1} MiB ({peak} B)",
        mib(peak)
    );
}

fn view(side: u32) -> PictureViewRust {
    sc::new_document(side)
}

fn with_layer(side: u32) -> PictureViewRust {
    let mut view = view(side);
    sc::add_layer(&mut view);
    view
}

fn with_marquee(side: u32) -> PictureViewRust {
    let mut view = with_layer(side);
    sc::marquee(&mut view, side);
    view
}

/// M9 and M10's start: the square bucket-filled on Layer 1, deselected.
fn with_square(side: u32) -> PictureViewRust {
    let mut view = with_marquee(side);
    sc::bucket(&mut view);
    view.session.set_selection(None);
    view
}

/// M7's start: the M3–M6 gestures in one history.
fn with_history(side: u32) -> PictureViewRust {
    let mut view = with_marquee(side);
    sc::fill_background(&mut view);
    sc::bucket(&mut view);
    view.session.set_selection(None);
    sc::bucket_at(&mut view, 4);
    sc::brush_stroke(&mut view, side);
    view
}

fn with_stroke(side: u32) -> PictureViewRust {
    let mut view = with_layer(side);
    sc::brush_stroke(&mut view, side);
    view
}

// M1
#[test]
fn new_document_heap_peak() {
    guard("new_document", || (), |_| drop(view(GUARD_SIDE)));
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn new_document_heap_peak_full() {
    report("new_document", || (), |_| drop(view(FULL_SIDE)));
}

// M2
#[test]
fn add_layer_heap_peak() {
    guard("add_layer", || view(GUARD_SIDE), sc::add_layer);
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn add_layer_heap_peak_full() {
    report("add_layer", || view(FULL_SIDE), sc::add_layer);
}

// M3
#[test]
fn edit_fill_heap_peak() {
    guard(
        "edit_fill",
        || with_marquee(GUARD_SIDE),
        sc::fill_background,
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn edit_fill_heap_peak_full() {
    report("edit_fill", || with_marquee(FULL_SIDE), sc::fill_background);
}

// M4
#[test]
fn bucket_in_selection_heap_peak() {
    guard(
        "bucket_in_selection",
        || with_marquee(GUARD_SIDE),
        sc::bucket,
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn bucket_in_selection_heap_peak_full() {
    report(
        "bucket_in_selection",
        || with_marquee(FULL_SIDE),
        sc::bucket,
    );
}

// M5
#[test]
fn bucket_whole_layer_heap_peak() {
    guard("bucket_whole_layer", || with_layer(GUARD_SIDE), sc::bucket);
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn bucket_whole_layer_heap_peak_full() {
    report("bucket_whole_layer", || with_layer(FULL_SIDE), sc::bucket);
}

// M6
#[test]
fn brush_stroke_heap_peak() {
    guard(
        "brush_stroke",
        || with_layer(GUARD_SIDE),
        |v| sc::brush_stroke(v, GUARD_SIDE),
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn brush_stroke_heap_peak_full() {
    report(
        "brush_stroke",
        || with_layer(FULL_SIDE),
        |v| sc::brush_stroke(v, FULL_SIDE),
    );
}

// M7
#[test]
fn undo_redo_heap_peak() {
    guard("undo_redo", || with_history(GUARD_SIDE), sc::undo_redo);
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn undo_redo_heap_peak_full() {
    report("undo_redo", || with_history(FULL_SIDE), sc::undo_redo);
}

// M8
#[test]
fn snapshot_restore_heap_peak() {
    guard(
        "snapshot_restore",
        || with_stroke(GUARD_SIDE),
        sc::snapshot_restore,
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn snapshot_restore_heap_peak_full() {
    report(
        "snapshot_restore",
        || with_stroke(FULL_SIDE),
        sc::snapshot_restore,
    );
}

// M9
#[test]
fn move_layer_heap_peak() {
    guard(
        "move_layer",
        || with_square(GUARD_SIDE),
        |v| sc::move_layer(v, GUARD_SIDE),
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn move_layer_heap_peak_full() {
    report(
        "move_layer",
        || with_square(FULL_SIDE),
        |v| sc::move_layer(v, FULL_SIDE),
    );
}

// M10
#[test]
fn free_transform_heap_peak() {
    guard(
        "free_transform",
        || with_square(GUARD_SIDE),
        sc::free_transform,
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn free_transform_heap_peak_full() {
    report(
        "free_transform",
        || with_square(FULL_SIDE),
        sc::free_transform,
    );
}

// M11
#[test]
fn save_psd_heap_peak() {
    let file = sc::TempFile::new("save");
    guard(
        "save_psd",
        || with_square(GUARD_SIDE),
        |v| sc::save_psd(v, &file),
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn save_psd_heap_peak_full() {
    let file = sc::TempFile::new("save");
    report(
        "save_psd",
        || with_square(FULL_SIDE),
        |v| sc::save_psd(v, &file),
    );
}

// M12
#[test]
fn open_psd_heap_peak() {
    let file = sc::TempFile::new("open");
    guard(
        "open_psd",
        || sc::write_fixture(GUARD_SIDE, &file),
        |path| sc::open_psd(path),
    );
}

#[test]
#[ignore = "17000x17000 heap peak; run explicitly with --run-ignored only"]
fn open_psd_heap_peak_full() {
    let file = sc::TempFile::new("open");
    report(
        "open_psd",
        || sc::write_fixture(FULL_SIDE, &file),
        |path| sc::open_psd(path),
    );
}

/// Each memory gate's estimate (#1211) covers the heap peak its scenario
/// measures at 4250², so the governor refuses a path before the path can
/// outgrow the room it was given.
#[test]
fn estimates_cover_the_measured_peaks() {
    use crate::cxxqt_object::helpers::memory_gate as gate;
    use pictura_core::BitDepth;
    if !alone_in_process("estimates_cover_the_measured_peaks") {
        return;
    }
    let side = GUARD_SIDE;
    let doc = |view: &PictureViewRust| view.session.doc.clone().expect("document");
    let check = |name: &str, peak: usize, estimate: u64| {
        println!(
            "{name}: heap peak {:.1} MiB, estimate {:.1} MiB",
            mib(peak),
            mib(estimate as usize)
        );
        assert!(
            peak as u64 <= estimate,
            "{name}: heap peak {peak} B is above its estimate {estimate} B"
        );
    };
    let file = sc::TempFile::new("estimate");

    let peak = measure(|| (), |_| drop(view(side)));
    check(
        "new_document",
        peak,
        gate::new_document_bytes(side, side, 3, BitDepth::Eight),
    );

    let estimate = gate::add_layer_bytes(&doc(&view(side)));
    check("add_layer", measure(|| view(side), sc::add_layer), estimate);

    let estimate = gate::brush_bytes(&doc(&with_layer(side)), "1");
    let peak = measure(|| with_layer(side), |v| sc::brush_stroke(v, side));
    check("brush_stroke", peak, estimate);

    let estimate = gate::restore_bytes(&doc(&with_history(side)));
    check(
        "undo_redo",
        measure(|| with_history(side), sc::undo_redo),
        estimate,
    );

    let start = doc(&with_stroke(side));
    let estimate = gate::snapshot_bytes(&start) + gate::snapshot_restore_bytes(&start);
    drop(start);
    let peak = measure(|| with_stroke(side), sc::snapshot_restore);
    check("snapshot_restore", peak, estimate);

    let estimate = gate::save_bytes(&doc(&with_square(side)));
    let peak = measure(|| with_square(side), |v| sc::save_psd(v, &file));
    check("save_psd", peak, estimate);
}
