use super::super::document_session::DocumentSession;
use super::super::edit::Edit;
use super::{ms, pixel_layer, rgba_from_argb, LayerTarget, PictureViewRust};
use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerMask, LayerStack, PsdRect};

fn large_profile_doc() -> Document {
    let mut doc = Document::new(6000, 4000, ColorMode::Rgb, BitDepth::Eight);
    let mut layers = vec![pixel_layer("background", 6000, 4000, (10, 10, 10))];
    for i in 0..7 {
        let mut layer = pixel_layer(&format!("layer {i}"), 500, 500, (20 + i as u8, 30, 40));
        layer.rect = PsdRect {
            top: 100 + i * 500,
            left: 100 + i * 500,
            bottom: 600 + i * 500,
            right: 600 + i * 500,
        };
        layers.push(layer);
    }
    doc.layers = LayerStack::from_layers(layers);
    doc.composite = pictura_render::composite_rgba(&doc);
    doc
}

#[test]
#[ignore = "6000x4000 structural-delete profile; run explicitly with --ignored --nocapture"]
fn structural_delete_profile() {
    crate::cxxqt_object::metrics::set_for_this_thread(true);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(large_profile_doc()),
        gpu_compute: false,
        ..Default::default()
    };
    let damage = view
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, "1"));
    if let Some(doc) = view.session.doc.as_mut() {
        assert_eq!(pictura_render::delete_paths(doc, &["1"]), 1);
    }
    let t = std::time::Instant::now();
    view.apply_edit(
        "Delete",
        Edit::Structural {
            damage: damage.map(|rect| vec![rect]),
        },
    );
    ms("structural_delete composite", t.elapsed());
    let doc = view.session.doc.as_ref().expect("document");
    println!(
        "structural_delete canvas_bytes={} damage_width={}",
        doc.composite.data.len(),
        damage.map_or(0, |rect| rect.width())
    );
    if let Some(json) = crate::cxxqt_object::metrics::snapshot_json(true) {
        println!("structural_delete metrics: {json}");
    }
}

#[test]
#[ignore = "6000x4000 stroke-capture profile; run explicitly with --ignored --nocapture"]
fn stroke_capture_profile() {
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeConfig, StrokeSample};

    let mut doc = large_profile_doc();
    let mut history = crate::history::History::default();
    history.capture_live(&mut doc, &None, &LayerTarget::default(), "Open");
    let cfg = StrokeConfig {
        color: rgba_from_argb(0xFF000000),
        diameter: 512,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_at(&doc, "1", cfg).expect("stroke begins");
    let mut hint = None;
    for i in 0..16 {
        stroke.sample(
            &mut doc,
            StrokeSample {
                x: 600.0 + i as f32 * 180.0,
                y: 2000.0,
                pressure: 1.0,
            },
        );
        if let Some(rect) = stroke.take_dirty() {
            hint = Some(hint.map_or(rect, |union: PsdRect| PsdRect {
                top: union.top.min(rect.top),
                left: union.left.min(rect.left),
                bottom: union.bottom.max(rect.bottom),
                right: union.right.max(rect.right),
            }));
        }
    }
    if let Some(rect) = stroke.finish() {
        hint = Some(hint.map_or(rect, |union| PsdRect {
            top: union.top.min(rect.top),
            left: union.left.min(rect.left),
            bottom: union.bottom.max(rect.bottom),
            right: union.right.max(rect.right),
        }));
    }
    let t = std::time::Instant::now();
    history.capture_live_with(
        &mut doc,
        &None,
        &LayerTarget::default(),
        "Brush",
        hint.as_ref().map(std::slice::from_ref),
    );
    ms("stroke_capture history", t.elapsed());
}

#[test]
#[ignore = "6000x4000 mask-feather profile; run explicitly with --ignored --nocapture"]
fn mask_feather_preview_profile() {
    let mut doc = large_profile_doc();
    let mut layers: Vec<Layer> = doc.layers.as_slice().to_vec();
    layers.push(Layer {
        name: "adjustment".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4000,
            right: 6000,
        },
        adjustment: Some(pictura_render::encode_brightness_contrast(20, 0)),
        mask: Some(LayerMask {
            rect: PsdRect {
                top: 200,
                left: 200,
                bottom: 700,
                right: 700,
            },
            default_color: 0,
            disabled: false,
            data: Some(vec![255u8; 500 * 500].into()),
            density: 255,
            feather_bits: 8.0f64.to_bits(),
            ..Default::default()
        }),
        ..Default::default()
    });
    doc.layers = LayerStack::from_layers(layers);
    let path = doc.layers.len().saturating_sub(1).to_string();
    let before = pictura_render::damage::influence(&doc, &path);
    assert!(pictura_render::set_layer_mask_feather(&mut doc, &path, 4.0));
    let after = pictura_render::damage::influence(&doc, &path);
    let damage = pictura_render::damage::union(before, after).expect("bounded");
    let t = std::time::Instant::now();
    let _ = pictura_render::composite_rgba_region(
        &doc,
        damage.left.max(0) as u32,
        damage.top.max(0) as u32,
        damage.width().max(0) as u32,
        damage.height().max(0) as u32,
    );
    ms("mask_feather preview", t.elapsed());
    println!("mask_feather damage={damage:?}");
}
