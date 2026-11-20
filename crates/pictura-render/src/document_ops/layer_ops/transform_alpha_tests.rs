//! A transform resamples colour weighted by alpha (#1446): erased pixels and
//! the transparent padding past the layer edge lend no colour to their
//! neighbours. A retained store left stale by an erase does not bring the
//! erased pixels back.

use pictura_core::{BitDepth, Document, Plane, Samples, SourceChannels, TileSample};

use super::kernel::Interpolation;
use super::paths::resolve_path;
use super::transform::{transform_layer_with, LayerTransform};
use super::transform_layer_warp;
use pictura_core::warp::Warp;

const YELLOW: [u8; 3] = [242, 238, 196];
const SIDE: u32 = 48;

/// An opaque yellow layer whose right third and a 6×6 hole are erased the way
/// the Clear-mode eraser leaves them: alpha 0, colour black.
fn erased_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let erased = x >= 32 || ((10..16).contains(&x) && (20..26).contains(&y));
            if erased {
                rgba.extend([0, 0, 0, 0]);
            } else {
                rgba.extend([YELLOW[0], YELLOW[1], YELLOW[2], 255]);
            }
        }
    }
    rgba
}

fn every<T: TileSample>(plane: &Plane<T>) -> Vec<T> {
    let mut out = vec![T::default(); plane.len()];
    plane.read_into(0, &mut out);
    out
}

fn transform(scale: f64, degrees: f64) -> LayerTransform {
    LayerTransform {
        scale_x: scale,
        scale_y: scale,
        angle_radians: degrees.to_radians(),
        dx: 0.0,
        dy: 0.0,
    }
}

/// Every pixel with non-zero alpha keeps the yellow, within one level; returns
/// how many such pixels were partly transparent.
fn assert_keeps_yellow(doc: &Document, what: &str) -> usize {
    let layer = resolve_path(doc, "0").expect("layer");
    let plane = |id: i16| {
        every(
            &layer
                .channels()
                .iter()
                .find(|c| c.id == id)
                .expect("channel")
                .data,
        )
    };
    let (alpha, rgb) = (plane(-1), [plane(0), plane(1), plane(2)]);
    let mut partial = 0;
    for (i, a) in alpha.iter().enumerate().filter(|(_, a)| **a > 0) {
        partial += usize::from(*a < 255);
        for (c, want) in rgb.iter().zip(YELLOW) {
            assert!(
                c[i].abs_diff(want) <= 1,
                "{what}: pixel {i} alpha {a} has {} for {want}",
                c[i]
            );
        }
    }
    partial
}

#[test]
fn a_scaled_erased_edge_keeps_its_colour() {
    for interpolation in [
        Interpolation::Bilinear,
        Interpolation::Bicubic,
        Interpolation::BicubicSmoother,
        Interpolation::BicubicSharper,
    ] {
        let mut doc = Document::from_rgba("erased", SIDE, SIDE, &erased_rgba());
        assert!(transform_layer_with(
            &mut doc,
            "0",
            transform(2.5, 0.0),
            interpolation
        ));
        let partial = assert_keeps_yellow(&doc, &format!("{interpolation:?}"));
        assert!(
            partial > 0,
            "{interpolation:?}: the erased edge is not soft"
        );
    }
}

#[test]
fn a_rotated_edge_keeps_its_colour() {
    let rgba: Vec<u8> = (0..SIDE * SIDE)
        .flat_map(|_| [YELLOW[0], YELLOW[1], YELLOW[2], 255])
        .collect();
    let mut doc = Document::from_rgba("opaque", SIDE, SIDE, &rgba);
    assert!(transform_layer_with(
        &mut doc,
        "0",
        transform(1.0, 30.0),
        Interpolation::Bilinear
    ));
    assert!(
        assert_keeps_yellow(&doc, "rotated") > 0,
        "no anti-aliased edge"
    );
}

#[test]
fn a_scaled_native_store_keeps_its_colour() {
    let rgba = erased_rgba();
    let mut doc = Document::from_rgba("erased", SIDE, SIDE, &rgba);
    let widen = |k: usize| -> Samples {
        Samples::U16(
            rgba.chunks(4)
                .map(|p| u16::from(p[k]) * 257)
                .collect::<Vec<_>>()
                .into(),
        )
    };
    let layer = &mut doc.layers[0];
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![(0, widen(0)), (1, widen(1)), (2, widen(2)), (-1, widen(3))],
    ));
    assert!(transform_layer_with(
        &mut doc,
        "0",
        transform(2.5, 0.0),
        Interpolation::Bilinear
    ));
    assert_keeps_yellow(&doc, "8-bit after a 16-bit scale");
    let layer = resolve_path(&doc, "0").expect("layer");
    let store = layer.source_channels.as_ref().expect("the store survives");
    let native = |id: i16| -> Vec<u16> {
        match &store
            .planes
            .iter()
            .find(|(p, _)| *p == id)
            .expect("plane")
            .1
        {
            Samples::U16(v) => every(v),
            _ => panic!("expected u16 samples"),
        }
    };
    let alpha = native(-1);
    for (k, want) in YELLOW.iter().enumerate() {
        let color = native(k as i16);
        let channel = every(
            &layer
                .channels()
                .iter()
                .find(|c| c.id == k as i16)
                .expect("channel")
                .data,
        );
        for (i, a) in alpha.iter().enumerate().filter(|(_, a)| **a > 0) {
            let unit = f64::from(color[i]) / 65535.0;
            assert!(
                (unit - f64::from(*want) / 255.0).abs() <= 1.0 / 255.0,
                "channel {k} pixel {i} alpha {a}: {}",
                color[i]
            );
            assert_eq!(channel[i], (color[i] >> 8) as u8, "8-bit is the narrowing");
        }
    }
}

/// An opaque yellow layer with a 16-bit store, then a 12×12 hole erased in the
/// 8-bit planes only, the way a brush stroke leaves the store behind.
fn erased_after_store() -> Document {
    let opaque: Vec<u8> = (0..SIDE * SIDE)
        .flat_map(|_| [YELLOW[0], YELLOW[1], YELLOW[2], 255])
        .collect();
    let mut doc = Document::from_rgba("stale", SIDE, SIDE, &opaque);
    let widen = |v: u8| -> Samples {
        Samples::widen_from_u8(&vec![v; (SIDE * SIDE) as usize], BitDepth::Sixteen)
    };
    let layer = &mut doc.layers[0];
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![
            (0, widen(YELLOW[0])),
            (1, widen(YELLOW[1])),
            (2, widen(YELLOW[2])),
            (-1, widen(255)),
        ],
    ));
    for channel in layer.channels_mut() {
        let data = channel.data.as_mut_slice();
        for y in 18..30 {
            for x in 18..30 {
                data[(y * SIDE + x) as usize] = 0;
            }
        }
    }
    doc
}

/// The erased hole survives as fully transparent pixels, and the store still
/// narrows to the 8-bit planes.
fn assert_hole_stays_erased(doc: &Document, what: &str) {
    let layer = resolve_path(doc, "0").expect("layer");
    let alpha = every(
        &layer
            .channels()
            .iter()
            .find(|c| c.id == -1)
            .expect("alpha")
            .data,
    );
    let clear = alpha.iter().filter(|a| **a == 0).count();
    assert!(clear >= 400, "{what}: only {clear} erased pixels remain");
    let store = layer.source_channels.as_ref().expect("the store survives");
    for (id, samples) in &store.planes {
        let channel = layer
            .channels()
            .iter()
            .find(|c| c.id == *id)
            .expect("channel");
        assert_eq!(
            samples.narrow_to_u8(),
            every(&channel.data),
            "{what}: plane {id} narrows to its 8-bit channel"
        );
    }
}

#[test]
fn a_scale_after_an_erase_keeps_the_hole() {
    let mut doc = erased_after_store();
    assert!(transform_layer_with(
        &mut doc,
        "0",
        transform(2.5, 0.0),
        Interpolation::Bilinear
    ));
    assert_hole_stays_erased(&doc, "scale");
}

#[test]
fn a_warp_after_an_erase_keeps_the_hole() {
    let mut doc = erased_after_store();
    let side = f64::from(SIDE);
    let quad = [
        (0.0, 0.0),
        (2.0 * side, 0.0),
        (2.0 * side, 2.0 * side),
        (0.0, 2.0 * side),
    ];
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &Warp::none([0.0, 0.0, side, side]),
        Some(quad),
        Interpolation::Bilinear
    ));
    assert_hole_stays_erased(&doc, "warp");
}
