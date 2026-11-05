//! The lasso and shape commits on a 17000×17000 document, stage by stage
//! (#1185): the polygon fill, its feather and combine, the Magnetic Lasso's
//! edge field, a shape layer's region composite, and a Fill Pixels shape.

use super::super::helpers::feathered_within;
use super::super::impl_selection::ShapeBox;
use super::{ms, pixel_layer, PictureViewRust};
use pictura_core::shape::{coverage_box, outline_in_box, ShapeKind, ShapeOptions};
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
use pictura_paint::bucket::{self, BucketPaint};
use pictura_paint::{Alpha, PaintMode, Rgba};
use pictura_select::{EdgeMap, Selection};
use std::time::Instant;

/// `PICTURA_PROFILE_DIVISOR` shrinks both sides for a machine without the
/// memory the full size takes.
fn size() -> (u32, u32) {
    let divisor = std::env::var("PICTURA_PROFILE_DIVISOR")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);
    (17000 / divisor, 17000 / divisor)
}

/// A lasso of `n` vertices around an ellipse over half the document.
fn ring(width: u32, height: u32, n: usize) -> Vec<(i32, i32)> {
    let (cx, cy) = (width as f64 * 0.45, height as f64 * 0.425);
    let (rx, ry) = (width as f64 * 0.25, height as f64 * 0.225);
    (0..n)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / n as f64;
            ((cx + rx * t.cos()) as i32, (cy + ry * t.sin()) as i32)
        })
        .collect()
}

#[test]
#[ignore = "17000x17000 lasso and shape profile; run explicitly with --ignored --nocapture"]
fn shape_lasso_profile() {
    let (width, height) = size();
    let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer(
        "Background",
        width,
        height,
        (255, 255, 255),
    )]);
    doc.composite = pictura_render::composite_rgba(&doc);

    for vertices in [30, 30_000] {
        let t = Instant::now();
        let (lasso, bounds) =
            Selection::polygon_bounded(width, height, &ring(width, height, vertices));
        ms(&format!("lasso fill, {vertices} vertices"), t.elapsed());
        let t = Instant::now();
        let (soft, _) = feathered_within(lasso.clone(), bounds, 20.0);
        ms("lasso feather 20", t.elapsed());
        drop(soft);
        let mut rust = PictureViewRust::default();
        rust.session
            .set_selection(Some(Selection::rect(width, height, 10, 10, 400, 400)));
        let t = Instant::now();
        let boxed = bounds.map(|zero_outside| ShapeBox {
            zero_outside,
            tight: true,
        });
        super::super::impl_selection::combine_for_profile(&mut rust, lasso, boxed);
        ms("lasso add", t.elapsed());
        let t = Instant::now();
        let _ = rust.selection_mask_bounds();
        ms("lasso bounds read", t.elapsed());
    }

    let t = Instant::now();
    let map = EdgeMap::from_buffer(&doc.composite, 10);
    ms("magnetic press (peak pass)", t.elapsed());
    let t = Instant::now();
    let wire = map.trace((2000, 2000), (2600, 2300), 10);
    ms(&format!("magnetic trace ({} px)", wire.len()), t.elapsed());

    let shape_box = (
        width as f64 * 0.2,
        height as f64 * 0.2,
        width as f64 * 0.5,
        height as f64 * 0.45,
    );
    for kind in [ShapeKind::Rectangle, ShapeKind::Ellipse, ShapeKind::Polygon] {
        let outline = outline_in_box(ShapeOptions::new(kind, 0.0, 5), shape_box).expect("a shape");
        let mut shaped = doc.clone();
        let path = pictura_render::add_shape_layer(
            &mut shaped,
            "",
            [255, 0, 0, 255],
            "Shape",
            &outline,
            None,
        );
        let reach =
            pictura_render::damage::content_influence(&shaped, &path).expect("a bounded shape");
        let t = Instant::now();
        let _ = pictura_render::composite(pictura_render::CompositeRequest::region(
            &shaped, reach, false,
        ));
        ms(
            &format!("{kind:?} layer region composite (CPU)"),
            t.elapsed(),
        );
    }
    let ellipse = outline_in_box(ShapeOptions::new(ShapeKind::Ellipse, 0.0, 3), shape_box)
        .expect("an ellipse");

    let mut filled = doc.clone();
    let t = Instant::now();
    let (bounds, mask) = coverage_box(&ellipse, width, height).expect("on the canvas");
    ms("shape pixels coverage", t.elapsed());
    let t = Instant::now();
    let red = BucketPaint::Foreground(Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    });
    bucket::fill_within(
        &mut filled,
        "0",
        bounds,
        Some(&mask),
        red,
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend,
    );
    ms("shape pixels fill", t.elapsed());
}
