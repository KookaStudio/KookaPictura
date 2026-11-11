use super::{gpu_paint_mode, preview_blend, preview_level, rasterize_preview};
use crate::cxxqt_object::edit::{presentation_of, Edit};
use crate::cxxqt_object::helpers::TileSet;
use pictura_core::PsdRect;
use pictura_paint::{tip_coverage, PaintMode, Rgba, StrokeConfig, StrokeSample};
use pictura_render::gpu::GpuPaintMode;

use super::PreviewStroke;

fn rect(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn preview(
    w: u32,
    h: u32,
    level: u32,
    snapshot: Vec<u8>,
    coverage: Vec<u8>,
    cfg: StrokeConfig,
) -> PreviewStroke {
    PreviewStroke {
        level,
        scale: 1 << level,
        width: w,
        height: h,
        snapshot,
        coverage,
        extent: Some(rect(w, h)),
        samples: Vec::new(),
        cfg,
        mask: None,
    }
}

#[test]
fn gpu_mode_covers_normal_and_clear_only() {
    assert_eq!(
        gpu_paint_mode(PaintMode::Normal),
        Some(GpuPaintMode::Normal)
    );
    assert_eq!(gpu_paint_mode(PaintMode::Clear), Some(GpuPaintMode::Clear));
    assert_eq!(gpu_paint_mode(PaintMode::Dissolve), None);
    assert_eq!(gpu_paint_mode(PaintMode::Behind), None);
    assert_eq!(gpu_paint_mode(PaintMode::Multiply), None);
}

#[test]
fn a_dab_that_fits_the_budget_is_not_previewed() {
    assert_eq!(preview_level(500, 4000, 4000), None, "500 px fits");
    assert_eq!(preview_level(1, 4000, 4000), None);
    assert_eq!(preview_level(0, 4000, 4000), None);
}

#[test]
fn a_dab_over_the_budget_previews_at_a_level_that_fits() {
    for (diameter, w, h) in [
        (600, 4000, 4000),
        (1024, 4000, 4000),
        (2000, 4000, 4000),
        (5000, 4000, 4000),
        (5000, 16000, 16000),
    ] {
        let level = preview_level(diameter, w, h)
            .unwrap_or_else(|| panic!("{diameter} on {w}x{h} must preview"));
        let d = (diameter as u64 + 2).min(w as u64).min(h as u64);
        let area = d * d;
        assert!(
            (area >> (2 * level)) <= 262_144,
            "{diameter} on {w}x{h} previews at {level} with {} px",
            area >> (2 * level)
        );
    }
}

#[test]
fn a_bounded_brush_previews_at_the_smallest_level_that_fits() {
    // A 16000² document with a 5000 px brush is the one case the starting
    // level does not cover on its own.
    assert_eq!(preview_level(5000, 4000, 4000), Some(3));
    assert_eq!(preview_level(5000, 16000, 16000), Some(4));
    assert_eq!(preview_level(600, 4000, 4000), Some(3));
}

#[test]
fn preview_blend_is_source_over_and_clear_removes_the_snapshot() {
    let snapshot = vec![10u8, 20, 30, 255];
    let blue = Rgba {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    let normal = |coverage| {
        let cfg = StrokeConfig {
            color: blue,
            mode: PaintMode::Normal,
            ..Default::default()
        };
        preview(1, 1, 3, snapshot.clone(), vec![coverage], cfg)
    };
    assert_eq!(
        preview_blend(&normal(0), rect(1, 1)),
        snapshot,
        "zero coverage leaves the snapshot untouched"
    );
    assert_eq!(
        preview_blend(&normal(255), rect(1, 1)),
        vec![0, 0, 255, 255],
        "full opaque coverage is exactly the paint colour"
    );

    let clear = preview(
        1,
        1,
        3,
        snapshot.clone(),
        vec![255],
        StrokeConfig {
            color: blue,
            mode: PaintMode::Clear,
            ..Default::default()
        },
    );
    assert_eq!(
        preview_blend(&clear, rect(1, 1)),
        vec![0, 0, 0, 0],
        "Clear removes the snapshot's coverage"
    );

    // Clear ignores the foreground alpha (the exact stroke does), so a
    // semi-transparent colour clears exactly as an opaque one does.
    let clear_translucent = preview(
        1,
        1,
        3,
        snapshot.clone(),
        vec![255],
        StrokeConfig {
            color: Rgba {
                r: 0,
                g: 0,
                b: 255,
                a: 128,
            },
            mode: PaintMode::Clear,
            ..Default::default()
        },
    );
    assert_eq!(
        preview_blend(&clear_translucent, rect(1, 1)),
        preview_blend(&clear, rect(1, 1)),
        "Clear ignores the foreground alpha"
    );
}

#[test]
fn rasterize_preview_samples_the_tip_at_scaled_offsets() {
    let cfg = StrokeConfig {
        diameter: 512,
        hardness: 100,
        ..Default::default()
    };
    let (w, h) = (64u32, 64u32);
    let mut p = preview(
        w,
        h,
        3,
        vec![0; (w * h * 4) as usize],
        vec![0; (w * h) as usize],
        cfg,
    );
    // The sample is document-space; level 3 means an 8x reduction.
    let centre = 32.0 * p.scale as f32;
    assert!(rasterize_preview(
        &mut p,
        StrokeSample {
            x: centre,
            y: centre,
            pressure: 1.0,
        }
    ));
    let flow = cfg.flow as f32 / 100.0;
    assert_eq!(p.coverage[32 * w as usize + 32], 255, "centre is full");
    for (lx, ly) in [(32usize, 20usize), (40, 32), (20, 20), (32, 44)] {
        let dx = (lx as f32 + 0.5 - 32.0) * p.scale as f32;
        let dy = (ly as f32 + 0.5 - 32.0) * p.scale as f32;
        let tip = tip_coverage(&cfg, dx, dy);
        let expected = ((flow * tip).clamp(0.0, 1.0) * 255.0).round() as u8;
        assert_eq!(
            p.coverage[ly * w as usize + lx],
            expected,
            "reduced coverage at level pixel ({lx},{ly}) is the full tip at its scaled offset"
        );
    }
}

#[test]
fn paint_commit_hints_all_stroke_tiles() {
    let mut tiles = TileSet::default();
    tiles.reset(128, 128);
    tiles.mark(PsdRect {
        top: 0,
        left: 0,
        bottom: 64,
        right: 64,
    });
    tiles.mark(PsdRect {
        top: 64,
        left: 64,
        bottom: 128,
        right: 128,
    });
    let bounds = tiles.bounds().expect("the stroke covers tiles");
    assert_eq!(
        (bounds.left, bounds.top, bounds.right, bounds.bottom),
        (0, 0, 128, 128)
    );
    let hint = tiles.regions(bounds);
    assert!(!hint.is_empty());
    let edit = Edit::regions_hinted(vec![], hint.clone(), false);
    let presentation = presentation_of(&edit, None, true);
    assert!(presentation.regions.is_empty());
    assert!(!presentation.notify);
    assert_eq!(edit.history_hint(), Some(hint.as_slice()));
}

#[test]
fn paint_commit_with_no_pending_region_hints_the_whole_stroke() {
    let mut tiles = TileSet::default();
    tiles.reset(128, 128);
    tiles.mark(PsdRect {
        top: 0,
        left: 0,
        bottom: 64,
        right: 64,
    });
    let bounds = tiles.bounds().expect("the stroke covers tiles");
    let hint = tiles.regions(bounds);
    let edit = Edit::regions_hinted(Vec::new(), hint.clone(), false);
    let presentation = presentation_of(&edit, None, true);
    assert!(presentation.regions.is_empty());
    assert!(!presentation.notify);
    assert_eq!(edit.history_hint(), Some(hint.as_slice()));
}

#[test]
fn a_mask_present_leaves_the_composite_unchanged() {
    use crate::cxxqt_object::document_session::{DocumentSession, MaskStroke, StrokeSession};
    use crate::cxxqt_object::state::PictureViewRust;
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer};
    use pictura_paint::Stroke;

    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![pixel_layer("0", 4, 4, (10, 20, 30))]);
    let full = pictura_render::composite_rgba(&doc);
    let seed_marker = PixelBuffer {
        width: 4,
        height: 4,
        channels: 4,
        data: vec![3u8; 4 * 4 * 4].into(),
    };
    let before = seed_marker.clone();
    doc.composite = seed_marker;

    let mask_doc = Document::from_rgba("0", 4, 4, &[0u8; 64]);
    let stroke =
        Stroke::begin_at(&mask_doc, "0", StrokeConfig::default()).expect("a mask stroke begins");
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        ..Default::default()
    };
    view.session.live.set_stroke_session(StrokeSession {
        mask_stroke: Some(MaskStroke {
            baseline: None,
            path: "0".to_string(),
            rect: rect(4, 4),
            document: mask_doc,
            stroke,
        }),
        ..Default::default()
    });

    let region = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    crate::cxxqt_object::tests::present_all(&view);
    assert_eq!(view.refresh_display_rect(region), Some(region));
    assert_eq!(
        view.session.doc.as_ref().expect("document").composite.data,
        before.data,
        "a mask stroke renders without writing the composite"
    );
    // The presented tiles follow the layer stack, not the seeded composite.
    let (crop, missing) = view.tile_crop(pictura_render::TileSource::Composite, 0, region);
    assert!(missing.is_empty());
    let plane = (4 * 4) as usize;
    for y in 0..2usize {
        for x in 0..2usize {
            let fi = y * 4 + x;
            let o = (y * 2 + x) * 4;
            let a = full.data[3 * plane + fi];
            let premul = |c: usize| {
                ((u32::from(full.data[c * plane + fi]) * u32::from(a) + 127) / 255) as u8
            };
            assert_eq!(
                &crop.data()[o..o + 4],
                &[premul(0), premul(1), premul(2), a],
                "pixel ({x}, {y})"
            );
        }
    }
}
