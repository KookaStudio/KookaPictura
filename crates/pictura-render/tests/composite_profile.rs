//! CPU composite profile: a 4000² document through the paths the per-layer
//! context targets. Run with
//! `cargo test -p pictura-render --test composite_profile -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorMode, Document, Layer, LayerMask, LayerStack, PsdRect,
    TypeSpec, VectorFillRule, VectorMask, VectorSubpath,
};
use pictura_render::{add_type_layer, composite_rgba};

const DEFAULT_EDGE: u32 = 4000;

fn edge() -> u32 {
    std::env::var("PICTURA_PROFILE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_EDGE)
}

fn full(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn pixel_layer(
    name: &str,
    w: u32,
    h: u32,
    rgb: (u8, u8, u8),
    alpha: u8,
    blend: BlendMode,
) -> Layer {
    let n = (w * h) as usize;
    Layer {
        name: name.into(),
        rect: full(w, h),
        blend,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: vec![
            Channel {
                id: 0,
                data: vec![rgb.0; n].into(),
            },
            Channel {
                id: 1,
                data: vec![rgb.1; n].into(),
            },
            Channel {
                id: 2,
                data: vec![rgb.2; n].into(),
            },
            Channel {
                id: -1,
                data: vec![alpha; n].into(),
            },
        ],
        ..Default::default()
    }
}

fn scene(w: u32, h: u32, top: Layer) -> Document {
    let mut d = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![
        pixel_layer("Background", w, h, (255, 255, 255), 255, BlendMode::Normal),
        top,
    ]);
    d
}

fn time(doc: &Document, label: &str) {
    let mut best = Duration::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let _ = composite_rgba(doc);
        best = best.min(start.elapsed());
    }
    eprintln!("{label}: {:.1} ms", best.as_secs_f64() * 1000.0);
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test composite_profile -- --ignored --nocapture"]
fn cpu_composite_profile_4000() {
    let (w, h) = (edge(), edge());

    let normal = scene(
        w,
        h,
        pixel_layer("top", w, h, (200, 100, 50), 255, BlendMode::Normal),
    );
    time(&normal, "Normal, no mask");

    let multiply = scene(
        w,
        h,
        pixel_layer("top", w, h, (200, 100, 50), 255, BlendMode::Multiply),
    );
    time(&multiply, "Multiply");

    let mut feathered = pixel_layer("masked", w, h, (0, 0, 255), 255, BlendMode::Multiply);
    let mut mask = LayerMask {
        rect: full(w, h),
        default_color: 0,
        data: Some(vec![128; (w * h) as usize].into()),
        ..Default::default()
    };
    mask.set_feather(20.0);
    feathered.mask = Some(mask);
    time(&scene(w, h, feathered), "feathered raster mask");

    let mut vector = pixel_layer("vector", w, h, (0, 255, 0), 255, BlendMode::Screen);
    let p = |x: i32, y: i32| [x * 256, y * 256];
    vector.vector_mask = Some(VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![
                p(0, 0),
                p(w as i32 / 2, 0),
                p(w as i32 / 2, h as i32),
                p(0, h as i32),
            ],
        }],
        ..Default::default()
    });
    time(&scene(w, h, vector), "vector mask");

    let mut type_doc = scene(
        w,
        h,
        pixel_layer("top", w, h, (200, 100, 50), 255, BlendMode::Normal),
    );
    let spec = TypeSpec::new("Profile", "Liberation Sans", (h as f64 / 20.0).max(24.0));
    add_type_layer(&mut type_doc, "", &spec);
    time(&type_doc, "full-canvas type layer");

    time(&mixed_doc(w, h), "Curves + Color Lookup + type + style");
}

fn adjustment_layer(name: &str, adjustment: pictura_core::AdjustmentData) -> Layer {
    Layer {
        name: name.into(),
        rect: full(0, 0),
        adjustment: Some(adjustment),
        ..Default::default()
    }
}

/// A standard 33³ identity `.CUBE`, the size a real Color Lookup carries, so
/// the profile measures the embedded parse rather than a trivial 2³ table.
fn identity_cube(size: usize) -> String {
    let mut cube = format!("LUT_3D_SIZE {size}\n");
    let d = (size - 1) as f64;
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                cube.push_str(&format!(
                    "{:.6} {:.6} {:.6}\n",
                    r as f64 / d,
                    g as f64 / d,
                    b as f64 / d
                ));
            }
        }
    }
    cube
}

fn mixed_doc(w: u32, h: u32) -> Document {
    let cube = identity_cube(33);
    let mut styled = pixel_layer("styled", w, h, (200, 100, 50), 255, BlendMode::Multiply);
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    let mut curves = adjustment_layer(
        "curves",
        pictura_render::encode_curves(&[(0, 0), (128, 140), (255, 255)], None, None, None),
    );
    curves.rect = full(w, h);
    let mut lookup = adjustment_layer(
        "lookup",
        pictura_render::encode_color_lookup(cube.as_bytes(), "identity"),
    );
    lookup.rect = full(w, h);
    let mut d = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![
        pixel_layer("Background", w, h, (255, 255, 255), 255, BlendMode::Normal),
        curves,
        lookup,
        styled,
    ]);
    let spec = TypeSpec::new("Profile", "Liberation Sans", (h as f64 / 20.0).max(24.0));
    add_type_layer(&mut d, "", &spec);
    d
}
