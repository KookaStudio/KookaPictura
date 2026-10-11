//! Displace against ImageMagick's `-compose displace`, which reads the same
//! two map channels with the same sign: a light sample pulls the picture up
//! and to the left.

use std::sync::Arc;

use pictura_core::PixelBuffer;
use pictura_filters::{apply, DisplaceFit, DisplaceMap, Filter};
use pictura_testkit::{compare, Diff};

use crate::common::{magick_available, oracle, scratch_dir, test_image_planar, CHANNELS, SIZE};

/// A `SIZE`x`SIZE` map whose red and green planes ramp at different angles,
/// so both axes and both signs are exercised.
fn map_planes() -> (Vec<u8>, Vec<u8>) {
    let mut red = Vec::new();
    let mut green = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            red.push((x * 16 + y * 3) as u8);
            green.push((255 - y * 15 - x) as u8);
        }
    }
    (red, green)
}

/// The map as a binary PPM, the file ImageMagick reads.
fn write_map(path: &std::path::Path, red: &[u8], green: &[u8]) {
    let mut ppm = format!("P6\n{SIZE} {SIZE}\n255\n").into_bytes();
    for (r, g) in red.iter().zip(green) {
        ppm.extend_from_slice(&[*r, *g, 128]);
    }
    std::fs::write(path, ppm).unwrap();
}

/// Diff Pictura's Displace against ImageMagick at `tolerance`, or `None`
/// without `magick`.
fn displace_diff(scale: (f64, f64), tolerance: u8) -> Option<Diff> {
    if !magick_available() {
        return None;
    }
    let (red, green) = map_planes();
    let dir = scratch_dir("displace-map");
    let map_path = dir.join("map.ppm");
    write_map(&map_path, &red, &green);
    let scale_arg = format!("{}x{}", scale.0, scale.1);
    let extra = [
        "--op",
        "displace",
        "--map",
        map_path.to_str().unwrap(),
        "--displace-scale",
        &scale_arg,
    ];
    let original = test_image_planar();
    let reference = oracle(&extra, &original);
    let _ = std::fs::remove_dir_all(&dir);

    let map = DisplaceMap::new(SIZE, SIZE, red, green).unwrap();
    let mut buf = PixelBuffer {
        width: SIZE,
        height: SIZE,
        channels: CHANNELS,
        data: original.into(),
    };
    let filter = Filter::Displace {
        map: Arc::new(map),
        horizontal_scale: scale.0,
        vertical_scale: scale.1,
        fit: DisplaceFit::StretchToFit,
        wrap: false,
    };
    apply(&filter, &mut buf).expect("displace applies");
    Some(compare(&buf.data, &reference, tolerance).unwrap())
}

/// Repeat Edge Pixels matches within 5 levels at these scales, mean under 0.5.
/// ImageMagick's neutral grey is 127.5 against Pictura's 128, a shift of
/// `0.5 × scale / 100` px that the test image's 170-level checker edges turn
/// into the delta; it reaches 9 at 8x-6, past the table's cap of 8.
/// Wrap Around is not diffed: Pictura wraps the sample position but blends
/// the edge pixel with itself, where ImageMagick's tiled virtual pixels blend
/// across the seam (57 of 768 samples over 9 at 8x-6, max 125; measure with
/// the script's `--wrap`).
#[test]
fn displace_matches_imagemagick() {
    for scale in [(4.0, 4.0), (6.0, -5.0)] {
        let Some(diff) = displace_diff(scale, 8) else {
            eprintln!("skipping Displace: `magick` not on PATH");
            return;
        };
        assert!(
            diff.is_empty(),
            "Displace {scale:?}: {} of {} samples over tolerance 8 (max delta {})",
            diff.differing,
            diff.samples,
            diff.max_delta
        );
    }
}
