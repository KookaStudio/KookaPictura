use super::*;
use crate::cxxqt_object::edit::{insertion_edit, Edit};
use crate::cxxqt_object::tests::pixel_layer;
use crate::cxxqt_object::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};

/// A CPU-only view over a two-layer document whose stored composite is current.
fn cpu_view() -> PictureViewRust {
    let mut doc = Document::new(8, 6, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("base", 8, 6, (200, 40, 10)),
        pixel_layer("top", 8, 6, (20, 90, 160)),
    ]);
    doc.composite = pictura_render::composite_rgba(&doc);
    PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    }
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
    assert!(after.contains("\"phases\":{}") && after.ends_with("\"edits\":[]}"));
}

#[test]
fn new_layer_keeps_composite_exact() {
    set_for_this_thread(false);
    let mut view = cpu_view();
    assert_eq!(new_layer(&mut view), Edit::Inserted);
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(doc.layers.len(), 3);
    assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
}

#[test]
fn new_group_keeps_composite_exact() {
    set_for_this_thread(false);
    let mut view = cpu_view();
    let doc = view.session.doc.as_mut().expect("document");
    let created = pictura_render::add_group_in(doc, "0", "");
    assert!(!created.is_empty());
    let edit = insertion_edit(doc, &created);
    assert_eq!(edit, Edit::Inserted);
    view.apply_edit("New Group", edit);
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
}
