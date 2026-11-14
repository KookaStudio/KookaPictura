//! Clipping groups in the CPU compositor (`composite_clipping`).

use super::smart_object::{embedded, payload_of, smart_layer};
use super::smart_object_alpha::payload_with_alpha;
use super::*;
use crate::composite::composite_rgba_region;
use pictura_core::{Channel, SmartObject};

const WHITE: [u8; 3] = [255, 255, 255];
const GREEN: [u8; 3] = [0, 255, 0];

const VSMS_FIXTURE: &[u8] =
    include_bytes!("../../../pictura-codec/tests/fixtures/vector_mask_vsms.psd");

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

/// An Invert clipped to a red base on a transparent canvas, with the base's
/// alpha and mask set by `base`.
fn inverted_base(alpha: u8, mask: Option<LayerMask>) -> Document {
    let mut base = solid(
        "Base",
        rect(2, 2, 6, 6),
        (255, 0, 0),
        alpha,
        BlendMode::Normal,
        255,
    );
    base.mask = mask;
    let invert = Layer {
        name: "Invert".into(),
        rect: rect(0, 0, 0, 0),
        opacity: 255,
        fill: 255,
        visible: true,
        clipping: true,
        adjustment: Some(crate::encode_invert()),
        ..Default::default()
    };
    doc(8, 8, vec![base, invert])
}

fn half_mask() -> LayerMask {
    LayerMask {
        rect: rect(2, 2, 6, 6),
        data: Some(vec![128u8; 16].into()),
        ..Default::default()
    }
}

#[test]
fn clipped_adjustment_keeps_a_soft_base_alpha() {
    let d = inverted_base(128, None);
    let out = px(&composite_rgba(&d), 3, 3);
    assert_eq!(&out[..3], &[0, 255, 255], "fully inverted");
    assert!(out[3].abs_diff(128) <= 1, "alpha unchanged: {}", out[3]);
    assert_eq!(px(&composite_rgba(&d), 1, 1)[3], 0, "outside the base");
}

#[test]
fn clipped_adjustment_follows_a_half_mask() {
    let d = inverted_base(255, Some(half_mask()));
    let out = px(&composite_rgba(&d), 3, 3);
    assert_eq!(&out[..3], &[0, 255, 255], "fully inverted");
    assert!(out[3].abs_diff(128) <= 1, "alpha unchanged: {}", out[3]);
}

#[test]
fn clipped_adjustment_region_matches_full() {
    let _guard = crate::composite_rows::serial_test_lock();
    let d = inverted_base(128, Some(half_mask()));
    let full_out = composite_rgba(&d);
    let forced = &crate::composite_rows::FORCE_SERIAL;
    forced.store(true, std::sync::atomic::Ordering::Relaxed);
    let serial = composite_rgba_region(&d, 1, 1, 6, 6);
    forced.store(false, std::sync::atomic::Ordering::Relaxed);
    let parallel = composite_rgba_region(&d, 1, 1, 6, 6);
    for y in 0..6 {
        for x in 0..6 {
            assert_eq!(px(&serial, x, y), px(&full_out, x + 1, y + 1));
            assert_eq!(px(&parallel, x, y), px(&full_out, x + 1, y + 1));
        }
    }
}

/// A white background, a smart-object base over the full canvas, and a
/// full-canvas green layer clipped to it.
fn smart_base_doc(so: SmartObject, channels: Vec<Channel>) -> Document {
    let base = smart_layer("Base", full(4, 4), so, channels);
    let mut green = solid(
        "Green",
        full(4, 4),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    green.clipping = true;
    doc(
        4,
        4,
        vec![
            solid(
                "Background",
                full(4, 4),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            base,
            green,
        ],
    )
}

/// A 4x4 red proxy whose left half is opaque and right half transparent.
fn proxy_with_alpha() -> Vec<Channel> {
    let alpha: Vec<u8> = (0..16).map(|i| if i % 4 < 2 { 255 } else { 0 }).collect();
    vec![
        Channel {
            id: 0,
            data: vec![255; 16].into(),
        },
        Channel {
            id: 1,
            data: vec![0; 16].into(),
        },
        Channel {
            id: 2,
            data: vec![0; 16].into(),
        },
        Channel {
            id: -1,
            data: alpha.into(),
        },
    ]
}

#[test]
fn a_clipped_layer_respects_a_smart_object_bases_alpha() {
    // The 2x2 source scales to 4x4: columns 0-1 opaque, columns 2-3 transparent.
    let so = embedded(payload_with_alpha([[255, 0, 0]; 4], [255, 0, 255, 0]));
    let out = composite_rgba(&smart_base_doc(so, Vec::new()));
    assert_eq!(rgb(&out, 0, 0), GREEN, "inside the opaque half");
    assert_eq!(rgb(&out, 1, 3), GREEN);
    assert_eq!(rgb(&out, 2, 0), WHITE, "over the transparent half");
    assert_eq!(rgb(&out, 3, 3), WHITE);
}

#[test]
fn a_clipped_layer_respects_a_proxy_bases_alpha() {
    let so = embedded(payload_of([255, 0, 0]));
    let out = composite_rgba(&smart_base_doc(so, proxy_with_alpha()));
    assert_eq!(rgb(&out, 0, 0), GREEN, "inside the proxy's opaque half");
    assert_eq!(rgb(&out, 1, 3), GREEN);
    assert_eq!(rgb(&out, 2, 0), WHITE, "over the proxy's transparent half");
    assert_eq!(rgb(&out, 3, 3), WHITE);
}

fn vsms_clipped_doc() -> Document {
    let mut d = pictura_codec::read_psd(VSMS_FIXTURE).expect("vector_mask_vsms.psd parses");
    let mut green = solid(
        "Clipped",
        full(8, 8),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    green.clipping = true;
    let _ = d.layers.push(green);
    d
}

fn shape_index(d: &Document) -> usize {
    d.layers
        .iter()
        .position(|layer| layer.name == "Shape")
        .expect("Shape layer")
}

#[test]
fn a_clipped_layer_follows_a_vsms_shape_base() {
    let out = composite_rgba(&vsms_clipped_doc());
    assert_eq!(rgb(&out, 2, 2), GREEN, "inside the vsms rectangle");
    assert_eq!(
        rgb(&out, 0, 0),
        [200, 100, 50],
        "outside it shows the base layer"
    );
    assert_eq!(rgb(&out, 6, 6), [200, 100, 50]);
}

#[test]
fn a_clipped_layer_follows_a_vmsk_shape_base() {
    let vsms = composite_rgba(&vsms_clipped_doc());

    let mut d = vsms_clipped_doc();
    let index = shape_index(&d);
    let data = {
        let block = d.layers[index]
            .extra_blocks
            .iter_mut()
            .find(|block| block.key == *b"vsms")
            .expect("Shape vsms block");
        block.key = *b"vmsk";
        block.data.clone()
    };
    d.layers[index].vector_mask = pictura_codec::decode_vector_mask(&data, 8, 8);
    assert!(
        d.layers[index].vector_mask.is_some(),
        "the vmsk block decodes"
    );
    let vmsk = composite_rgba(&d);
    assert_eq!(vmsk.data, vsms.data, "vmsk and vsms clip identically");
}

#[test]
fn a_disabled_vsms_mask_leaves_the_base_alpha() {
    let mut d = vsms_clipped_doc();
    let index = shape_index(&d);
    let path = index.to_string();
    d.layers[index].rect = full(8, 8);
    assert!(crate::set_vector_mask_enabled(&mut d, &path, false));
    assert!(crate::vector_mask_disabled(&d, &path));

    let out = composite_rgba(&d);
    assert_eq!(
        rgb(&out, 0, 0),
        GREEN,
        "the fill base's unmasked alpha now reveals the clipped layer"
    );
    assert_eq!(rgb(&out, 7, 7), GREEN);
}
