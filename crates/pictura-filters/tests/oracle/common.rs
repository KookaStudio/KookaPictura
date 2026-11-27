//! Shared constants and helpers for the ImageMagick oracle tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_core::PixelBuffer;
use pictura_filters::{apply, Filter};
use pictura_testkit::{compare, oracle, Diff};

/// Side length of the raw planar RGB8 test image.
pub(crate) const SIZE: u32 = 16;
pub(crate) const CHANNELS: u8 = 3;

pub(crate) fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/filter_oracle.py")
}

/// Unique scratch directory per call so tests can run in parallel.
pub(crate) fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-filters-oracle-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Deterministic `SIZE`x`SIZE` RGB8 test image: two ramps plus a hard block
/// checker so every filter sees both gradients and edges.
pub(crate) fn test_image_interleaved() -> Vec<u8> {
    let mut data = Vec::with_capacity((SIZE * SIZE * 3) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let r = (x * 16) as u8;
            let g = (y * 16) as u8;
            let b = if (x / 4 + y / 4) % 2 == 0 { 40 } else { 210 };
            data.extend_from_slice(&[r, g, b]);
        }
    }
    data
}

/// Interleaved RGB8 -> planar RGB (the `PixelBuffer` layout `apply` consumes).
pub(crate) fn planarize(interleaved: &[u8]) -> Vec<u8> {
    let pixels = (SIZE * SIZE) as usize;
    let mut planar = vec![0u8; interleaved.len()];
    for i in 0..pixels {
        for c in 0..CHANNELS as usize {
            planar[c * pixels + i] = interleaved[i * CHANNELS as usize + c];
        }
    }
    planar
}

pub(crate) fn test_image_planar() -> Vec<u8> {
    planarize(&test_image_interleaved())
}

/// The `SIZE`x`SIZE` test image as a `PixelBuffer`.
pub(crate) fn test_image_buffer() -> PixelBuffer {
    PixelBuffer {
        width: SIZE,
        height: SIZE,
        channels: CHANNELS,
        data: test_image_planar().into(),
    }
}

/// Run `scripts/filter_oracle.py apply` over the planar test image and return
/// the raw planar result. `extra` holds the operator arguments.
pub(crate) fn oracle(extra: &[&str], input_planar: &[u8]) -> Vec<u8> {
    let dir = scratch_dir("oracle");
    let input_path = dir.join("in.rgb");
    let output_path = dir.join("out.rgb");
    std::fs::write(&input_path, input_planar).unwrap();
    let result = oracle::Command::new("python3")
        .arg(script())
        .arg("apply")
        .args(["--size", "16x16", "--planar"])
        .args(extra)
        .arg(&input_path)
        .arg(&output_path)
        .output()
        .expect("run filter_oracle.py apply");
    assert!(
        result.status.success(),
        "oracle failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&output_path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

/// Apply `filter` and diff it against the ImageMagick oracle.
pub(crate) fn oracle_diff(filter: &Filter, extra: &[&str], tolerance: u8) -> Diff {
    let original = test_image_planar();
    let reference = oracle(extra, &original);
    let mut buf = PixelBuffer {
        width: SIZE,
        height: SIZE,
        channels: CHANNELS,
        data: original.into(),
    };
    apply(filter, &mut buf).expect("pictura_filters::apply");
    compare(&buf.data, &reference, tolerance).expect("buffer lengths agree")
}

/// Diff `apply` against the ImageMagick oracle. Skips when `magick` is absent.
pub(crate) fn differential(filter: &Filter, extra: &[&str], tolerance: u8, label: &str) {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let diff = oracle_diff(filter, extra, tolerance);
    assert!(
        diff.is_empty(),
        "{label}: {} of {} samples over tolerance {tolerance} (max delta {})",
        diff.differing,
        diff.samples,
        diff.max_delta
    );
}

/// One horizontal planar RGB row from interleaved greys.
pub(crate) fn pixel_row(px: &[u8]) -> PixelBuffer {
    let n = px.len();
    let mut data = vec![0u8; n * 3];
    for (i, &v) in px.iter().enumerate() {
        data[i] = v;
        data[n + i] = v;
        data[2 * n + i] = v;
    }
    PixelBuffer {
        width: n as u32,
        height: 1,
        channels: 3,
        data: data.into(),
    }
}

pub(crate) fn plane_range(buf: &PixelBuffer) -> u8 {
    let plane = &buf.data[..buf.pixel_count()];
    *plane.iter().max().unwrap() - *plane.iter().min().unwrap()
}
