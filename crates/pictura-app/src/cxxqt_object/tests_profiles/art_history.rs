//! The present half of an Art History stroke's motion at 8000×6000 (#1238):
//! the region refresh one move's dirty box costs once fit-to-screen tiles are
//! current.

use super::super::document_session::DocumentSession;
use super::{ms, pixel_layer, PictureViewRust};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};
use std::time::Instant;

#[test]
#[ignore = "8000x6000 Art History present profile; run explicitly with --ignored --nocapture"]
fn art_history_present_profile() {
    for gpu in [false, true] {
        println!("gpu_compute={gpu}");
        present_profile(gpu);
    }
}

fn present_profile(gpu_compute: bool) {
    let (w, h) = (8000_u32, 6000_u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("Background", w, h, (200, 120, 40))]);
    doc.composite
        .replace(pictura_render::composite_rgba(&doc))
        .expect("valid composite");
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute,
        ..Default::default()
    };
    view.reset_pyramid();
    // Fit-to-screen: a 1280-wide view of 8000 px shows level 3.
    assert!(view.level_snapshot(3).is_some());
    crate::cxxqt_object::metrics::set_for_this_thread(true);
    for (label, side) in [("history-brush box", 330), ("art-history box", 520)] {
        let _ = crate::cxxqt_object::metrics::snapshot_json(true);
        let t = Instant::now();
        for i in 0..60 {
            let t = i as f32 / 60.0;
            let wave = ((t * 3.0) % 2.0 - 1.0).abs();
            let (x, y) = (
                (w as f32 * (0.15 + 0.7 * t)) as i32,
                (h as f32 * (0.35 + 0.3 * wave)) as i32,
            );
            let r = side / 2;
            view.refresh_display_rect(PsdRect {
                top: y - r,
                left: x - r,
                bottom: y + r,
                right: x + r,
            });
        }
        ms(label, t.elapsed());
        if let Some(json) = crate::cxxqt_object::metrics::snapshot_json(true) {
            println!("{label} metrics: {json}");
        }
    }
}

/// Where a region composite on the GPU starts to beat the CPU, for the
/// present path's size floor.
#[test]
#[ignore = "GPU/CPU region composite crossover; run explicitly with --ignored --nocapture"]
fn region_composite_crossover_profile() {
    use pictura_render::{composite, CompositeRequest};
    let (w, h) = (8000_u32, 6000_u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut layers = vec![pixel_layer("Background", w, h, (200, 120, 40))];
    for i in 0..3 {
        layers.push(pixel_layer(
            &format!("Layer {i}"),
            w,
            h,
            (20 * i as u8, 90, 160),
        ));
    }
    for layers_used in [1, 4] {
        doc.layers = LayerStack::from_layers(layers[..layers_used].to_vec());
        let _ = composite(CompositeRequest::region(
            &doc,
            PsdRect {
                top: 0,
                left: 0,
                bottom: 64,
                right: 64,
            },
            true,
        ));
        for side in [256, 512, 768, 1024, 1536, 2048, 4096] {
            let rect = PsdRect {
                top: 1000,
                left: 1000,
                bottom: 1000 + side,
                right: 1000 + side,
            };
            let mut row = format!("layers {layers_used} side {side}:");
            for gpu in [false, true] {
                let t = Instant::now();
                let mut backend = None;
                for _ in 0..5 {
                    backend = Some(composite(CompositeRequest::region(&doc, rect, gpu)).1);
                }
                row += &format!(
                    " {:?} {:.2} ms",
                    backend.unwrap(),
                    t.elapsed().as_secs_f64() * 200.0
                );
            }
            println!("{row}");
        }
    }
}
