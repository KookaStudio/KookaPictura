//! The brush path's region composite: exact against the full composite, and
//! fast beside shape layers and layer effects.

use super::*;
use crate::composite::composite_rgba_region;
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};

/// A white document with an ellipse shape layer at (70, 20)-(110, 60) given a
/// 4 px outside stroke, and a half-covered pixel layer.
fn shapes_doc() -> Document {
    let mut d = doc(
        120,
        80,
        vec![
            solid(
                "Background",
                full(120, 80),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "Half",
                rect(0, 0, 40, 60),
                (0, 0, 200),
                128,
                BlendMode::Multiply,
                255,
            ),
        ],
    );
    let ellipse = outline_in_box(
        ShapeOptions::new(ShapeKind::Ellipse, 0.0, 3),
        (70.0, 20.0, 40.0, 40.0),
    )
    .unwrap();
    let path = crate::add_shape_layer(&mut d, "", [255, 0, 0, 255], "Ellipse", &ellipse, None);
    let layer = crate::resolve_path_mut(&mut d, &path).unwrap();
    crate::set_shape_stroke(
        layer,
        Some(&crate::ShapeStroke {
            color: [0, 160, 0],
            width: 4,
            position: crate::StrokePosition::Outside,
        }),
    );
    d
}

fn assert_region_matches_full(d: &Document, x0: u32, y0: u32, w: u32, h: u32) {
    let full = composite_rgba(d);
    let region = composite_rgba_region(d, x0, y0, w, h);
    for y in 0..h {
        for x in 0..w {
            assert_eq!(
                px(&region, x, y),
                px(&full, x0 + x, y0 + y),
                "pixel ({}, {})",
                x0 + x,
                y0 + y
            );
        }
    }
}

#[test]
fn a_region_no_effect_reaches_composites_alone_and_exactly() {
    let d = shapes_doc();
    // Left of the stroked ellipse (its reach starts at x = 64): the fast path.
    assert_region_matches_full(&d, 5, 5, 50, 50);
}

#[test]
fn a_region_an_effect_reaches_still_matches_the_full_composite() {
    let d = shapes_doc();
    // Across the stroke band: the full-composite fallback.
    assert_region_matches_full(&d, 50, 10, 40, 40);
    let full = composite_rgba(&d);
    assert_eq!(
        rgb(&full, 68, 40),
        [0, 160, 0],
        "the outside stroke is drawn"
    );
}

#[test]
#[ignore = "profiling: prints the brush-dab region cost beside shape layers"]
fn shape_layer_region_profile() {
    let (w, h) = (2000u32, 1500u32);
    let mut d = Document::from_rgba("Background", w, h, &vec![255; (w * h * 4) as usize]);
    let time = |d: &Document, label: &str| {
        let t = std::time::Instant::now();
        for i in 0..20 {
            let _ = composite_rgba_region(d, 100 + i * 10, 100, 64, 64);
        }
        eprintln!("{label}: {:?} per 64x64 region", t.elapsed() / 20);
    };
    time(&d, "background only");
    for (k, kind) in [
        ShapeKind::Rectangle,
        ShapeKind::Ellipse,
        ShapeKind::CustomShape,
        ShapeKind::Polygon,
    ]
    .into_iter()
    .enumerate()
    {
        let mut o = ShapeOptions::new(kind, 0.0, 5);
        o.custom = 4;
        let s = outline_in_box(o, (100.0 + k as f64 * 300.0, 100.0, 400.0, 300.0)).unwrap();
        crate::add_shape_layer(&mut d, "", [255, 0, 0, 255], "S", &s, None);
    }
    time(&d, "4 shape layers");
    let t = std::time::Instant::now();
    let _ = composite_rgba(&d);
    eprintln!("full composite with 4 shapes: {:?}", t.elapsed());
    let path = format!("{}", d.layers.len() - 1);
    let layer = crate::resolve_path_mut(&mut d, &path).unwrap();
    crate::set_shape_stroke(
        layer,
        Some(&crate::ShapeStroke {
            color: [0, 0, 0],
            width: 3,
            position: crate::StrokePosition::Center,
        }),
    );
    time(&d, "4 shapes, one stroked away from the region");
}

/// Print-only: what adding one shape layer costs on a large document (issue
/// #719) — the full composite the commit used to run, against a composite of
/// the shape's bounds only, on the CPU and (when present) the GPU, for a
/// rectangle and an ellipse. `PICTURA_PROFILE_SIZE` sets the square edge
/// (default 8000; the world map is ~16000).
#[test]
#[ignore = "profiling: prints a shape layer's commit composite cost"]
fn shape_commit_profile() {
    let n: u32 = std::env::var("PICTURA_PROFILE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8000);
    let base = Document::from_rgba("Background", n, n, &vec![200; (n as usize).pow(2) * 4]);
    let gpu = crate::gpu_available();
    let box_ = (
        n as f64 * 0.2,
        n as f64 * 0.2,
        n as f64 * 0.5,
        n as f64 * 0.45,
    );
    for kind in [ShapeKind::Rectangle, ShapeKind::Ellipse] {
        for backend in [false, true] {
            if backend && !gpu {
                continue;
            }
            let mut d = base.clone();
            let s = outline_in_box(ShapeOptions::new(kind, 0.0, 5), box_).unwrap();
            let path = crate::add_shape_layer(&mut d, "", [255, 0, 0, 255], "S", &s, None);
            let (l, t, r, b) =
                crate::shape_bounds(crate::resolve_path(&d, &path).unwrap(), n, n).unwrap();
            let rect = PsdRect {
                top: t.floor() as i32 - 2,
                left: l.floor() as i32 - 2,
                bottom: b.ceil() as i32 + 2,
                right: r.ceil() as i32 + 2,
            };
            let t = std::time::Instant::now();
            let _ = crate::composite_active(&d, backend);
            let full = t.elapsed();
            let t = std::time::Instant::now();
            let _ = crate::composite_region_active(&d, rect, backend);
            let region = t.elapsed();
            eprintln!(
                "shape_commit_profile {n}x{n} {kind:?} gpu={backend}: full={:.1}ms \
                 bounds={:.1}ms ({}x{})",
                full.as_secs_f64() * 1000.0,
                region.as_secs_f64() * 1000.0,
                rect.width(),
                rect.height(),
            );
        }
    }
}

/// `layer` with every channel textured, so a band composited from the wrong
/// rows or columns cannot go unnoticed.
fn textured(mut layer: Layer, seed: usize) -> Layer {
    for (c, channel) in layer.channels.iter_mut().enumerate() {
        for (i, v) in channel.data.as_mut_slice().iter_mut().enumerate() {
            *v = ((i * (3 + c) + seed * 17 + c * 41) % 256) as u8;
        }
    }
    layer
}

/// A per-pixel stack taller than two bands: offset and partial layers, blend
/// modes including Dissolve, a raster mask, an isolated group with a clipped
/// child, and an adjustment.
fn banded_doc() -> Document {
    let (w, h) = (101u32, 333u32);
    let mut masked = textured(
        solid(
            "Masked",
            rect(40, 10, 300, 90),
            (0, 0, 0),
            255,
            BlendMode::Screen,
            220,
        ),
        3,
    );
    masked.mask = Some(LayerMask {
        rect: rect(50, 0, 280, 70),
        default_color: 0,
        data: Some((0..230 * 70).map(|i| (i % 253) as u8).collect()),
        ..Default::default()
    });
    let mut clipped = textured(
        solid(
            "Clipped",
            rect(0, 0, 333, 101),
            (0, 0, 0),
            255,
            BlendMode::Overlay,
            255,
        ),
        5,
    );
    clipped.clipping = true;
    doc(
        w,
        h,
        vec![
            textured(
                solid("Base", full(w, h), (0, 0, 0), 255, BlendMode::Normal, 255),
                1,
            ),
            textured(
                solid(
                    "Offset",
                    rect(-7, -9, 200, 60),
                    (0, 0, 0),
                    255,
                    BlendMode::Multiply,
                    200,
                ),
                2,
            ),
            masked,
            group(
                "Group",
                BlendMode::Normal,
                230,
                None,
                vec![
                    textured(
                        solid(
                            "Inner",
                            rect(100, 20, 320, 101),
                            (0, 0, 0),
                            255,
                            BlendMode::Difference,
                            255,
                        ),
                        4,
                    ),
                    clipped,
                ],
            ),
            textured(
                solid(
                    "Dissolve",
                    rect(120, 5, 250, 95),
                    (0, 0, 0),
                    255,
                    BlendMode::Dissolve,
                    140,
                ),
                6,
            ),
            adjustment_layer("Invert", *b"nvrt", Vec::new(), 180, None),
        ],
    )
}

#[test]
fn banded_compositing_equals_one_canvas() {
    let d = banded_doc();
    let whole = crate::composite::composite_whole(&d);
    let banded = composite_rgba(&d);
    assert!(banded.data == whole.data, "the banded composite differs");

    let (x0, y0, w, h) = (3, 17, 90, 290);
    let region = composite_rgba_region(&d, x0, y0, w, h);
    for y in 0..h {
        for x in 0..w {
            assert_eq!(
                px(&region, x, y),
                px(&whole, x0 + x, y0 + y),
                "pixel ({}, {})",
                x0 + x,
                y0 + y
            );
        }
    }
}

#[test]
fn banding_with_a_short_type_layer_equals_one_canvas() {
    let mut d = banded_doc();
    let mut spec = pictura_core::TypeSpec::new("Band", "Liberation Sans", 40.0);
    spec.origin = (10.0, 140.0);
    spec.character.fill_color = [0.9, 0.2, 0.1, 1.0];
    let path = crate::add_type_layer(&mut d, "", &spec);
    let layer = crate::resolve_path_mut(&mut d, &path).expect("type layer");
    layer.blend = BlendMode::Multiply;
    // Without its rasterized proxy the text is rendered while compositing.
    layer.channels.clear();
    assert!(
        layer.rect.height() > 0 && layer.rect.height() <= 256,
        "{:?}",
        layer.rect
    );
    let whole = crate::composite::composite_whole(&d);
    assert!(
        composite_rgba(&d).data == whole.data,
        "the banded composite differs"
    );
    assert!(
        whole.data != crate::composite::composite_whole(&banded_doc()).data,
        "no text drawn"
    );
}
