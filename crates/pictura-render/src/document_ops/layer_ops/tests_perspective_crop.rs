//! Perspective Crop tests.

use super::perspective_crop::{
    perspective_crop, perspective_crop_refusal, perspective_crop_size, perspective_crop_sized,
};
use super::tests::pixel_layer;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerBlock, LayerMask, PsdRect};

fn canvas(w: u32, h: u32) -> [(f64, f64); 4] {
    let (w, h) = (w as f64, h as f64);
    [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
}

/// A `w×h` document with one opaque layer of gray `value`.
fn doc(w: u32, h: u32, value: u8) -> Document {
    let mut d = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(vec![pixel_layer("base", w, h, value)]);
    d
}

fn set_gray(d: &mut Document, x: usize, y: usize, value: u8) {
    let w = d.layers[0].rect.width() as usize;
    for c in d.layers[0].channels.iter_mut().filter(|c| c.id >= 0) {
        c.data[y * w + x] = value;
    }
}

fn gray_at(d: &Document, x: usize, y: usize) -> u8 {
    d.layers[0].channels[0].data[y * d.width as usize + x]
}

#[test]
fn size_takes_the_longer_of_each_edge_pair() {
    // Top edge 60, bottom 100; both sides hypot(20, 50) ≈ 53.85.
    let quad = [(20.0, 0.0), (80.0, 0.0), (100.0, 50.0), (0.0, 50.0)];
    assert_eq!(perspective_crop_size(quad), (100, 54));
    assert_eq!(perspective_crop_size([(0.0, 0.0); 4]), (1, 1));
}

#[test]
fn an_explicit_output_size_wins() {
    // Auto would be 100×54; a manual size overrides it.
    let quad = [(20.0, 0.0), (80.0, 0.0), (100.0, 50.0), (0.0, 50.0)];
    let mut d = doc(128, 64, 255);
    assert!(perspective_crop_sized(&mut d, quad, 40, 20));
    assert_eq!((d.width, d.height), (40, 20));

    // Zero falls back to the automatic size.
    let mut auto = doc(128, 64, 255);
    assert!(perspective_crop_sized(&mut auto, quad, 0, 0));
    assert_eq!((auto.width, auto.height), perspective_crop_size(quad));
}

#[test]
fn the_whole_canvas_quad_is_the_identity() {
    let mut d = doc(16, 16, 255);
    set_gray(&mut d, 4, 4, 0);
    assert!(perspective_crop(&mut d, canvas(16, 16)));

    assert_eq!((d.width, d.height), (16, 16));
    assert_eq!(gray_at(&d, 4, 4), 0, "the marked pixel moved");
    assert_eq!((gray_at(&d, 0, 0), gray_at(&d, 15, 15)), (255, 255));
}

#[test]
fn a_skewed_region_is_straightened_and_becomes_the_canvas() {
    let mut d = doc(64, 64, 255);
    let quad = [(20.0, 10.0), (44.0, 10.0), (56.0, 50.0), (8.0, 50.0)];
    for y in 10..50usize {
        let t = (y - 10) as f64 / 40.0;
        let left = 20.0 + (8.0 - 20.0) * t;
        let right = 44.0 + (56.0 - 44.0) * t;
        for x in (left.ceil() as usize + 1)..(right.floor() as usize - 1) {
            set_gray(&mut d, x, y, 0);
        }
    }
    let (w, h) = perspective_crop_size(quad);
    assert!(perspective_crop(&mut d, quad));

    assert_eq!((d.width, d.height), (w, h));
    assert_eq!(
        d.layers[0].rect,
        PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32
        }
    );
    let (w, h) = (w as usize, h as usize);
    for x in [w / 4, w / 2, w * 3 / 4] {
        assert_eq!(gray_at(&d, x, h / 2), 0, "column {x} is not trapezoid");
    }
    d.composite = crate::composite_rgba(&d);
    assert_eq!(d.composite.width, w as u32, "composite not rebuilt");
}

#[test]
fn a_degenerate_quad_refuses_without_change() {
    let mut d = doc(8, 8, 90);
    let before = d.clone();
    let line = [(0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (15.0, 0.0)];
    assert!(!perspective_crop(&mut d, line));
    assert!(!perspective_crop(&mut d, [(3.0, 3.0); 4]));
    assert_eq!(d, before);
}

#[test]
fn outside_the_source_is_transparent_or_white_on_a_background() {
    let quad = [(-8.0, -8.0), (8.0, -8.0), (8.0, 8.0), (-8.0, 8.0)];
    let mut layered = doc(8, 8, 0);
    assert!(perspective_crop(&mut layered, quad));
    let alpha = &layered.layers[0].channels[3].data;
    assert_eq!((alpha[0], alpha[16 * 16 - 1]), (0, 255));

    let mut background = doc(8, 8, 0);
    background.layers[0].channels.retain(|c| c.id != -1);
    assert!(perspective_crop(&mut background, quad));
    assert_eq!(
        (gray_at(&background, 0, 0), gray_at(&background, 15, 15)),
        (255, 0)
    );
}

#[test]
fn masks_and_extra_channels_warp_with_the_pixels() {
    let mut d = doc(8, 8, 128);
    d.layers[0].mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 8,
        },
        default_color: 255,
        data: Some(vec![0; 32].into()),
        ..Default::default()
    });
    d.channels = vec![Channel {
        id: 0,
        data: (0..64).map(|i| if i < 32 { 255 } else { 0 }).collect(),
    }];
    assert!(perspective_crop(&mut d, canvas(8, 8)));

    let mask = d.layers[0].mask.as_ref().unwrap();
    assert_eq!(
        mask.rect,
        PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8
        }
    );
    let data = mask.data.as_ref().unwrap();
    assert_eq!(
        (data[2 * 8 + 3], data[6 * 8 + 3]),
        (0, 255),
        "mask default not kept outside"
    );
    assert_eq!((d.channels[0].data[8], d.channels[0].data[56]), (255, 0));
}

#[test]
fn live_type_and_high_depth_refuse() {
    let mut d = doc(8, 8, 10);
    d.layers[0].extra_blocks.push(LayerBlock {
        key: *b"TySh",
        data: Vec::new(),
    });
    assert!(perspective_crop_refusal(&d).is_some());
    let before = d.clone();
    assert!(!perspective_crop(&mut d, canvas(8, 8)));
    assert_eq!(d, before);
    assert!(perspective_crop_refusal(&doc(8, 8, 10)).is_none());
}
