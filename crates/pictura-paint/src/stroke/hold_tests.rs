use super::*;
use crate::spacing::SpacingMode;
use pictura_core::{Channel, ColorMode};

const SIDE: i32 = 64;
const WHITE: u8 = 255;

fn white_doc() -> Document {
    let mut doc = Document::new(SIDE as u32, SIDE as u32, ColorMode::Rgb, BitDepth::Eight);
    let _ = doc.layers.push(Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIDE,
            right: SIDE,
        },
        channels: [0, 1, 2, -1]
            .map(|id| Channel {
                id,
                data: vec![WHITE; (SIDE * SIDE) as usize].into(),
            })
            .into(),
        ..Default::default()
    });
    doc
}

fn black_cfg(opacity: u8, flow: u8) -> StrokeConfig {
    StrokeConfig {
        color: Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        diameter: 20,
        spacing: SpacingMode::Fixed(25),
        opacity,
        flow,
        ..StrokeConfig::default()
    }
}

fn press() -> StrokeSample {
    StrokeSample {
        x: 32.0,
        y: 32.0,
        pressure: 1.0,
    }
}

fn centre_red(doc: &Document) -> u8 {
    layer_rgba(&doc.layers[0]).data[(32 * SIDE + 32) as usize][0]
}

#[test]
fn hold_before_any_sample_paints_nothing() {
    let mut doc = white_doc();
    let before = doc.clone();
    let mut stroke = Stroke::begin_at(&doc, "0", black_cfg(100, 10)).expect("begin");
    assert!(!stroke.hold(&mut doc));
    assert!(doc == before);
}

#[test]
fn hold_builds_to_opacity_cap() {
    let mut doc = white_doc();
    let mut stroke = Stroke::begin_at(&doc, "0", black_cfg(50, 10)).expect("begin");
    stroke.sample(&mut doc, press());
    let first = centre_red(&doc);
    assert!(stroke.hold(&mut doc), "a held dab changed nothing");
    assert!(centre_red(&doc) < first, "coverage did not rise");
    for _ in 0..200 {
        stroke.hold(&mut doc);
    }
    let capped = centre_red(&doc);
    assert!(capped.abs_diff(128) <= 3, "held to {capped}, not the cap");
    assert!(
        !stroke.hold(&mut doc),
        "a dab past the cap reported a change"
    );
}

#[test]
fn hold_paints_nothing_at_flow_zero() {
    let mut doc = white_doc();
    let before = doc.clone();
    let mut stroke = Stroke::begin_at(&doc, "0", black_cfg(100, 0)).expect("begin");
    stroke.sample(&mut doc, press());
    assert!(!stroke.hold(&mut doc));
    assert!(doc == before);
}

#[test]
fn hold_matches_serial_raster() {
    let original = white_doc();
    let mut banded_doc = original.clone();
    let mut serial_doc = original.clone();
    let cfg = black_cfg(100, 10);
    let mut banded = Stroke::begin_at(&banded_doc, "0", cfg).expect("begin");
    let mut serial = Stroke::begin_at(&serial_doc, "0", cfg)
        .expect("begin")
        .serialized();
    banded.sample(&mut banded_doc, press());
    serial.sample(&mut serial_doc, press());
    for _ in 0..8 {
        assert_eq!(banded.hold(&mut banded_doc), serial.hold(&mut serial_doc));
        assert_eq!(banded.take_dirty(), serial.take_dirty());
    }
    assert!(banded_doc == serial_doc);
    assert!(banded_doc != original);
}

#[test]
fn hold_paints_art_history() {
    let mut doc = white_doc();
    let source = RgbaImage {
        width: SIDE,
        height: SIDE,
        data: vec![[200, 30, 30, 255]; (SIDE * SIDE) as usize],
    };
    let cfg = StrokeConfig {
        diameter: 4,
        ..StrokeConfig::default()
    };
    let options = ArtHistoryOptions::default();
    let mut stroke =
        Stroke::begin_art_history(&doc, "0", cfg, source.into(), options).expect("begin");
    stroke.sample(&mut doc, press());
    let first = doc.clone();
    assert!(stroke.hold(&mut doc), "the held dab scattered nothing");
    assert!(doc != first);
}

#[test]
fn pressure_scaled_applies_the_overrides() {
    let cfg = StrokeConfig {
        diameter: 100,
        flow: 80,
        opacity: 60,
        pressure_size: true,
        pressure_opacity: true,
        ..StrokeConfig::default()
    };
    let half = cfg.pressure_scaled(0.5);
    assert_eq!(half.diameter, 50);
    assert_eq!(half.flow, 40);
    assert_eq!(half.opacity, 30);

    let off = StrokeConfig {
        pressure_size: false,
        pressure_opacity: false,
        ..cfg
    };
    let half = off.pressure_scaled(0.5);
    assert_eq!(half.diameter, 100);
    assert_eq!(half.flow, 80);
    assert_eq!(half.opacity, 60);
}

fn pixel_red(doc: &Document, x: i32, y: i32) -> u8 {
    layer_rgba(&doc.layers[0]).data[(y * SIDE + x) as usize][0]
}

#[test]
fn pressure_size_scales_the_dab_footprint() {
    let cfg = StrokeConfig {
        pressure_size: true,
        ..black_cfg(100, 100)
    };
    let mut full = white_doc();
    let mut stroke = Stroke::begin_at(&full, "0", cfg).expect("begin");
    stroke.sample(&mut full, press());

    let mut half = white_doc();
    let mut stroke = Stroke::begin_at(&half, "0", cfg).expect("begin");
    stroke.sample(
        &mut half,
        StrokeSample {
            pressure: 0.5,
            ..press()
        },
    );

    // The 20 px tip reaches x = 40; the half-pressure 10 px tip does not.
    assert_eq!(pixel_red(&full, 40, 32), 0);
    assert_eq!(pixel_red(&half, 40, 32), WHITE);
    assert_eq!(pixel_red(&full, 32, 32), 0);
    assert_eq!(pixel_red(&half, 32, 32), 0);
}

#[test]
fn pressure_opacity_lightens_the_dab() {
    let cfg = StrokeConfig {
        pressure_opacity: true,
        ..black_cfg(100, 100)
    };
    let mut full = white_doc();
    let mut stroke = Stroke::begin_at(&full, "0", cfg).expect("begin");
    stroke.sample(&mut full, press());

    let mut half = white_doc();
    let mut stroke = Stroke::begin_at(&half, "0", cfg).expect("begin");
    stroke.sample(
        &mut half,
        StrokeSample {
            pressure: 0.5,
            ..press()
        },
    );

    assert_eq!(centre_red(&full), 0);
    assert!(centre_red(&half) > 100, "half pressure paints lighter");
}

#[test]
fn hold_keeps_the_last_pressure() {
    let cfg = StrokeConfig {
        pressure_opacity: true,
        ..black_cfg(100, 10)
    };
    let mut doc = white_doc();
    let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin");
    stroke.sample(
        &mut doc,
        StrokeSample {
            pressure: 0.5,
            ..press()
        },
    );
    for _ in 0..50 {
        stroke.hold(&mut doc);
    }
    assert!(
        centre_red(&doc) > 40,
        "a half-pressure hold stops at the 50% cap"
    );
}
