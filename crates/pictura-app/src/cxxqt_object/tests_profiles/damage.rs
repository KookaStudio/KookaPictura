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

/// 4500x1152 document matching the reported clipping-move scenario: a small
/// base, a full-canvas layer clipped to it, and two siblings above.
fn clipping_move_doc(clipped_layers: usize, extra_layers: usize) -> Document {
    let mut doc = Document::new(4500, 1152, ColorMode::Rgb, BitDepth::Eight);
    let mut base = pixel_layer("base", 400, 300, (200, 30, 30));
    base.rect = PsdRect {
        top: 400,
        left: 1000,
        bottom: 700,
        right: 1400,
    };
    let mut clipped = pixel_layer("clipped", 4500, 1152, (30, 30, 200));
    clipped.clipping = true;
    let mut sibling = pixel_layer("sibling", 500, 500, (40, 200, 40));
    sibling.rect = PsdRect {
        top: 200,
        left: 2000,
        bottom: 700,
        right: 2500,
    };
    let mut sibling2 = pixel_layer("sibling2", 300, 300, (200, 200, 40));
    sibling2.rect = PsdRect {
        top: 600,
        left: 3000,
        bottom: 900,
        right: 3300,
    };
    let mut layers = vec![
        pixel_layer("background", 4500, 1152, (10, 10, 10)),
        base,
        clipped,
    ];
    for i in 0..clipped_layers {
        let mut more = pixel_layer(&format!("clipped {i}"), 4500, 1152, (30, 40 + i as u8, 200));
        more.clipping = true;
        layers.push(more);
    }
    layers.push(sibling);
    layers.push(sibling2);
    for i in 0..extra_layers {
        // Full-canvas solid fills: cheap to hold, but each one costs a
        // full-canvas pass, which is what the reported document pays per layer.
        layers.push(Layer {
            name: format!("extra {i}"),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: doc.height as i32,
                right: doc.width as i32,
            },
            adjustment: Some(pictura_render::encode_solid_color_fill([
                20 + (i % 200) as u8,
                60,
                120,
            ])),
            ..Default::default()
        });
    }
    doc.layers = LayerStack::from_layers(layers);
    // Sized blank composite: the profile measures the edit, not the open.
    doc.composite = pictura_core::PixelBuffer::new(doc.width, doc.height, 4);
    doc
}

fn clipping_move_damage(doc: &Document, path: &str, delta: i32) -> (Option<PsdRect>, f64) {
    let mut doc = doc.clone();
    let before = pictura_render::damage::influence(&doc, path);
    let t = std::time::Instant::now();
    assert!(
        pictura_render::move_path(&mut doc, path, delta),
        "move {path} by {delta}"
    );
    let mutation_ms = t.elapsed().as_secs_f64() * 1000.0;
    let index = path.parse::<i32>().expect("numeric path");
    let landed = (index + delta).to_string();
    let after = pictura_render::damage::influence(&doc, &landed);
    (pictura_render::damage::union(before, after), mutation_ms)
}

fn apply_clipping_move(
    doc: Document,
    path: &str,
    delta: i32,
    damage: Option<PsdRect>,
    gpu: bool,
) -> f64 {
    crate::cxxqt_object::metrics::set_for_this_thread(true);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: gpu,
        ..Default::default()
    };
    assert!(pictura_render::move_path(
        view.session.doc.as_mut().expect("document"),
        path,
        delta
    ));
    let t = std::time::Instant::now();
    view.apply_edit(
        "Move Layer",
        Edit::Structural {
            damage: damage.map(|rect| vec![rect]),
        },
    );
    t.elapsed().as_secs_f64() * 1000.0
}

fn run_clipping_move(
    gpu: bool,
    path: &str,
    delta: i32,
    clipped_layers: usize,
    extra_layers: usize,
) {
    let doc = clipping_move_doc(clipped_layers, extra_layers);
    let (damage, mutation_ms) = clipping_move_damage(&doc, path, delta);
    let bounded_ms = apply_clipping_move(doc.clone(), path, delta, damage, gpu);
    let full_ms = apply_clipping_move(doc, path, delta, None, gpu);
    println!(
        "clipping_move gpu={gpu} clipped={clipped_layers} extra={extra_layers} path={path} damage_w={} damage_h={} mutation_ms={mutation_ms:.3} bounded_ms={bounded_ms:.3} full_ms={full_ms:.3}",
        damage.map_or(0, |r| r.width()),
        damage.map_or(0, |r| r.height()),
    );
}

#[test]
#[ignore = "4500x1152 clipping structural-move profile; run explicitly with --ignored --nocapture"]
fn structural_move_clipping_profile() {
    // Within-run reorder: the base's reach bounds the full-canvas member.
    run_clipping_move(false, "2", 1, 1, 0);
    run_clipping_move(true, "2", 1, 1, 0);
    // Releasing the run (moving the base out) stays full-canvas by design.
    run_clipping_move(false, "1", 1, 1, 0);
    // The user's scale: a few full-canvas clipped layers under a small base,
    // surrounded by many ordinary small layers.
    run_clipping_move(false, "2", 1, 1, 200);
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
    let _ = pictura_render::composite(pictura_render::CompositeRequest::region(
        &doc, damage, false,
    ));
    ms("mask_feather preview", t.elapsed());
    println!("mask_feather damage={damage:?}");
}

/// The total of one named phase in a fresh (post-`reset`) metrics snapshot.
fn phase_total(json: &str, name: &str) -> Option<f64> {
    let key = format!("\"{name}\":{{");
    let at = json.find(&key)? + key.len();
    let tail = &json[at..];
    let total = tail.find("\"total_ms\":")? + "\"total_ms\":".len();
    let tail = &tail[total..];
    let end = tail.find([',', '}']).unwrap_or(tail.len());
    tail[..end].parse().ok()
}

fn report_edit(tag: &str, kind: &str, wall_ms: f64, json: &str) {
    let full = phase_total(json, "recomposite_full_composite");
    let total = phase_total(json, "recomposite_total");
    let regional =
        json.contains("\"commit_refresh_region\"") || json.contains("\"rr_composite_region\"");
    println!(
        "real_file_clipping_insert {tag} edit={kind} wall_ms={wall_ms:.3} recomposite_full_composite_ms={} recomposite_total_ms={} path={}",
        full.map_or_else(|| "none".to_string(), |v| format!("{v:.3}")),
        total.map_or_else(|| "none".to_string(), |v| format!("{v:.3}")),
        if regional {
            "refresh_regions"
        } else if full.is_some() {
            "composite_full"
        } else {
            "none"
        },
    );
}

fn edit_kind(edit: &Edit) -> &'static str {
    match edit {
        Edit::Inserted => "Inserted",
        Edit::Structural { .. } => "Structural",
        _ => "other",
    }
}

/// Insert at the root top, move into the run, and time the app's move edit.
fn time_move(doc: Document, source: &str) -> (f64, String, &'static str) {
    crate::cxxqt_object::metrics::set_for_this_thread(true);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    let before_doc = view.session.doc.clone();
    let before = before_doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, source));
    let moved = pictura_render::move_path_to_result(
        view.session.doc.as_mut().expect("document"),
        source,
        "2/0/2",
        1,
    )
    .expect("move into the run");
    assert_eq!(moved, "2/0/2", "the moved layer lands in the run");
    let edit = super::super::impl_layers::move_edit(
        before_doc.as_ref(),
        view.session.doc.as_ref(),
        source,
        Some(&moved),
        before,
    );
    let kind = edit_kind(&edit);
    let t = std::time::Instant::now();
    view.apply_edit("Move Layer", edit);
    let wall_ms = t.elapsed().as_secs_f64() * 1000.0;
    let json = crate::cxxqt_object::metrics::snapshot_json(true).unwrap_or_default();
    (wall_ms, json, kind)
}

#[test]
#[ignore = "real document clipping insert; set PICTURA_PROFILE_PSD and run with --ignored --nocapture"]
fn real_file_clipping_insert_profile() {
    let Some(path) = std::env::var_os("PICTURA_PROFILE_PSD").map(std::path::PathBuf::from) else {
        eprintln!("real_file_clipping_insert: skip, PICTURA_PROFILE_PSD is not set");
        return;
    };
    if !path.is_file() {
        eprintln!(
            "real_file_clipping_insert: skip, missing {}",
            path.display()
        );
        return;
    }
    let bytes = std::fs::read(&path).expect("read profile PSD");
    let original = pictura_codec::read_psd(&bytes).expect("parse profile PSD");
    println!(
        "real_file_clipping_insert file={} size={}x{}",
        path.display(),
        original.width,
        original.height
    );
    for path in ["2/0/1", "2/0/2"] {
        let layer = pictura_render::resolve_path(&original, path);
        println!(
            "real_file_clipping_insert original path={path} name={:?} clipping={:?}",
            layer.map(|l| l.name.as_str()),
            layer.map(|l| l.clipping),
        );
    }

    // The user action: create a new empty layer, then drag it into the clipping
    // run between `2/0/1` (base) and `2/0/2` (Layer 3 copy). Inserting at the
    // root top is content-neutral, so the insertion itself composites nothing.
    let mut inserted = original.clone();
    let before_path = pictura_render::add_layer_in(&mut inserted, "", "");
    assert!(!before_path.is_empty(), "new layer inserted");
    let insertion = super::super::edit::insertion_edit(&inserted, &before_path);
    let insertion_kind = match &insertion {
        Edit::Inserted => "Inserted",
        Edit::Structural { .. } => "Structural",
        _ => "other",
    };
    println!("real_file_clipping_insert insertion edit={insertion_kind} path={before_path}");

    // The task's literal `add_layer_in(doc, "2/0/1", "")` lands at `2/0/2`,
    // which is also the suggested move target, so that move is refused. Probe
    // it so the path difference is visible rather than assumed.
    let mut literal = original.clone();
    let literal_path = pictura_render::add_layer_in(&mut literal, "2/0/1", "");
    let literal_move = pictura_render::move_path_to_result(&mut literal, &literal_path, "2/0/2", 1);
    let literal_edit = super::super::edit::insertion_edit(&literal, &literal_path);
    println!(
        "real_file_clipping_insert literal_insert_path={literal_path} literal_insert_edit={} literal_move_to_2/0/2={literal_move:?}",
        match &literal_edit {
            Edit::Inserted => "Inserted",
            Edit::Structural { .. } => "Structural",
            _ => "other",
        },
    );

    crate::cxxqt_object::metrics::set_for_this_thread(true);
    let mut insert_view = PictureViewRust {
        session: DocumentSession::with_document(inserted.clone()),
        gpu_compute: false,
        ..Default::default()
    };
    let t = std::time::Instant::now();
    insert_view.apply_edit("New Layer", insertion);
    let insert_wall = t.elapsed().as_secs_f64() * 1000.0;
    let insert_json = crate::cxxqt_object::metrics::snapshot_json(true).unwrap_or_default();
    report_edit("insert", insertion_kind, insert_wall, &insert_json);

    let source_before = pictura_render::damage::influence(&inserted, &before_path);
    let slot_before = pictura_render::damage::influence(&original, "2/0/2");
    let mut moved = inserted.clone();
    let after_path = pictura_render::move_path_to_result(&mut moved, &before_path, "2/0/2", 1)
        .expect("move into the run");
    let after = pictura_render::damage::influence(&moved, &after_path);
    let source_union = pictura_render::damage::union(source_before, after);
    let slot_union = pictura_render::damage::union(slot_before, after);
    let base_rect = pictura_render::resolve_path(&moved, "2/0/1").map(|l| l.rect);
    let clipped_rect = pictura_render::resolve_path(&moved, "2/0/3").map(|l| l.rect);
    let clipping = pictura_render::resolve_path(&moved, &after_path).is_some_and(|l| l.clipping);
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: moved.height as i32,
        right: moved.width as i32,
    };
    println!(
        "real_file_clipping_insert after_path={after_path} clipping={clipping} base_rect={base_rect:?} clipped_rect={clipped_rect:?}"
    );
    println!(
        "real_file_clipping_insert source_before={source_before:?} slot_before={slot_before:?} after={after:?} source_union={source_union:?} slot_union={slot_union:?}"
    );

    assert!(clipping, "the new layer joins the clipping run");
    let after = after.expect("the joined run is bounded");
    assert_ne!(after, canvas, "the joined run must not be the full canvas");
    let slot_union = slot_union.expect("the slot reach is bounded");
    assert_ne!(
        slot_union, canvas,
        "the slot reach must not be the full canvas"
    );
    match source_union {
        Some(rect) if rect == canvas => {
            eprintln!(
                "NOTE: the damage union is still full canvas because the new document-sized layer is its own full-canvas reach before it joins, but the move is content-neutral, so the app skips compositing"
            );
        }
        Some(_) => {}
        None => eprintln!("NOTE: the app-equivalent union is unbounded"),
    }

    // The move itself: insert at the root top, then move into the run. The
    // app's move edit should now be `Edit::Inserted`, so it composites nothing.
    let (move_wall, move_json, move_kind) = time_move(inserted, &before_path);
    report_edit("move", move_kind, move_wall, &move_json);
    assert_eq!(move_kind, "Inserted", "the neutral move must not composite");
    assert!(
        phase_total(&move_json, "recomposite_full_composite").is_none(),
        "the neutral move ran a full composite"
    );
    assert!(
        !move_json.contains("\"commit_refresh_region\"")
            && !move_json.contains("\"rr_composite_region\""),
        "the neutral move ran a regional composite"
    );
}
