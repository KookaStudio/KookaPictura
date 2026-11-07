use super::*;
use crate::cxxqt_object::{document_session::DocumentSession, tests::pixel_layer};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode};

fn fixture() -> PictureViewRust {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers
        .push(pixel_layer("pixels", 8, 8, (20, 40, 60)))
        .unwrap();
    PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        ..Default::default()
    }
}

#[test]
fn quick_mask_none_is_implicit_all() {
    let mut rust = fixture();
    rust.session.quick_mask.active = true;
    assert_eq!(rust.quick_mask_coverage(3, 3), 255);
    assert!(rust.session.selection.is_none());
    rust.session.quick_mask.coverage = Some(Selection::none(8, 8));
    assert_eq!(rust.quick_mask_coverage(3, 3), 0);
}

#[test]
fn quick_mask_untouched_preserves_optional_selection() {
    for selection in [
        None,
        Some(Selection::none(8, 8)),
        Some(Selection::all(8, 8)),
        Some(Selection::rect(8, 8, 2, 2, 3, 3).feather(1.0)),
    ] {
        let mut rust = fixture();
        rust.session.selection = selection.clone();
        assert!(rust.set_quick_mask_mode(true));
        assert!(rust.set_quick_mask_mode(false));
        assert_eq!(rust.session.selection, selection);
        assert_eq!(rust.session.history.count(), 0);
    }
}

#[test]
fn quick_mask_paint_requires_layer_target() {
    let mut rust = fixture();
    rust.set_quick_mask_mode(true);
    rust.layer_target = LayerTarget::default();
    assert!(!rust.begin_quick_mask_paint(StrokeConfig::default(), "Brush"));
    assert!(rust.session.live.kind().is_none());
    assert!(rust.session.selection.is_none());
}

#[test]
fn quick_mask_blend_respects_polarity() {
    for selected in [false, true] {
        for mode in [
            pictura_paint::PaintMode::Normal,
            pictura_paint::PaintMode::Multiply,
        ] {
            let mut rust = fixture();
            rust.set_quick_mask_mode(true);
            rust.session.quick_mask.selected = selected;
            rust.session.quick_mask.coverage = Some(Selection {
                width: 8,
                height: 8,
                data: vec![128; 64].into(),
            });
            let before = rust.session.doc.clone();
            let cfg = StrokeConfig {
                diameter: 8,
                hardness: 100,
                mode,
                ..Default::default()
            };
            assert!(rust.begin_quick_mask_paint(cfg, "Brush"));
            assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
            assert_eq!(
                rust.quick_mask_coverage(4, 4),
                if selected { 255 } else { 0 }
            );
            assert!(rust.finish_quick_mask_paint());
            assert_eq!(
                rust.quick_mask_coverage(4, 4),
                if selected { 255 } else { 0 }
            );
            assert_eq!(rust.session.doc, before);
        }
    }
}

#[test]
fn quick_mask_history_preserves_none_and_coverage() {
    let mut rust = fixture();
    rust.set_quick_mask_mode(true);
    rust.session.record(&rust.layer_target, "Initial");
    assert!(rust.begin_quick_mask_paint(
        StrokeConfig {
            diameter: 8,
            hardness: 100,
            ..Default::default()
        },
        "Brush"
    ));
    assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
    assert!(rust.finish_quick_mask_paint());
    rust.apply_edit("Brush", Edit::Meta);
    let edited = rust.session.quick_mask.coverage.clone();
    let restored = rust.session.history.undo().unwrap();
    assert_eq!(restored.quick_mask, Some(None));
    let restored = rust.session.history.redo().unwrap();
    assert_eq!(restored.quick_mask, Some(edited));
}

#[test]
fn quick_mask_content_moves_refuse() {
    let mut rust = fixture();
    rust.set_quick_mask_mode(true);
    assert!(!rust.quick_mask_content_allowed());
    rust.set_quick_mask_mode(false);
    assert!(rust.quick_mask_content_allowed());
}

#[test]
fn quick_mask_presentation_avoids_materialization() {
    let mut rust = fixture();
    rust.set_quick_mask_mode(true);
    let mut selection = Selection::all(8, 8);
    selection.data.fracture(8);
    rust.session.quick_mask.coverage = Some(selection);
    pictura_core::assert_no_materialization(|| {
        assert_eq!(rust.quick_mask_coverage(4, 4), 255);
        rust.session.quick_mask.selected = true;
        assert!(rust.begin_quick_mask_paint(
            StrokeConfig {
                diameter: 8,
                hardness: 100,
                color: pictura_paint::Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255
                },
                ..Default::default()
            },
            "Brush"
        ));
        assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
        assert_eq!(rust.quick_mask_coverage(4, 4), 0);
        assert!(rust.finish_quick_mask_paint());
    });
}

#[test]
fn quick_mask_save_keeps_document_data() {
    let mut rust = fixture();
    let before = rust.session.doc.clone();
    rust.set_quick_mask_mode(true);
    assert!(rust.begin_quick_mask_paint(
        StrokeConfig {
            diameter: 8,
            hardness: 100,
            ..Default::default()
        },
        "Brush"
    ));
    rust.sample_quick_mask(4.0, 4.0, 1.0, false);
    rust.finish_quick_mask_paint();
    assert_eq!(rust.session.doc, before);
    assert!(rust.session.doc.as_ref().unwrap().channels.is_empty());
}

#[test]
fn quick_mask_clipping_keeps_coverage_independent() {
    let mut rust = fixture();
    rust.set_quick_mask_mode(true);
    let clip = Selection::rect(8, 8, 0, 0, 4, 8);
    rust.session.set_selection(Some(clip.clone()));
    assert!(rust.begin_quick_mask_paint(
        StrokeConfig {
            diameter: 8,
            hardness: 100,
            ..Default::default()
        },
        "Brush"
    ));
    assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
    assert_eq!(rust.quick_mask_coverage(2, 4), 0);
    assert_eq!(rust.quick_mask_coverage(6, 4), 255);
    assert!(rust.finish_quick_mask_paint());
    assert_eq!(rust.quick_mask_coverage(2, 4), 0);
    assert_eq!(rust.quick_mask_coverage(6, 4), 255);
    assert_eq!(rust.session.selection, Some(clip));
    rust.set_quick_mask_mode(false);
    assert_eq!(
        rust.session
            .selection
            .as_ref()
            .unwrap()
            .data
            .sample(4 * 8 + 6),
        Some(255)
    );
}
#[test]
fn quick_mask_independent_history() {
    use crate::history::History;
    let mut rust = fixture();
    rust.session.record(&rust.layer_target, "Initial");
    rust.set_quick_mask_mode(true);
    let clip = Selection::rect(8, 8, 0, 0, 4, 8);
    rust.session.set_selection(Some(clip.clone()));
    rust.session.record(&rust.layer_target, "Clip");
    assert!(rust.begin_quick_mask_paint(
        StrokeConfig {
            diameter: 8,
            hardness: 100,
            ..Default::default()
        },
        "Brush"
    ));
    assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
    assert!(rust.finish_quick_mask_paint());
    rust.session.record(&rust.layer_target, "Paint");
    rust.session
        .restore(&mut rust.layer_target, History::undo_live, History::undo)
        .unwrap();
    assert!(rust.session.quick_mask.coverage.is_none());
    assert_eq!(rust.session.selection, Some(clip.clone()));
    rust.session
        .restore(&mut rust.layer_target, History::undo_live, History::undo)
        .unwrap();
    assert!(rust.session.selection.is_none());
    assert!(rust.session.quick_mask.coverage.is_none());
    rust.session
        .restore(&mut rust.layer_target, History::redo_live, History::redo)
        .unwrap();
    assert_eq!(rust.session.selection, Some(clip));
    rust.session
        .restore(&mut rust.layer_target, History::redo_live, History::redo)
        .unwrap();
    assert_eq!(rust.quick_mask_coverage(2, 4), 0);
    assert_eq!(rust.quick_mask_coverage(6, 4), 255);
    rust.set_quick_mask_mode(false);
    rust.session
        .restore(&mut rust.layer_target, History::undo_live, History::undo)
        .unwrap();
    assert!(rust.session.selection.is_none());
    rust.session
        .restore(&mut rust.layer_target, History::redo_live, History::redo)
        .unwrap();
    assert_eq!(
        rust.session
            .selection
            .as_ref()
            .unwrap()
            .data
            .sample(4 * 8 + 6),
        Some(255)
    );
}

#[test]
fn quick_mask_soft_clip_is_applied_once() {
    let mut rust = fixture();
    rust.set_quick_mask_mode(true);
    rust.session.selection = Some(Selection {
        width: 8,
        height: 8,
        data: vec![64; 64].into(),
    });
    assert!(rust.begin_quick_mask_paint(
        StrokeConfig {
            diameter: 8,
            hardness: 100,
            ..Default::default()
        },
        "Brush"
    ));
    assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
    assert_eq!(rust.quick_mask_coverage(4, 4), 191);
    rust.sample_quick_mask(5.0, 4.0, 1.0, false);
    assert_eq!(rust.quick_mask_coverage(4, 4), 191);
    assert!(rust.finish_quick_mask_paint());
    assert_eq!(rust.quick_mask_coverage(4, 4), 191);
}
