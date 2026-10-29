//! Clipping groups in the CPU compositor (`composite_clipping`).

use super::*;
use crate::composite::composite_rgba_region;

const WHITE: [u8; 3] = [255, 255, 255];
const GREEN: [u8; 3] = [0, 255, 0];

/// A white background, a red 4 x 4 base at (2, 2), and a full-canvas green
/// layer clipped to it.
fn clipped_doc() -> Document {
    let mut green = solid(
        "Green",
        full(10, 10),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    green.clipping = true;
    doc(
        10,
        10,
        vec![
            solid(
                "Background",
                full(10, 10),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "Base",
                rect(2, 2, 6, 6),
                (255, 0, 0),
                255,
                BlendMode::Normal,
                255,
            ),
            green,
        ],
    )
}

#[test]
fn a_clipped_layer_shows_only_over_its_base() {
    let out = composite_rgba(&clipped_doc());
    assert_eq!(rgb(&out, 3, 3), GREEN, "inside the base");
    assert_eq!(rgb(&out, 8, 8), WHITE, "outside the base");
    assert_eq!(rgb(&out, 1, 3), WHITE);
}

#[test]
fn the_base_opacity_fades_the_group_but_its_fill_does_not() {
    let mut d = clipped_doc();
    d.layers[1].opacity = 128;
    // The whole group (green over the base) at half strength over white.
    let half = rgb(&composite_rgba(&d), 3, 3);
    assert!(
        (126..=129).contains(&half[0]) && half[1] == 255 && (126..=129).contains(&half[2]),
        "half strength: {half:?}"
    );

    let mut d = clipped_doc();
    d.layers[1].fill = 0;
    let out = composite_rgba(&d);
    assert_eq!(rgb(&out, 3, 3), GREEN, "a base at Fill 0 still clips");
    assert_eq!(rgb(&out, 8, 8), WHITE);
}

#[test]
fn a_run_shares_the_base_and_a_bottom_or_hidden_base_behaves() {
    let mut d = clipped_doc();
    let mut blue = solid(
        "Blue",
        rect(0, 0, 10, 4),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    blue.clipping = true;
    let _ = d.layers.push(blue);
    let out = composite_rgba(&d);
    // Blue covers x < 4; the base covers x and y in 2..6.
    assert_eq!(
        rgb(&out, 3, 3),
        [0, 0, 255],
        "the second clipped layer, inside the base"
    );
    assert_eq!(rgb(&out, 5, 3), GREEN);
    assert_eq!(
        rgb(&out, 1, 3),
        WHITE,
        "blue outside the base is clipped away"
    );

    let mut d = clipped_doc();
    d.layers[1].visible = false;
    assert_eq!(
        rgb(&composite_rgba(&d), 3, 3),
        WHITE,
        "a hidden base hides its group"
    );

    let mut alone = clipped_doc();
    let _ = alone.layers.remove(&[0]);
    let _ = alone.layers.remove(&[0]);
    assert_eq!(
        rgb(&composite_rgba(&alone), 8, 8),
        GREEN,
        "no base: unclipped"
    );
}

#[test]
fn a_region_composite_matches_the_full_one() {
    let d = clipped_doc();
    let full_out = composite_rgba(&d);
    let region = composite_rgba_region(&d, 1, 1, 6, 6);
    for y in 0..6 {
        for x in 0..6 {
            assert_eq!(px(&region, x, y), px(&full_out, x + 1, y + 1));
        }
    }
}

#[test]
fn clipping_parallel_matches_serial() {
    use crate::composite_rows::{serial_test_lock, FORCE_SERIAL, PARALLEL_RUNS};
    use std::sync::atomic::Ordering;

    let _guard = serial_test_lock();
    let d = clipped_doc();
    FORCE_SERIAL.store(true, Ordering::Relaxed);
    let serial = composite_rgba(&d);
    FORCE_SERIAL.store(false, Ordering::Relaxed);
    let before = PARALLEL_RUNS.load(Ordering::Relaxed);
    let parallel = composite_rgba(&d);
    assert!(PARALLEL_RUNS.load(Ordering::Relaxed) > before);
    assert_eq!(
        serial.data, parallel.data,
        "clipping lerp differs between serial and parallel"
    );

    FORCE_SERIAL.store(true, Ordering::Relaxed);
    let serial = composite_rgba_region(&d, 1, 1, 6, 6);
    FORCE_SERIAL.store(false, Ordering::Relaxed);
    let parallel = composite_rgba_region(&d, 1, 1, 6, 6);
    assert_eq!(
        serial.data, parallel.data,
        "clipping region differs between serial and parallel"
    );
}
