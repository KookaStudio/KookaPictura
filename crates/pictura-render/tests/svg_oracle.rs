//! Independent librsvg oracle for SVG export: a vector-only document is
//! serialized with `pictura_render::encode_document_svg`, rendered by
//! ImageMagick's librsvg coder, and compared against
//! `pictura_render::composite_rgba` after both are flattened onto white.
//!
//! The comparison is pixel-level within a tolerance (antialiased edges differ
//! between the two rasterizers), never byte-level. Needs `magick` with the
//! `RSVG` coder; self-skips with a message otherwise.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_adjust::{GradientKind, GradientStop, PatternFillParams};
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
use pictura_core::{BitDepth, BlendMode, ColorMode, Document, Layer};
use pictura_testkit::compare;

// A unique path per call: nextest runs one binary's tests in parallel threads.
fn scratch(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "pictura-svg-oracle-{}-{tag}-{n}.svg",
        std::process::id()
    ))
}

/// Rasterize `svg` with librsvg (forced by the `RSVG:` coder prefix) onto
/// white, as interleaved RGB8.
fn rsvg_render(svg: &str, tag: &str) -> Result<Vec<u8>, String> {
    let path = scratch(tag);
    std::fs::write(&path, svg).expect("write temp svg");
    let out = Command::new("magick")
        .args(["-background", "white"])
        .arg(format!("RSVG:{}", path.display()))
        .args(["-flatten", "-depth", "8", "rgb:-"])
        .output();
    let _ = std::fs::remove_file(&path);
    let out = out.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(out.stdout)
}

/// Whether magick has the librsvg coder. A build without librsvg still
/// accepts the `RSVG:` prefix but silently renders with its internal MSVG
/// renderer, which ignores `mix-blend-mode`, so a successful render is not
/// enough; the format list must name librsvg.
fn rsvg_available() -> bool {
    Command::new("magick")
        .args(["-list", "format"])
        .output()
        .is_ok_and(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|line| line.trim_start().starts_with("RSVG") && line.contains("Librsvg"))
        })
}

/// `doc`'s CPU composite flattened onto white, as interleaved RGB8.
fn composite_on_white(doc: &Document) -> Vec<u8> {
    let buffer = pictura_render::composite_rgba(doc);
    let plane = (buffer.width * buffer.height) as usize;
    let mut rgb = Vec::with_capacity(plane * 3);
    for i in 0..plane {
        let a = u32::from(buffer.data[3 * plane + i]);
        for c in 0..3 {
            let v = u32::from(buffer.data[c * plane + i]);
            rgb.push(((v * a + 255 * (255 - a) + 127) / 255) as u8);
        }
    }
    rgb
}

fn add_shape(doc: &mut Document, kind: ShapeKind, rect: (f64, f64, f64, f64), rgba: [u8; 4]) {
    let outline = outline_in_box(ShapeOptions::new(kind, 0.0, 3), rect).unwrap();
    pictura_render::add_shape_layer(doc, "", rgba, "Shape", &outline, None);
}

/// A shape on its own, plus an ellipse that multiplies onto it and a faded
/// group, so primitives, `mix-blend-mode`, and group opacity are all covered.
fn vector_document() -> Document {
    let mut doc = Document::new(64, 56, ColorMode::Rgb, BitDepth::Eight);
    add_shape(
        &mut doc,
        ShapeKind::Rectangle,
        (8.0, 8.0, 40.0, 40.0),
        [220, 40, 30, 255],
    );
    add_shape(
        &mut doc,
        ShapeKind::Ellipse,
        (20.0, 10.0, 60.0, 50.0),
        [40, 90, 230, 255],
    );
    add_shape(
        &mut doc,
        ShapeKind::Rectangle,
        (30.0, 30.0, 56.0, 44.0),
        [30, 200, 80, 255],
    );
    let grouped = doc.layers.pop().unwrap();
    {
        let _ = doc.layers.push(pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: "Group".into(),
            is_group: true,
            opacity: 160,
            children: pictura_core::LayerStack::from_layers(vec![grouped]),
            ..Default::default()
        }));
    };
    let ellipse = &mut doc.layers[1];
    ellipse.blend = BlendMode::Multiply;
    ellipse.opacity = 200;
    doc
}

fn add_gradient_shape(
    doc: &mut Document,
    kind: GradientKind,
    rect: (f64, f64, f64, f64),
    angle: f32,
) {
    let outline = outline_in_box(ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3), rect).unwrap();
    let path = pictura_render::add_shape_layer(doc, "", [0, 0, 0, 255], "Gradient", &outline, None);
    let layer = pictura_render::resolve_path_mut(doc, &path).unwrap();
    pictura_codec::edit_layer(layer.as_mut(), |parts| { parts.adjustment = Some(pictura_render::encode_gradient_fill(
        kind,
        &[
            GradientStop {
                location: 0,
                color: [255, 30, 20],
            },
            GradientStop {
                location: 4096,
                color: [20, 40, 230],
            },
        ],
        angle,
    )) });
}

fn add_pattern_shape(doc: &mut Document, rect: (f64, f64, f64, f64)) {
    let outline = outline_in_box(ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3), rect).unwrap();
    let path = pictura_render::add_shape_layer(doc, "", [0, 0, 0, 255], "Pattern", &outline, None);
    let layer = pictura_render::resolve_path_mut(doc, &path).unwrap();
    pictura_codec::edit_layer(layer.as_mut(), |parts| { parts.adjustment = Some(pictura_render::encode_pattern_fill(&PatternFillParams {
        pattern_id: "oracle-checker".into(),
        scale: 100.0,
        link_with_layer: true,
        origin: (0, 0),
    })) });
}

fn add_stroked_shape(doc: &mut Document, rect: (f64, f64, f64, f64)) {
    let outline = outline_in_box(ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3), rect).unwrap();
    let path =
        pictura_render::add_shape_layer(doc, "", [250, 220, 30, 255], "Stroke", &outline, None);
    let layer = pictura_render::resolve_path_mut(doc, &path).unwrap();
    pictura_render::set_shape_stroke(
        layer,
        Some(&pictura_render::ShapeStroke {
            color: [20, 60, 240],
            width: 3,
            position: pictura_render::StrokePosition::Outside,
        }),
    );
}

/// A vector-only document with both gradient kinds, a pattern, and an outside
/// stroke.
fn primitive_document() -> Document {
    let mut doc = Document::new(64, 56, ColorMode::Rgb, BitDepth::Eight);
    let mut tile = Vec::with_capacity(8 * 8 * 4);
    for y in 0..8 {
        for x in 0..8 {
            let v = if (x + y) % 2 == 0 { 32 } else { 224 };
            tile.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let record =
        pictura_codec::encode_rgb_pattern("Checker", "oracle-checker", 8, 8, &tile).unwrap();
    assert!(pictura_codec::add_document_pattern(
        &mut doc,
        "oracle-checker",
        &record
    ));
    add_gradient_shape(&mut doc, GradientKind::Linear, (4.0, 4.0, 24.0, 20.0), 0.0);
    add_gradient_shape(&mut doc, GradientKind::Radial, (32.0, 4.0, 28.0, 20.0), 0.0);
    add_pattern_shape(&mut doc, (4.0, 30.0, 24.0, 22.0));
    add_stroked_shape(&mut doc, (34.0, 30.0, 24.0, 20.0));
    doc
}

#[test]
fn librsvg_renders_gradients_patterns_and_strokes_like_the_composite() {
    let doc = primitive_document();
    let svg = pictura_render::encode_document_svg(&doc, |_: &Document| {
        panic!("a primitive document rasterized a layer")
    });
    let body = svg
        .split_once("</defs>")
        .map_or(svg.as_str(), |(_, body)| body);
    assert!(!body.contains("<image"), "{svg}");
    assert!(svg.contains("<linearGradient"), "{svg}");
    assert!(svg.contains("<radialGradient"), "{svg}");
    assert!(svg.contains("<pattern"), "{svg}");
    assert!(svg.contains("stroke="), "{svg}");

    if !rsvg_available() {
        eprintln!("skipping: magick with the RSVG coder not available");
        return;
    }
    let rendered = rsvg_render(&svg, "primitives").expect("librsvg renders the export");
    let expected = composite_on_white(&doc);
    let diff = compare(&rendered, &expected, 6).expect("same size");
    assert!(
        diff.differing * 100 <= diff.samples * 3 && diff.mean_delta() < 2.0,
        "{diff:?} mean {:.3}\n{svg}",
        diff.mean_delta()
    );
}

#[test]
fn librsvg_renders_the_exported_svg_like_the_composite() {
    let doc = vector_document();
    // A vector-only document must not need a raster fallback.
    let svg = pictura_render::encode_document_svg(&doc, |_: &Document| {
        panic!("a vector-only document rasterized a layer")
    });
    assert!(!svg.contains("<image"), "{svg}");
    assert!(svg.contains("mix-blend-mode:multiply"), "{svg}");
    assert!(svg.contains("<g opacity="), "{svg}");

    if !rsvg_available() {
        eprintln!("skipping: magick with the RSVG coder not available");
        return;
    }
    let rendered = rsvg_render(&svg, "vector").expect("librsvg renders the export");
    let expected = composite_on_white(&doc);
    let diff = compare(&rendered, &expected, 6).expect("same size");
    // Interiors must agree to rounding; only antialiased edge samples may
    // differ beyond the tolerance.
    assert!(
        diff.differing * 100 <= diff.samples * 2 && diff.mean_delta() < 1.5,
        "{diff:?} mean {:.3}\n{svg}",
        diff.mean_delta()
    );
}
