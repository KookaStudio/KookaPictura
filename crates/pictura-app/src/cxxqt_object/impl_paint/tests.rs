use super::{gpu_paint_mode, preview_blend, preview_level, rasterize_preview};
use crate::cxxqt_object::edit::{effect_of, Edit, Effect};
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
    assert_eq!(effect_of(&edit), Effect::Blit { notify: false });
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
    assert_eq!(effect_of(&edit), Effect::Blit { notify: false });
    assert_eq!(edit.history_hint(), Some(hint.as_slice()));
}
