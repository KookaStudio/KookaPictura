use super::*;
use crate::cxxqt_object::edit::{insertion_edit, Edit};
use crate::cxxqt_object::tests::pixel_layer;
use crate::cxxqt_object::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
use pictura_render::{FrameTileSample, TilePyramid, TileSource};

/// A CPU-only view over a two-layer document whose stored composite is current.
fn cpu_view() -> PictureViewRust {
    let mut doc = Document::new(8, 6, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("base", 8, 6, (200, 40, 10)),
        pixel_layer("top", 8, 6, (20, 90, 160)),
    ]);
    let view = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    crate::cxxqt_object::tests::present_all(&view);
    view
}

/// What the New Layer handler does inside its edit closure.
fn new_layer(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let name = pictura_render::next_layer_name(doc, "Layer");
    let created = pictura_render::add_layer(doc, 1, &name);
    assert!(created >= 0);
    let edit = insertion_edit(doc, &created.to_string());
    view.apply_edit("New Layer", edit.clone());
    edit
}

#[test]
fn metrics_off_retains_nothing() {
    let mut m = Metrics::new(false);
    m.begin_edit("Edit");
    m.record_phase("recomposite_full_composite", Duration::from_millis(3));
    m.mark_composited();
    assert!(m.end_edit().is_none());
    assert!(m.phases().is_empty());
    assert_eq!(m.edits().count(), 0);
    assert!(m.snapshot_json(false).is_none());
}

#[test]
fn metrics_on_and_off_composite_identically() {
    let run = |on: bool| {
        set_for_this_thread(on);
        let mut view = cpu_view();
        new_layer(&mut view);
        view.session.doc.as_mut().expect("document").layers[0].opacity = 90;
        view.apply_edit("Opacity", Edit::Full);
        view.session
            .doc
            .as_ref()
            .expect("document")
            .composite
            .buffer()
            .clone()
    };
    let on = run(true);
    let off = run(false);
    assert_eq!(on, off);
}

#[test]
fn full_recomposite_is_counted() {
    set_for_this_thread(true);
    let mut view = cpu_view();
    let count = || {
        with(|m| {
            m.phases()
                .iter()
                .find(|(l, _)| l == "recomposite_full_composite")
                .map_or(0, |(_, p)| p.count)
        })
    };
    let before = count();
    view.apply_edit("Edit", Edit::Full);
    assert_eq!(count(), before + 1);
}

#[test]
fn bridge_phase_is_recorded() {
    set_for_this_thread(true);
    record_bridge_phase("layers_panel_refresh", 1500);
    let phase = with(|m| {
        m.phases()
            .iter()
            .find(|(l, _)| l == "layers_panel_refresh")
            .map(|(_, p)| *p)
    })
    .expect("phase recorded");
    assert_eq!(phase.count, 1);
    assert_eq!(phase.total, Duration::from_micros(1500));
}

#[test]
fn new_layer_produces_one_edit_record() {
    set_for_this_thread(true);
    let mut view = cpu_view();
    new_layer(&mut view);
    let labels: Vec<String> = with(|m| m.edits().map(|e| e.label.clone()).collect());
    assert_eq!(labels, vec!["New Layer".to_string()]);
}

#[test]
fn compositeless_edit_reports_no_composite() {
    set_for_this_thread(true);
    let mut view = cpu_view();
    assert_eq!(new_layer(&mut view), Edit::Inserted);
    let record = with(|m| m.edits().last().cloned()).expect("record");
    assert!(!record.composited);
    assert!(record
        .phases
        .iter()
        .all(|(label, _)| !label.contains("composite")));
}

#[test]
fn record_without_device_time_omits_field() {
    let record = EditRecord {
        label: "New Layer".into(),
        ..EditRecord::default()
    };
    assert!(!edit_line(&record).contains("gpu_ms="));
    let timed = EditRecord {
        gpu_ms: Some(1.25),
        ..record
    };
    assert!(edit_line(&timed).contains("gpu_ms=1.250"));
}

#[test]
fn edit_record_line_format() {
    let record = EditRecord {
        label: "New Layer".into(),
        composited: true,
        wall: Duration::from_micros(2500),
        phases: vec![(
            "history_capture".into(),
            Phase {
                count: 1,
                total: Duration::from_micros(500),
                max: Duration::from_micros(500),
            },
        )],
        canvas_bytes: 64,
        gpu_ms: None,
    };
    assert_eq!(
        edit_line(&record),
        "pictura metrics: edit label=\"New Layer\" composited=true wall_ms=2.500 \
         canvas_bytes=64 history_capture_ms=0.500"
    );
}

#[test]
fn snapshot_json_resets_on_request() {
    let mut m = Metrics::new(true);
    m.begin_edit("Edit \"quoted\"");
    m.record_phase("history_capture", Duration::from_millis(1));
    m.end_edit();
    let json = m.snapshot_json(true).expect("enabled");
    assert!(json.starts_with("{\"enabled\":true,\"phases\":{\"history_capture\":"));
    assert!(json.contains("\"label\":\"Edit \\\"quoted\\\"\""));
    let after = m.snapshot_json(false).expect("enabled");
    assert!(after.contains("\"phases\":{}") && after.contains("\"edits\":[]"));
}

#[test]
fn disabled_frame_metrics_skip_capture() {
    let mut metrics = Metrics::new(false);
    metrics.record_frame(|| panic!("disabled capture must not read the clock"));
    assert!(metrics.frames.is_none());
    assert!(metrics.snapshot_json(false).is_none());
}

#[test]
fn frame_history_allocates_only_on_first_frame() {
    let mut metrics = Metrics::new(true);
    metrics.record_phase("worker", Duration::from_millis(1));
    assert!(metrics.frames.is_none());
    assert!(metrics
        .snapshot_json(true)
        .unwrap()
        .contains("\"frames\":[]"));
    assert!(metrics.frames.is_none());
    metrics.record_frame(FrameRecord::default);
    let capacity = metrics.frames.as_ref().unwrap().capacity();
    assert!(capacity >= FRAME_RING);
    metrics.snapshot_json(true);
    assert_eq!(metrics.frames.as_ref().unwrap().capacity(), capacity);
}

#[test]
fn frame_history_is_bounded_and_resettable() {
    let mut metrics = Metrics::new(true);
    let generation = metrics.frame_generation;
    for canvas_id in 1..=1026 {
        metrics.record_frame(|| FrameRecord {
            canvas_id,
            ..FrameRecord::default()
        });
    }
    let snapshot = metrics.snapshot_json(true).unwrap();
    let first = snapshot.find("\"canvas_id\":3,").unwrap();
    let last = snapshot.find("\"canvas_id\":1026,").unwrap();
    assert!(first < last);
    assert!(!snapshot.contains("\"canvas_id\":2,"));
    assert_eq!(snapshot.matches("\"canvas_id\":").count(), 1024);
    assert!(snapshot.contains("\"frame_records_evicted\":2"));
    let after = metrics.snapshot_json(false).unwrap();
    assert!(after.contains("\"frames\":[]"));
    assert!(after.contains("\"frame_records_evicted\":0"));
    assert_ne!(metrics.frame_generation, generation);
}

fn frame_rect(left: i32, top: i32, right: i32, bottom: i32) -> pictura_core::PsdRect {
    pictura_core::PsdRect {
        left,
        top,
        right,
        bottom,
    }
}

fn frame_tiles(level: usize, rect: pictura_core::PsdRect) -> TilePyramid {
    let mut tiles = TilePyramid::new(512, 512, u64::MAX);
    for key in tiles.keys_in(TileSource::Composite, level, rect) {
        let tile = tiles.tile_rect(key);
        tiles.insert(
            key,
            vec![255; (tile.width() * tile.height() * 4) as usize].into(),
        );
    }
    tiles
}

fn take_in(
    samples: &pictura_render::FrameTileSamples,
    rect: pictura_core::PsdRect,
    generation: u64,
) -> Option<u64> {
    samples
        .rects()
        .enumerate()
        .filter_map(|(index, covered)| {
            covered.intersect(rect)?;
            samples.take(index, generation)
        })
        .reduce(u64::saturating_add)
}

#[test]
fn frame_gpu_samples_follow_drawn_tiles() {
    let rect = frame_rect(0, 0, 512, 256);
    let mut tiles = frame_tiles(0, rect);
    let pixels = pictura_core::PixelBuffer::new(512, 256, 4);
    let sample = FrameTileSample::new(3, 1_250_000);
    tiles.repair_with_sample(TileSource::Composite, rect, &pixels, Some(&sample));
    let inset = frame_rect(4, 4, 12, 12);
    let second = FrameTileSample::new(3, 2_000_000);
    tiles.repair_with_sample(
        TileSource::Composite,
        inset,
        &pictura_core::PixelBuffer::new(8, 8, 4),
        Some(&second),
    );
    let mut samples = pictura_render::FrameTileSamples::default();
    tiles.crop_with_samples(TileSource::Composite, 0, rect, Some(&mut samples));
    let gpu_ns = take_in(&samples, rect, 3);
    assert_eq!(gpu_ns, Some(3_250_000));
    assert_eq!(take_in(&samples, rect, 3), None);
    let mut metrics = Metrics::new(true);
    metrics.record_frame(|| FrameRecord {
        paint_ns: 2_000_000,
        gpu_ns,
        ..FrameRecord::default()
    });
    let json = metrics.snapshot_json(false).unwrap();
    assert!(json.contains("\"gpu_composite_ms\":3.250"));
    assert!(json.contains("\"cpu_budget_slack_ms\":14.600"));
}

#[test]
fn frame_gpu_samples_ignore_unpainted_results() {
    let rect = frame_rect(0, 0, 32, 32);
    let mut tiles = frame_tiles(0, rect);
    let pixels = pictura_core::PixelBuffer::new(32, 32, 4);
    let sample = FrameTileSample::new(3, 1_250_000);
    tiles.repair_with_sample(TileSource::Composite, rect, &pixels, Some(&sample));
    let mut samples = pictura_render::FrameTileSamples::default();
    tiles.crop_with_samples(TileSource::Mask, 0, rect, Some(&mut samples));
    assert_eq!(take_in(&samples, rect, 3), None);
    tiles.repair(TileSource::Composite, rect, &pixels);
    tiles.crop_with_samples(TileSource::Composite, 0, rect, Some(&mut samples));
    assert_eq!(take_in(&samples, rect, 3), None);
    let mut metrics = Metrics::new(true);
    metrics.record_frame(FrameRecord::default);
    assert!(!metrics
        .snapshot_json(false)
        .unwrap()
        .contains("gpu_composite_ms"));
}

#[test]
fn frame_gpu_standin_attribution() {
    let shown = frame_rect(0, 0, 8, 8);
    let overwritten = frame_rect(8, 0, 16, 8);
    let mut tiles = frame_tiles(1, frame_rect(0, 0, 16, 8));
    tiles.invalidate();
    let pixels = pictura_core::PixelBuffer::new(8, 8, 4);
    let a = FrameTileSample::new(3, 500_000);
    let b = FrameTileSample::new(3, 800_000);
    tiles.paint_stand_in_with_sample(TileSource::Composite, 1, shown, &pixels, Some(&a));
    tiles.paint_stand_in_with_sample(TileSource::Composite, 1, overwritten, &pixels, Some(&b));
    tiles.paint_stand_in(TileSource::Composite, 1, overwritten, &pixels, 255, false);
    let mut samples = pictura_render::FrameTileSamples::default();
    tiles.crop_with_samples(
        TileSource::Composite,
        1,
        frame_rect(0, 0, 16, 8),
        Some(&mut samples),
    );
    assert_eq!(take_in(&samples, overwritten, 3), None);
    assert_eq!(take_in(&samples, shown, 4), None);
    assert_eq!(take_in(&samples, shown, 3), Some(500_000));
    assert_eq!(take_in(&samples, shown, 3), None);
}

#[test]
fn new_layer_keeps_the_display_exact() {
    set_for_this_thread(false);
    let mut view = cpu_view();
    assert_eq!(new_layer(&mut view), Edit::Inserted);
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(doc.layers.len(), 3);
    crate::cxxqt_object::tests::assert_display_current(&view);
}

#[test]
fn new_group_keeps_the_display_exact() {
    set_for_this_thread(false);
    let mut view = cpu_view();
    let doc = view.session.doc.as_mut().expect("document");
    let created = pictura_render::add_group_in(doc, "0", "");
    assert!(!created.is_empty());
    let edit = insertion_edit(doc, &created);
    assert_eq!(edit, Edit::Inserted);
    view.apply_edit("New Group", edit);
    crate::cxxqt_object::tests::assert_display_current(&view);
}
