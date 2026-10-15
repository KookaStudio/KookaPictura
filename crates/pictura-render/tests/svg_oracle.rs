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
    doc.layers.push(Layer {
        name: "Group".into(),
        is_group: true,
        opacity: 160,
        children: vec![grouped],
        ..Default::default()
    });
    let ellipse = &mut doc.layers[1];
    ellipse.blend = BlendMode::Multiply;
    ellipse.opacity = 200;
    doc
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
