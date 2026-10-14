//! ImageMagick differential oracle for the `pictura_select` modify ops
//! (task M5-B).
//!
//! ImageMagick implements grayscale morphology and Gaussian blur, which is the
//! mathematical core of `Selection::{expand,contract,feather}`. This is a
//! *sanity* oracle, not a parity oracle: the reference does not publish the structuring
//! element (square vs. Euclidean disk), the operation ordering, or the feather
//! radius→sigma mapping, and `pictura_select` is implemented concurrently by
//! task M5-A. The differential rows are therefore `#[ignore]`d until M5-A
//! lands; the ImageMagick-independent property tests below the same gate.
//!
//! Only the oracle script itself is exercised by active tests. The
//! operator→ImageMagick mapping, the flags and the tolerances are in
//! `tests/README.md`.
//!
//! Regenerate/inspect a result manually with `scripts/select_oracle.py`; see
//! `tests/README.md`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_select::{SelectOp, Selection};

/// Side length of the raw 8-bit grayscale test mask.
const SIZE: u32 = 8;

/// One row of the `Selection` op -> ImageMagick mapping table. `im` is the
/// equivalent operator (documentation; the actual invocation lives with each
/// test) and `tolerance` is the absolute per-sample allowance for
/// [`compare`].
struct Mapping {
    op: &'static str,
    im: &'static str,
    tolerance: u8,
    note: &'static str,
}

const MAPPING: &[Mapping] = &[
    Mapping {
        op: "expand(r)",
        im: "-morphology Dilate Square:r",
        tolerance: 0,
        note:
            "expand/contract uses a separable box element; IM Square:r is (2r+1)^2. Disk:r diverges at corners",
    },
    Mapping {
        op: "contract(r)",
        im: "-morphology Erode Square:r",
        tolerance: 0,
        note: "as expand; IM clamps at the canvas edge like PS's edge exemption",
    },
    Mapping {
        op: "feather(r)",
        im: "-gaussian-blur 0x(r/2)",
        tolerance: 0,
        note: "sigma=r/2 matches the selection feather's separable Gaussian on the 8x8 mask; \
               the differential pins r=2, the exact narrow-kernel path (sigma >= 2 uses the \
               variance-matched box stack)",
    },
    Mapping {
        op: "smooth(r)",
        im: "-morphology Smooth Square:r",
        tolerance: 32,
        note: "IM Smooth is a mean, PS Smooth is a majority vote; shape only",
    },
    Mapping {
        op: "invert",
        im: "-negate",
        tolerance: 0,
        note: "identical M-v per sample",
    },
];

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/select_oracle.py")
}

fn magick_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("magick")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Unique scratch directory per call so tests can run in parallel.
fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-select-oracle-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Compare two byte buffers with an absolute per-sample tolerance. Mirrors
/// `pictura_testkit::compare`, inlined because M5-B owns only `tests/**` and
/// may not add a dev-dependency to the crate manifest.
struct Diff {
    samples: usize,
    differing: usize,
    max_delta: u16,
}

fn compare(a: &[u8], b: &[u8], tolerance: u8) -> Diff {
    assert_eq!(a.len(), b.len(), "length mismatch");
    let mut differing = 0usize;
    let mut max_delta = 0u16;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = x.abs_diff(*y);
        if d > tolerance {
            differing += 1;
        }
        max_delta = max_delta.max(d as u16);
    }
    Diff {
        samples: a.len(),
        differing,
        max_delta,
    }
}

/// A 4x4 block centred in the 8x8 mask (expand/contract stay clear of the
/// canvas edge so IM's edge clamp and PS's exemption do not interact).
fn test_mask() -> Selection {
    let mut sel = Selection::none(SIZE, SIZE);
    for y in 2..6 {
        for x in 2..6 {
            sel.data[(y * SIZE + x) as usize] = 255;
        }
    }
    sel
}

/// A second mask for boolean identities: the same block shifted right/down.
fn shifted_mask() -> Selection {
    let mut sel = Selection::none(SIZE, SIZE);
    for y in 3..7 {
        for x in 3..7 {
            sel.data[(y * SIZE + x) as usize] = 255;
        }
    }
    sel
}

fn single_pixel_mask() -> Selection {
    let mut sel = Selection::none(SIZE, SIZE);
    let c = (SIZE / 2) as usize;
    sel.data[c * SIZE as usize + c] = 255;
    sel
}

/// Run `scripts/select_oracle.py apply` over `input` and return the raw mask.
fn run_oracle(extra: &[&str], input: &[u8]) -> Vec<u8> {
    let dir = scratch_dir("oracle");
    let input_path = dir.join("in.gray");
    let output_path = dir.join("out.gray");
    std::fs::write(&input_path, input).unwrap();
    let result = Command::new("python3")
        .arg(script())
        .arg("apply")
        .args(["--size", "8x8"])
        .args(extra)
        .arg(&input_path)
        .arg(&output_path)
        .output()
        .expect("run select_oracle.py apply");
    assert!(
        result.status.success(),
        "oracle failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&output_path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

/// Diff a `Selection` op against the ImageMagick oracle. Skips when `magick`
/// is absent.
fn differential<F: Fn(&Selection) -> Selection>(op: F, extra: &[&str], tolerance: u8, label: &str) {
    if !magick_available() {
        eprintln!("skipping {label}: `magick` not on PATH");
        return;
    }
    let mask = test_mask();
    let reference = run_oracle(extra, &mask.data);
    let got = op(&mask);
    let diff = compare(&got.data, &reference, tolerance);
    assert!(
        diff.differing == 0,
        "{label}: {} of {} samples over tolerance {tolerance} (max delta {})",
        diff.differing,
        diff.samples,
        diff.max_delta
    );
}

#[test]
fn reference_script_is_present() {
    let path = script();
    assert!(
        path.is_file(),
        "missing oracle script at {}",
        path.display()
    );
    let help = Command::new("python3")
        .arg(&path)
        .arg("apply")
        .arg("--help")
        .output()
        .expect("run select_oracle.py apply --help");
    assert!(
        help.status.success(),
        "`apply --help` failed:\n{}",
        String::from_utf8_lossy(&help.stderr)
    );
}

#[test]
fn imagemagick_runs() {
    if !magick_available() {
        eprintln!("skipping: `magick` not on PATH");
        return;
    }
    let out = Command::new("python3")
        .arg(script())
        .arg("version")
        .output()
        .expect("run select_oracle.py version");
    assert!(
        out.status.success(),
        "`version` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("ImageMagick"),
        "unexpected version output:\n{text}"
    );
}

/// The oracle itself, independent of `pictura_select`: `-negate` must produce
/// `255 - v` for every sample (also proves the raw grayscale round-trip).
#[test]
fn oracle_negate_matches_expected_bytes() {
    if !magick_available() {
        eprintln!("skipping: `magick` not on PATH");
        return;
    }
    let input = test_mask().data;
    let output = run_oracle(&["--im-args=-negate"], &input);
    assert_eq!(output.len(), input.len());
    let expected: Vec<u8> = input.iter().map(|v| 255 - v).collect();
    assert_eq!(output, expected);
}

/// `Disk:1` dilation of a single selected pixel yields exactly its 5-pixel
/// four-connected cross (proves the morphology flags and kernel shape).
#[test]
fn oracle_dilate_grows_a_single_pixel() {
    if !magick_available() {
        eprintln!("skipping: `magick` not on PATH");
        return;
    }
    let output = run_oracle(
        &["--op", "dilate", "--radius", "1"],
        &single_pixel_mask().data,
    );
    let selected: Vec<usize> = output
        .iter()
        .enumerate()
        .filter(|(_, &v)| v == 255)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        selected.len(),
        5,
        "Disk:1 cross is 5 pixels, got {selected:?}"
    );
    assert!(
        output.iter().all(|&v| v == 0 || v == 255),
        "binary mask must stay binary under morphology"
    );
}

#[test]
fn mapping_documents_ops_and_tolerances() {
    assert_eq!(MAPPING.len(), 5, "one row per mapped operation");
    for m in MAPPING {
        assert!(!m.op.is_empty() && !m.im.is_empty(), "empty mapping row");
        assert!(!m.note.is_empty(), "{}: empty note", m.op);
        assert!(m.tolerance <= 32, "{}: tolerance out of range", m.op);
    }
}

#[test]
fn expand_matches_imagemagick_dilate() {
    differential(
        |s| s.expand(1),
        &["--op", "dilate", "--kernel", "Square:1"],
        0,
        "Expand r=1",
    );
}

#[test]
fn contract_matches_imagemagick_erode() {
    differential(
        |s| s.contract(1),
        &["--op", "erode", "--kernel", "Square:1"],
        0,
        "Contract r=1",
    );
}

#[test]
fn feather_matches_imagemagick_gaussian() {
    // The oracle blurs with sigma = radius / 2 (the spec's inferred mapping).
    differential(
        |s| s.feather(2.0),
        &["--op", "gaussian", "--sigma", "1.0"],
        0,
        "Feather r=2",
    );
}

#[test]
fn invert_is_an_involution() {
    let mask = test_mask();
    assert_eq!(mask.invert().invert(), mask);
    assert_eq!(
        Selection::all(SIZE, SIZE).invert(),
        Selection::none(SIZE, SIZE)
    );
}

#[test]
fn boolean_identities() {
    let a = test_mask();
    let b = shifted_mask();
    let none = Selection::none(SIZE, SIZE);
    let all = Selection::all(SIZE, SIZE);

    let combined = |base: &Selection, other: &Selection, op: SelectOp| {
        let mut out = base.clone();
        out.combine(other, op).expect("combine");
        out
    };

    assert_eq!(
        combined(&a, &b, SelectOp::Add),
        combined(&b, &a, SelectOp::Add),
        "union is commutative"
    );
    assert_eq!(combined(&a, &a, SelectOp::Add), a, "union is idempotent");
    assert_eq!(
        combined(&a, &none, SelectOp::Add),
        a,
        "none is union identity"
    );
    assert_eq!(
        combined(&a, &all, SelectOp::Intersect),
        a,
        "all is intersection identity"
    );
    assert_eq!(
        combined(&a, &none, SelectOp::Intersect),
        none,
        "none annihilates intersection"
    );
    assert_eq!(
        combined(&a, &none, SelectOp::Subtract),
        a,
        "subtracting none is identity"
    );
    assert_eq!(
        combined(&a, &b, SelectOp::Replace),
        b,
        "replace copies other"
    );
    assert_eq!(
        combined(&a, &b, SelectOp::Subtract),
        combined(&a, &b.invert(), SelectOp::Intersect),
        "a - b == a intersect !b"
    );
    assert_eq!(
        combined(&a, &b, SelectOp::Intersect).invert(),
        combined(&a.invert(), &b.invert(), SelectOp::Add),
        "De Morgan: !(a & b) == !a | !b"
    );
}
