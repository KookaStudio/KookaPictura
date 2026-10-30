//! Interactive preview profiles: one old full-composite tick vs one display
//! preview tick on a 4000² eight-layer document.

use super::super::display_preview::PreviewKind;
use super::super::helpers;
use super::super::tests::pixel_layer;
use super::super::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document, PsdRect};

fn preview_ticks(kind: PreviewKind, level: u32, rect: PsdRect) -> (f64, f64, f64) {
    let n = 4000u32;
    let mut doc = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    let mut layers: Vec<_> = (0..8)
        .map(|i| pixel_layer(&format!("L{i}"), n, n, (30, 60, 90)))
        .collect();
    if kind == PreviewKind::Adjustment {
        layers.push(helpers::adjustment_layer("curves", None).expect("curves"));
    }
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc.composite = pictura_render::composite_active(&doc, false).0;
    let target = if kind == PreviewKind::Adjustment {
        "8".to_string()
    } else {
        "7".to_string()
    };
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        presented_level: level as i32,
        presented_rect: rect,
        ..Default::default()
    };
    rust.reset_pyramid();

    let steps = 5usize;
    rust.composite_full();
    let t = std::time::Instant::now();
    for _ in 0..steps {
        rust.composite_full();
    }
    let old = t.elapsed().as_secs_f64() * 1000.0 / steps as f64;

    rust.begin_display_preview(vec![target.clone()], kind);
    let t = std::time::Instant::now();
    let _ = rust.present_display_preview();
    let first = t.elapsed().as_secs_f64() * 1000.0;

    let t = std::time::Instant::now();
    for i in 0..steps {
        rust.begin_display_preview(vec![target.clone()], kind);
        if let Some(doc) = rust.session.doc.as_mut() {
            if let Some(layer) = pictura_render::resolve_path_mut(doc, &target) {
                match kind {
                    PreviewKind::Adjustment => {
                        let points = if i % 2 == 0 {
                            vec![(0, 0), (128, 160), (255, 255)]
                        } else {
                            vec![(0, 0), (128, 96), (255, 255)]
                        };
                        layer.adjustment = pictura_render::set_curve_points(
                            layer.adjustment.as_ref().expect("adjustment"),
                            0,
                            &points,
                        );
                    }
                    PreviewKind::Opacity => {
                        layer.opacity = if i % 2 == 0 { 128 } else { 200 };
                    }
                    PreviewKind::Fill => {}
                }
            }
        }
        let _ = rust.present_display_preview();
    }
    let new = t.elapsed().as_secs_f64() * 1000.0 / steps as f64;
    (old, first, new)
}

#[test]
#[ignore = "4000x4000 adjustment preview profile; run explicitly with --ignored --nocapture"]
fn adjustment_preview_profile_4000() {
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: 4000,
        right: 4000,
    };
    let viewport = PsdRect {
        top: 1600,
        left: 1360,
        bottom: 2400,
        right: 2640,
    };
    for (label, level, rect) in [("fit", 2u32, whole), ("100%", 0u32, viewport)] {
        let (old, first, new) = preview_ticks(PreviewKind::Adjustment, level, rect);
        println!(
            "adjustment_preview {label} level={level} old={old:.2}ms first={first:.2}ms steady={new:.2}ms ratio={:.1}x",
            old / new.max(0.01)
        );
    }
}

#[test]
#[ignore = "4000x4000 canvas opacity preview profile; run explicitly with --ignored --nocapture"]
fn canvas_opacity_preview_profile_4000() {
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: 4000,
        right: 4000,
    };
    let viewport = PsdRect {
        top: 1600,
        left: 1360,
        bottom: 2400,
        right: 2640,
    };
    for (label, level, rect) in [("fit", 2u32, whole), ("100%", 0u32, viewport)] {
        let (old, first, new) = preview_ticks(PreviewKind::Opacity, level, rect);
        println!(
            "opacity_preview {label} level={level} old={old:.2}ms first={first:.2}ms steady={new:.2}ms ratio={:.1}x",
            old / new.max(0.01)
        );
    }
}
