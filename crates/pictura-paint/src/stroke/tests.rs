use super::*;
use crate::healing::RgbaImage;
use crate::spacing::SpacingMode;
use pictura_core::{BitDepth, BlendMode, Channel, ColorLabel, ColorMode, LockFlags};

const BASE: (u8, u8, u8, u8) = (100, 120, 140, 255);

fn layer_doc(w: u32, h: u32, rgba: (u8, u8, u8, u8)) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let n = (w * h) as usize;
    let (r, g, b, a) = rgba;
    {
        let _ = doc.layers.push(Layer {
            name: "px".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: None,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![r; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![g; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![b; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![a; n].into(),
                },
            ],
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        });
    };
    doc
}

fn transparent_doc(w: u32, h: u32) -> Document {
    layer_doc(w, h, (0, 0, 0, 0))
}

fn chan(doc: &Document, id: i16, i: usize) -> u8 {
    channel_data(&doc.layers[0], id).expect("channel")[i]
}

fn max_alpha(doc: &Document) -> u8 {
    *channel_data(&doc.layers[0], -1)
        .expect("alpha channel")
        .iter()
        .max()
        .unwrap()
}

fn sample(x: f32, y: f32) -> StrokeSample {
    StrokeSample {
        x,
        y,
        pressure: 1.0,
    }
}

/// Paint into the single layer at path `"0"`.
fn paint(doc: &mut Document, cfg: &StrokeConfig, samples: &[StrokeSample]) -> Option<PsdRect> {
    paint_stroke(doc, "0", cfg, samples)
}

fn red() -> Rgba {
    Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    }
}

/// A hard, fully flowing stroke in `mode` at `opacity`.
fn mode_cfg(mode: PaintMode, opacity: u8) -> StrokeConfig {
    StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity,
        flow: 100,
        mode,
        ..StrokeConfig::default()
    }
}

#[test]
fn take_dirty_reports_each_dab_not_the_stroke_union() {
    let mut doc = layer_doc(128, 32, BASE);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 8,
        hardness: 100,
        spacing: SpacingMode::Fixed(150),
        opacity: 100,
        flow: 100,
        ..StrokeConfig::default()
    };
    // Diameter 8 at 150% spacing steps 12 px, so each sample places one dab.
    let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin");
    assert!(stroke.sample(&mut doc, sample(50.0, 16.0)));
    let first = stroke.take_dirty().expect("first dab");
    assert!(stroke.sample(&mut doc, sample(62.0, 16.0)));
    let second = stroke.take_dirty().expect("second dab");
    assert!(
        second.left >= first.right,
        "second rect {second:?} overlaps the first {first:?}"
    );
    assert!(
        second.width() <= 12,
        "second rect grew to {}",
        second.width()
    );
    assert!(stroke.take_dirty().is_none(), "take_dirty must clear");
    // The committed union still covers the whole stroke.
    let dirty = stroke.finish().expect("painted");
    assert!(dirty.left <= first.left);
    assert!(dirty.right >= second.right);
}

#[test]
fn horizontal_stroke_changes_pixels_and_reports_dirty() {
    let mut doc = layer_doc(64, 32, BASE);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 8,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let dirty = paint(&mut doc, &cfg, &[sample(2.0, 16.0), sample(40.0, 16.0)]).expect("painted");
    assert!(dirty.width() > 0 && dirty.height() > 0);
    assert!(dirty.top >= 0 && dirty.left >= 0 && dirty.bottom <= 32 && dirty.right <= 64);
    assert!(channel_data(&doc.layers[0], 0)
        .unwrap()
        .iter()
        .any(|&v| v != BASE.0));
    assert!(chan(&doc, 0, 16 * 64 + 20) > BASE.0);
}

#[test]
fn pixel_locked_layer_refuses_stroke_and_writes_nothing() {
    let mut doc = layer_doc(16, 16, BASE);
    doc.layers[0].lock = LockFlags::default().with(LockFlags::PIXELS, true);
    let before = doc.clone();
    let cfg = StrokeConfig {
        color: red(),
        diameter: 6,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let begun = Stroke::begin_at(&doc, "0", cfg);
    assert!(matches!(begun, Err(PaintError::Locked)));
    assert!(paint(&mut doc, &cfg, &[sample(4.0, 4.0), sample(12.0, 4.0)]).is_none());
    assert_eq!(doc, before, "a refused stroke writes no pixels");
}

#[test]
fn transparency_lock_refuses_clear_but_allows_normal() {
    let mut doc = layer_doc(16, 16, BASE);
    doc.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let clear = StrokeConfig {
        color: red(),
        diameter: 6,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        mode: PaintMode::Clear,
        ..StrokeConfig::default()
    };
    let begun = Stroke::begin_at(&doc, "0", clear);
    assert!(matches!(begun, Err(PaintError::Locked)));
    let normal = StrokeConfig {
        mode: PaintMode::Normal,
        ..clear
    };
    assert!(Stroke::begin_at(&doc, "0", normal).is_ok());
}

#[test]
fn transparency_lock_preserves_alpha_per_pixel() {
    let mut doc = layer_doc(2, 1, (10, 20, 30, 180));
    channel_data_mut(&mut doc.layers[0], -1).expect("alpha")[1] = 0;
    doc.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 3,
        hardness: 100,
        spacing: SpacingMode::Fixed(25),
        opacity: 100,
        flow: 100,
        ..StrokeConfig::default()
    };
    paint(&mut doc, &cfg, &[sample(0.5, 0.5)]).expect("painted");
    assert_eq!(chan(&doc, -1, 0), 180, "semi-transparent pixel keeps alpha");
    assert_ne!(
        (chan(&doc, 0, 0), chan(&doc, 1, 0), chan(&doc, 2, 0)),
        (10, 20, 30),
        "semi-transparent pixel keeps its colour edit"
    );
    assert_eq!(
        (
            chan(&doc, 0, 1),
            chan(&doc, 1, 1),
            chan(&doc, 2, 1),
            chan(&doc, -1, 1)
        ),
        (10, 20, 30, 0),
        "fully transparent pixel is untouched"
    );
}

#[test]
fn a_locked_background_without_alpha_takes_paint_as_opaque() {
    let mut doc = layer_doc(32, 32, (255, 255, 255, 255));
    doc.layers[0].channels.retain(|c| c.id != -1);
    doc.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 8,
        hardness: 0,
        ..StrokeConfig::default()
    };
    paint(&mut doc, &cfg, &[sample(16.0, 16.0)]).expect("painted");
    assert!(
        chan(&doc, 1, 16 * 32 + 16) < 64,
        "the centre was not painted"
    );
    // A soft edge blends with the white beneath rather than replacing it.
    let edge = 16 * 32 + 19;
    assert!(chan(&doc, 1, edge) > 0 && chan(&doc, 1, edge) < 255);
}

#[test]
fn dense_spacing_is_continuous_where_sparse_leaves_a_gap() {
    let samples = [sample(10.5, 16.0), sample(30.5, 16.0)];
    let base = |spacing| StrokeConfig {
        color: red(),
        diameter: 20,
        hardness: 100,
        spacing,
        ..StrokeConfig::default()
    };

    let mut dense = transparent_doc(64, 32);
    paint(&mut dense, &base(SpacingMode::Fixed(10)), &samples).expect("dense");
    for lx in 11..=30 {
        assert!(
            chan(&dense, -1, 16 * 64 + lx) > 0,
            "dense left a gap at {lx}"
        );
    }

    let mut sparse = transparent_doc(64, 32);
    paint(&mut sparse, &base(SpacingMode::Fixed(100)), &samples).expect("sparse");
    assert_eq!(
        chan(&sparse, -1, 16 * 64 + 20),
        0,
        "expected a gap at the midpoint"
    );
    assert!(chan(&sparse, -1, 16 * 64 + 19) > 0);
    assert!(chan(&sparse, -1, 16 * 64 + 21) > 0);
}

#[test]
fn opacity_caps_a_single_stroke() {
    let cfg = StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity: 33,
        flow: 100,
        ..StrokeConfig::default()
    };
    let path = [sample(5.0, 16.0), sample(55.0, 16.0), sample(5.0, 16.0)];

    let mut doc = transparent_doc(64, 32);
    paint(&mut doc, &cfg, &path).expect("first");
    let first = max_alpha(&doc);
    assert!(
        first as f32 / 255.0 <= 0.33 + 1.0 / 255.0,
        "single stroke exceeded the opacity cap: {first}"
    );

    paint(&mut doc, &cfg, &path).expect("second");
    let second = max_alpha(&doc);
    assert!(
        second > first,
        "second stroke must raise coverage: {first} -> {second}"
    );
}

#[test]
fn higher_flow_reaches_more_coverage() {
    let base = |flow| StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity: 100,
        flow,
        ..StrokeConfig::default()
    };
    let path = [sample(5.0, 16.0), sample(55.0, 16.0)];

    let mut lo = transparent_doc(64, 32);
    paint(&mut lo, &base(20), &path).expect("lo");
    let mut hi = transparent_doc(64, 32);
    paint(&mut hi, &base(80), &path).expect("hi");

    let sum = |d: &Document| -> u32 {
        channel_data(&d.layers[0], -1)
            .unwrap()
            .iter()
            .map(|&v| v as u32)
            .sum()
    };
    assert!(
        sum(&lo) < sum(&hi),
        "flow 20 {} vs flow 80 {}",
        sum(&lo),
        sum(&hi)
    );
}

#[test]
fn airbrush_soft_round_spacing_keeps_the_stroke_airy() {
    // Issue #731's Airbrush Soft Round 50% flow: 45 px, hardness 0, 50 % flow,
    // 25 % spacing. The wider spacing keeps the core below full opacity; the
    // owner's comparison showed a dense 10 % spacing building a heavy, nearly
    // solid core instead.
    let cfg = |spacing| StrokeConfig {
        color: red(),
        diameter: 45,
        hardness: 0,
        spacing: SpacingMode::Fixed(spacing),
        opacity: 100,
        flow: 50,
        ..StrokeConfig::default()
    };
    let path = [sample(20.0, 64.0), sample(236.0, 64.0)];
    let mut airy = transparent_doc(256, 128);
    paint(&mut airy, &cfg(25), &path).expect("airy");
    let mut dense = transparent_doc(256, 128);
    paint(&mut dense, &cfg(10), &path).expect("dense");

    let peak = max_alpha(&airy);
    assert!(
        (150..=220).contains(&peak),
        "expected a light core, got alpha {peak}"
    );
    let dense_peak = max_alpha(&dense);
    assert!(
        dense_peak > peak + 20,
        "10 % spacing did not build a heavier core: {peak} vs {dense_peak}"
    );
}

#[test]
fn pencil_is_binary_brush_has_soft_edges() {
    let path = [sample(2.0, 16.0), sample(60.0, 16.0)];
    let cfg = |aliased| StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 50,
        spacing: SpacingMode::Fixed(25),
        opacity: 100,
        flow: 100,
        aliased,
        ..StrokeConfig::default()
    };

    let mut pencil = transparent_doc(64, 32);
    paint(&mut pencil, &cfg(true), &path).expect("pencil");
    let alpha = channel_data(&pencil.layers[0], -1).unwrap();
    assert!(alpha.contains(&255), "pencil painted nothing");
    assert!(
        alpha.iter().all(|&a| a == 0 || a == 255),
        "pencil produced a partial pixel"
    );

    let mut brush = transparent_doc(64, 32);
    paint(&mut brush, &cfg(false), &path).expect("brush");
    let alpha = channel_data(&brush.layers[0], -1).unwrap();
    assert!(
        alpha.iter().any(|&a| a > 0 && a < 255),
        "brush produced no soft edge"
    );
}

#[test]
fn clear_drives_opaque_alpha_to_zero() {
    let mut doc = layer_doc(64, 32, BASE);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity: 100,
        flow: 100,
        mode: PaintMode::Clear,
        ..StrokeConfig::default()
    };
    paint(&mut doc, &cfg, &[sample(5.0, 16.0), sample(55.0, 16.0)]).expect("clear");
    assert_eq!(chan(&doc, -1, 16 * 64 + 30), 0);
}

#[test]
fn clear_zeroes_rgb() {
    let mut doc = layer_doc(64, 32, BASE);
    let cfg = mode_cfg(PaintMode::Clear, 100);
    paint(&mut doc, &cfg, &[sample(5.0, 16.0), sample(55.0, 16.0)]).expect("clear");
    let i = 16 * 64 + 30;
    assert_eq!(chan(&doc, -1, i), 0);
    assert_eq!(
        (chan(&doc, 0, i), chan(&doc, 1, i), chan(&doc, 2, i)),
        (0, 0, 0)
    );
}

#[test]
fn clear_keeps_rgb_below_full() {
    let mut doc = layer_doc(64, 32, BASE);
    let cfg = mode_cfg(PaintMode::Clear, 50);
    paint(&mut doc, &cfg, &[sample(5.0, 16.0), sample(55.0, 16.0)]).expect("clear");
    let i = 16 * 64 + 30;
    let a = chan(&doc, -1, i);
    assert!(a > 0 && a < 255, "alpha {a} should stay partial");
    assert_eq!(
        (chan(&doc, 0, i), chan(&doc, 1, i), chan(&doc, 2, i)),
        (BASE.0, BASE.1, BASE.2)
    );
}

#[test]
fn noop_stroke_adds_no_history() {
    let mut doc = layer_doc(32, 16, BASE);
    let behind = mode_cfg(PaintMode::Behind, 100);
    assert!(paint(&mut doc, &behind, &[sample(8.0, 8.0)]).is_none());

    let zero = mode_cfg(PaintMode::Normal, 0);
    assert!(paint(&mut doc, &zero, &[sample(8.0, 8.0)]).is_none());

    let mut no_alpha = layer_doc(32, 16, BASE);
    no_alpha.layers[0].channels.retain(|c| c.id != -1);
    let clear = mode_cfg(PaintMode::Clear, 100);
    assert!(paint(&mut no_alpha, &clear, &[sample(8.0, 8.0)]).is_none());
}

#[test]
fn behind_leaves_opaque_rgb_unchanged() {
    let mut doc = layer_doc(64, 32, BASE);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity: 100,
        flow: 100,
        mode: PaintMode::Behind,
        ..StrokeConfig::default()
    };
    assert!(
        paint(&mut doc, &cfg, &[sample(5.0, 16.0), sample(55.0, 16.0)]).is_none(),
        "a Behind stroke over opaque pixels changes nothing"
    );
    let i = 16 * 64 + 30;
    assert_eq!(
        (chan(&doc, 0, i), chan(&doc, 1, i), chan(&doc, 2, i)),
        (BASE.0, BASE.1, BASE.2)
    );
}

#[test]
fn dissolve_is_deterministic() {
    let cfg = StrokeConfig {
        color: red(),
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity: 100,
        flow: 100,
        mode: PaintMode::Dissolve,
        ..StrokeConfig::default()
    };
    let path = [sample(5.0, 16.0), sample(55.0, 16.0)];

    let mut a = transparent_doc(64, 32);
    paint(&mut a, &cfg, &path).expect("a");
    let mut b = transparent_doc(64, 32);
    paint(&mut b, &cfg, &path).expect("b");

    for id in [0, 1, 2, -1] {
        assert_eq!(
            channel_data(&a.layers[0], id).unwrap(),
            channel_data(&b.layers[0], id).unwrap(),
            "channel {id} differed between runs"
        );
    }
}

#[test]
fn empty_samples_leave_document_unchanged() {
    let mut doc = layer_doc(16, 16, BASE);
    let before = doc.clone();
    let cfg = StrokeConfig {
        color: red(),
        diameter: 8,
        ..StrokeConfig::default()
    };
    assert!(paint(&mut doc, &cfg, &[]).is_none());
    assert_eq!(doc, before);
}

#[test]
fn group_only_document_has_no_raster_layer() {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    {
        let _ = doc.layers.push(Layer {
            name: "group".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 16,
                right: 16,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: None,
            channels: Vec::new(),
            children: pictura_core::LayerStack::default(),
            is_group: true,
            background: false,
            ..Default::default()
        });
    };
    assert!(matches!(
        Stroke::begin_at(&doc, "0", StrokeConfig::default()),
        Err(PaintError::NoRasterLayer)
    ));

    let empty = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    assert!(matches!(
        Stroke::begin_at(&empty, "0", StrokeConfig::default()),
        Err(PaintError::EmptyDocument)
    ));
}

#[test]
fn default_brush_paints_center_with_configured_color() {
    let mut doc = transparent_doc(64, 32);
    let cfg = StrokeConfig {
        color: red(),
        diameter: 20,
        hardness: 50,
        spacing: SpacingMode::Fixed(25),
        opacity: 100,
        flow: 100,
        ..StrokeConfig::default()
    };
    paint(&mut doc, &cfg, &[sample(10.0, 16.0), sample(50.0, 16.0)]).expect("paint");
    let i = 16 * 64 + 30;
    assert_eq!(
        (
            chan(&doc, 0, i),
            chan(&doc, 1, i),
            chan(&doc, 2, i),
            chan(&doc, -1, i)
        ),
        (255, 0, 0, 255)
    );
}

fn run(doc: &mut Document, cfg: StrokeConfig, kind: StrokeKind, xs: &[f32]) -> Stroke {
    let mut stroke = Stroke::begin_kind(doc, "0", cfg, kind).expect("begin");
    for &x in xs {
        stroke.sample(doc, sample(x, 16.0));
    }
    stroke
}

#[test]
fn a_replace_stroke_recolours_the_layer_live_and_commits_once() {
    let base = layer_doc(32, 32, (120, 120, 120, 255));
    let mut doc = base.clone();
    let cfg = StrokeConfig {
        color: Rgba {
            r: 220,
            g: 30,
            b: 30,
            a: 255,
        },
        diameter: 10,
        ..StrokeConfig::default()
    };
    let kind = StrokeKind::Replace(ReplaceOptions::default());
    let mut stroke = run(&mut doc, cfg, kind, &[8.0, 24.0]);
    let i = 16 * 32 + 16;
    let live = (chan(&doc, 0, i), chan(&doc, 1, i));
    assert!(
        live.0 > live.1 + 20,
        "the grey was not recoloured: {live:?}"
    );
    assert!(stroke.take_dirty().is_some());
    assert_eq!(stroke.mixer_reservoir(), None);
    assert!(stroke.finish().is_some(), "painted");
    assert_eq!(chan(&doc, 0, i), live.0);
    assert_eq!(chan(&doc, -1, i), 255);

    let mut again = base.clone();
    run(&mut again, cfg, kind, &[8.0, 24.0]).cancel(&mut again);
    assert!(again == base, "a cancelled stroke left paint behind");
}

#[test]
fn a_mixer_stroke_smears_and_reports_its_reservoir() {
    let mut doc = layer_doc(64, 32, (255, 255, 255, 255));
    for y in 0..32 {
        for x in 0..20 {
            let i = y * 64 + x;
            for id in 0..3 {
                channel_data_mut(&mut doc.layers[0], id).unwrap()[i] = 0;
            }
        }
    }
    let cfg = StrokeConfig {
        diameter: 10,
        spacing: SpacingMode::Fixed(25),
        ..StrokeConfig::default()
    };
    let kind = StrokeKind::Mixer {
        options: MixerOptions {
            wet: 1.0,
            load: 1.0,
            mix: 1.0,
            flow: 1.0,
        },
        reservoir: Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    };
    let stroke = run(&mut doc, cfg, kind, &[10.0, 40.0]);
    assert!(chan(&doc, 0, 16 * 64 + 26) < 250, "nothing was dragged");
    let carried = stroke.mixer_reservoir().expect("a mixer reservoir");
    assert!(carried.r < 255, "the reservoir picked nothing up");
}

/// The red pixels of a one-dab stroke at the centre of a 64² transparent layer.
fn inked(cfg: &StrokeConfig) -> Vec<usize> {
    let mut doc = transparent_doc(64, 64);
    paint(&mut doc, cfg, &[sample(32.0, 32.0)]);
    (0..64 * 64).filter(|&i| chan(&doc, -1, i) > 0).collect()
}

fn extent(pixels: &[usize]) -> usize {
    let xs = pixels.iter().map(|i| i % 64);
    xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0)
}

fn dab(diameter: u32) -> StrokeConfig {
    StrokeConfig {
        color: red(),
        diameter,
        ..StrokeConfig::default()
    }
}

#[test]
fn scatter_and_count_spread_a_cluster_off_the_path() {
    let plain = inked(&dab(6));
    let cluster = inked(&StrokeConfig {
        scatter: 300,
        count: 12,
        ..dab(6)
    });
    assert!(
        extent(&cluster) > extent(&plain) * 2,
        "scatter did not spread the dabs"
    );
    assert!(
        cluster.len() > plain.len() * 3,
        "a higher count laid no extra ink"
    );
}

#[test]
fn grain_breaks_up_the_dab_alpha() {
    let cfg = dab(16);
    let mut plain = transparent_doc(64, 64);
    let mut grained = transparent_doc(64, 64);
    paint(&mut plain, &cfg, &[sample(32.0, 32.0)]);
    paint(
        &mut grained,
        &StrokeConfig { grain: 75, ..cfg },
        &[sample(32.0, 32.0)],
    );

    let alpha = |doc: &Document, i: usize| chan(doc, -1, i);
    let solid = (0..64 * 64).filter(|&i| alpha(&plain, i) == 255).count();
    let partial = (0..64 * 64)
        .filter(|&i| {
            let a = alpha(&grained, i);
            a > 0 && a < 255
        })
        .count();
    let core_zero = (0..64 * 64)
        .filter(|&i| {
            let (dx, dy) = ((i % 64) as i32 - 32, (i / 64) as i32 - 32);
            dx * dx + dy * dy <= 25
        })
        .filter(|&i| alpha(&grained, i) == 0)
        .count();
    let sum = |doc: &Document| (0..64 * 64).map(|i| u32::from(alpha(doc, i))).sum::<u32>();
    assert!(solid > 0, "the plain dab is not solid");
    assert!(partial > 0, "grain left no partial alpha");
    assert_eq!(core_zero, 0, "grain punched holes in the dab core");
    assert!(
        sum(&grained) * 2 > sum(&plain),
        "grain scattered the dab: {} vs {}",
        sum(&grained),
        sum(&plain)
    );
    assert!(sum(&grained) < sum(&plain), "grain did not reduce coverage");
    let ratio = sum(&grained) as f32 / sum(&plain) as f32;
    assert!(
        (0.55..=0.67).contains(&ratio),
        "grain 75 should leave about 60 % coverage, got {ratio:.2}"
    );

    let mut repeat = transparent_doc(64, 64);
    paint(
        &mut repeat,
        &StrokeConfig { grain: 75, ..cfg },
        &[sample(32.0, 32.0)],
    );
    for id in [0, 1, 2, -1] {
        assert_eq!(
            channel_data(&grained.layers[0], id).unwrap(),
            channel_data(&repeat.layers[0], id).unwrap(),
            "channel {id} differed between runs"
        );
    }
}

#[test]
fn grain_strength_matches_the_pencil_presets() {
    let cfg = dab(16);
    let mut plain = transparent_doc(64, 64);
    paint(&mut plain, &cfg, &[sample(32.0, 32.0)]);
    let sum = |doc: &Document| {
        (0..64 * 64)
            .map(|i| u32::from(chan(doc, -1, i)))
            .sum::<u32>()
    };
    let ratio = |grain: u8| {
        let mut grained = transparent_doc(64, 64);
        paint(
            &mut grained,
            &StrokeConfig { grain, ..cfg },
            &[sample(32.0, 32.0)],
        );
        sum(&grained) as f32 / sum(&plain) as f32
    };
    let blue = ratio(55);
    let pencil = ratio(50);
    assert!(
        (0.67..=0.77).contains(&blue),
        "grain 55 should leave about 72 %, got {blue:.2}"
    );
    assert!(
        (0.70..=0.80).contains(&pencil),
        "grain 50 should leave about 75 %, got {pencil:.2}"
    );
}

#[test]
fn size_jitter_only_ever_shrinks() {
    let jittered = inked(&StrokeConfig {
        size_jitter: 90,
        ..dab(20)
    });
    assert!(extent(&jittered) <= extent(&inked(&dab(20))));
}

#[test]
fn dynamics_replay_exactly() {
    let cfg = StrokeConfig {
        scatter: 100,
        count: 6,
        size_jitter: 60,
        angle_jitter: 90,
        roundness: 40,
        roundness_jitter: 30,
        ..dab(10)
    };
    assert_eq!(inked(&cfg), inked(&cfg));
}

fn offset_layer_doc(w: u32, h: u32, rect: PsdRect, rgba: (u8, u8, u8, u8)) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let n = (rect.width() * rect.height()) as usize;
    let (r, g, b, a) = rgba;
    {
        let _ = doc.layers.push(Layer {
            name: "px".into(),
            rect,
            blend: BlendMode::Normal,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![r; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![g; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![b; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![a; n].into(),
                },
            ],
            ..Default::default()
        });
    };
    doc
}

#[test]
fn gpu_seed_and_patch_use_the_layer_document_rect() {
    let rect = PsdRect {
        top: 1,
        left: 2,
        bottom: 4,
        right: 6,
    };
    let mut doc = offset_layer_doc(8, 8, rect, (10, 20, 30, 200));
    let cfg = StrokeConfig {
        color: red(),
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin");

    let seed = stroke.base_layer_rgba_doc(&doc);
    assert_eq!(seed.len(), 8 * 8 * 4);
    assert_eq!(
        &seed[..4],
        &[0, 0, 0, 0],
        "outside the layer is transparent"
    );
    let inside = (8 + 2) * 4;
    assert_eq!(&seed[inside..inside + 4], &[10, 20, 30, 200]);

    // A 2x2 document patch at (1,0): only its document pixel (2,1) lands on
    // the layer (local 0,0), which reads patch index (3). Planes are
    // [R,G,B,A], row-major, index 3 = region row 1, col 1.
    let planes = vec![
        9u8, 8, 7, 6, // R
        19, 18, 17, 16, // G
        29, 28, 27, 26, // B
        39, 38, 37, 36, // A
    ];
    let changed = stroke.patch_working_layer(
        &mut doc,
        PsdRect {
            top: 0,
            left: 1,
            bottom: 2,
            right: 3,
        },
        4,
        &planes,
    );
    assert!(changed);
    let layer = &doc.layers[0];
    let at = |id| channel_data(layer, id).unwrap()[0];
    assert_eq!(
        (at(0), at(1), at(2), at(-1)),
        (6, 16, 26, 36),
        "the patch's document (2,1) writes the layer's local (0,0)"
    );
    let dirty = stroke.finish().expect("patched");
    assert_eq!(
        dirty,
        PsdRect {
            top: 1,
            left: 2,
            bottom: 2,
            right: 3,
        },
    );
}

/// A layer of distinct pixels, offset into the document, so a pixel read from
/// the wrong place or a tile never saved shows up in the result.
fn textured_doc() -> Document {
    let rect = PsdRect {
        top: 7,
        left: -5,
        bottom: 7 + 150,
        right: -5 + 170,
    };
    let mut doc = offset_layer_doc(160, 170, rect, BASE);
    for (id, c) in doc.layers[0].channels.iter_mut().enumerate() {
        for (i, v) in c.data.as_mut_slice().iter_mut().enumerate() {
            *v = ((i * (7 + id) + id * 31) % 251) as u8;
        }
    }
    doc
}

#[test]
fn painting_in_place_matches_a_stroke_that_saved_the_whole_layer() {
    let samples: Vec<StrokeSample> = (0..40)
        .map(|i| {
            let t = i as f32 * 0.37;
            sample(
                20.0 + t.cos() * 40.0 + i as f32 * 2.5,
                70.0 + t.sin() * 45.0,
            )
        })
        .collect();
    let base = StrokeConfig {
        color: red(),
        diameter: 30,
        hardness: 40,
        spacing: SpacingMode::Fixed(10),
        opacity: 60,
        flow: 45,
        ..StrokeConfig::default()
    };
    let configs = [
        base,
        StrokeConfig {
            mode: PaintMode::Behind,
            ..base
        },
        StrokeConfig {
            mode: PaintMode::Clear,
            ..base
        },
        StrokeConfig {
            mode: PaintMode::Dissolve,
            ..base
        },
        StrokeConfig {
            roundness: 30,
            angle_deg: 35,
            diameter: 64,
            ..base
        },
        StrokeConfig {
            square: true,
            ..base
        },
        StrokeConfig {
            scatter: 120,
            count: 3,
            size_jitter: 60,
            angle_jitter: 90,
            roundness_jitter: 50,
            ..base
        },
        StrokeConfig {
            aliased: true,
            opacity: 100,
            flow: 100,
            ..base
        },
    ];
    for (n, cfg) in configs.into_iter().enumerate() {
        let original = textured_doc();
        let mut lazy = original.clone();
        let mut stroke = Stroke::begin_at(&lazy, "0", cfg).expect("begin");
        let mut oracle = original.clone();
        let mut whole = Stroke::begin_at(&oracle, "0", cfg).expect("begin");
        whole.save_all(&oracle);
        for &s in &samples {
            stroke.sample(&mut lazy, s);
            whole.sample(&mut oracle, s);
        }
        assert_eq!(stroke.finish(), whole.finish(), "config {n}: dirty differs");
        assert!(
            lazy == oracle,
            "config {n}: in-place pixels differ from the oracle"
        );
        assert!(lazy != original, "config {n}: the stroke painted nothing");
    }
}

/// A large dab's rows run on several threads; every mode, tip, source and lock
/// must give exactly the bytes and the dirty box of the one-thread raster.
#[test]
fn a_banded_large_dab_matches_the_serial_raster() {
    let rect = PsdRect {
        top: 11,
        left: -9,
        bottom: 11 + 300,
        right: -9 + 420,
    };
    let mut textured = offset_layer_doc(400, 320, rect, BASE);
    for (id, c) in textured.layers[0].channels.iter_mut().enumerate() {
        for (i, v) in c.data.as_mut_slice().iter_mut().enumerate() {
            *v = ((i * (5 + id) + id * 17) % 253) as u8;
        }
    }
    let mut locked = textured.clone();
    locked.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let samples: Vec<StrokeSample> = (0..12)
        .map(|i| {
            sample(
                30.0 + i as f32 * 33.0,
                120.0 + (i as f32 * 0.9).sin() * 90.0,
            )
        })
        .collect();
    let base = StrokeConfig {
        color: red(),
        diameter: 260,
        hardness: 30,
        spacing: SpacingMode::Fixed(15),
        opacity: 70,
        flow: 55,
        ..StrokeConfig::default()
    };
    let configs = [
        base,
        StrokeConfig {
            mode: PaintMode::Behind,
            ..base
        },
        StrokeConfig {
            mode: PaintMode::Clear,
            ..base
        },
        StrokeConfig {
            mode: PaintMode::Dissolve,
            ..base
        },
        StrokeConfig {
            roundness: 35,
            angle_deg: 25,
            ..base
        },
        StrokeConfig {
            square: true,
            ..base
        },
        StrokeConfig {
            scatter: 60,
            count: 2,
            size_jitter: 40,
            angle_jitter: 60,
            roundness_jitter: 30,
            ..base
        },
        StrokeConfig {
            aliased: true,
            hardness: 100,
            ..base
        },
    ];
    let source = || {
        let (w, h) = (400, 320);
        let data = (0..(w * h) as usize)
            .map(|i| [(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255])
            .collect();
        StampSource::new(
            RgbaImage {
                width: w,
                height: h,
                data,
            },
            (17, -9),
        )
    };
    type Begin = Box<dyn Fn(&Document) -> Stroke>;
    let mut cases: Vec<(String, Document, Begin)> = Vec::new();
    for (n, cfg) in configs.into_iter().enumerate() {
        cases.push((
            format!("config {n}"),
            textured.clone(),
            Box::new(move |doc| Stroke::begin_at(doc, "0", cfg).expect("begin")),
        ));
    }
    cases.push((
        "transparency lock".into(),
        locked,
        Box::new(move |doc| Stroke::begin_at(doc, "0", base).expect("begin")),
    ));
    cases.push((
        "stamp source".into(),
        textured.clone(),
        Box::new(move |doc| Stroke::begin_source(doc, "0", base, source()).expect("begin")),
    ));
    for (name, original, begin) in cases {
        let mut banded_doc = original.clone();
        let mut banded = begin(&banded_doc);
        let mut serial_doc = original.clone();
        let mut serial = begin(&serial_doc).serialized();
        for &s in &samples {
            assert_eq!(
                banded.sample(&mut banded_doc, s),
                serial.sample(&mut serial_doc, s),
                "{name}: changed differs"
            );
            assert_eq!(
                banded.take_dirty(),
                serial.take_dirty(),
                "{name}: dab dirty differs"
            );
        }
        assert_eq!(banded.finish(), serial.finish(), "{name}: dirty differs");
        assert!(banded_doc == serial_doc, "{name}: banded pixels differ");
        assert!(banded_doc != original, "{name}: the stroke painted nothing");
    }
}

#[test]
fn cancelling_a_stroke_restores_every_pixel_it_touched() {
    let original = textured_doc();
    let mut doc = original.clone();
    let cfg = StrokeConfig {
        color: red(),
        diameter: 50,
        scatter: 80,
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("begin");
    for i in 0..30 {
        stroke.sample(&mut doc, sample(i as f32 * 6.0, 20.0 + i as f32 * 4.0));
    }
    assert!(doc != original, "the stroke painted nothing");
    stroke.cancel(&mut doc);
    assert!(doc == original, "cancel left paint behind");
}

#[test]
fn a_stroke_refuses_a_layer_restructured_under_it() {
    let mut doc = textured_doc();
    let mut stroke = Stroke::begin_at(&doc, "0", StrokeConfig::default()).expect("begin");
    doc.layers[0].channels.pop();
    let before = doc.clone();
    assert!(
        !stroke.sample(&mut doc, sample(40.0, 40.0)),
        "painted a changed layer"
    );
    stroke.cancel(&mut doc);
    assert!(doc == before, "a refused stroke touched the document");
}

#[test]
fn a_stroke_refuses_a_target_moved_under_it() {
    let mut doc = textured_doc();
    let mut stroke = Stroke::begin_at(&doc, "0", StrokeConfig::default()).expect("begin");
    let r = &mut doc.layers[0].rect;
    (r.left, r.right) = (r.left + 5, r.right + 5);
    let before = doc.clone();
    assert!(
        !stroke.sample(&mut doc, sample(40.0, 40.0)),
        "painted a moved layer"
    );
    stroke.cancel(&mut doc);
    assert!(doc == before, "a refused stroke touched the document");
}

#[test]
fn multiply_darkens_by_the_backdrop() {
    let grey = Rgba {
        r: 128,
        g: 128,
        b: 128,
        a: 255,
    };
    let cfg = StrokeConfig {
        color: grey,
        diameter: 12,
        hardness: 100,
        spacing: SpacingMode::Fixed(10),
        opacity: 100,
        flow: 100,
        mode: PaintMode::Multiply,
        ..StrokeConfig::default()
    };
    let path = [sample(5.0, 16.0), sample(55.0, 16.0)];
    let i = 16 * 64 + 30;
    let rgba = |doc: &Document| {
        (
            chan(doc, 0, i),
            chan(doc, 1, i),
            chan(doc, 2, i),
            chan(doc, -1, i),
        )
    };

    let mut opaque = layer_doc(64, 32, (200, 100, 50, 255));
    paint(&mut opaque, &cfg, &path).expect("multiply");
    assert_eq!(rgba(&opaque), (100, 50, 25, 255));

    let mut empty = transparent_doc(64, 32);
    paint(&mut empty, &cfg, &path).expect("multiply");
    assert_eq!(rgba(&empty), (128, 128, 128, 255));
}

#[test]
fn wet_edges_lighten_the_core_and_darken_the_rim() {
    // Issue #731's Paintbrush Oval 45 pixels Multiply, measured from the
    // owner's Photoshop screenshot: on white a lone dab paints 26 below white,
    // a stroke's interior 20, and its rim back up near the dab's tone.
    let cfg = StrokeConfig {
        color: Rgba {
            r: 0x47,
            g: 0x48,
            b: 0x4a,
            a: 255,
        },
        diameter: 45,
        hardness: 100,
        roundness: 50,
        angle_deg: -50,
        spacing: SpacingMode::Fixed(5),
        opacity: 35,
        flow: 50,
        mode: PaintMode::Multiply,
        wet_edges: true,
        ..StrokeConfig::default()
    };
    let white = (255, 255, 255, 255);
    let below_white = |doc: &Document, x: usize, y: usize| 255 - chan(doc, 0, y * 200 + x);

    let mut stroke = layer_doc(200, 200, white);
    paint(
        &mut stroke,
        &cfg,
        &[sample(40.0, 100.0), sample(160.0, 100.0)],
    )
    .expect("stroke");
    let core = below_white(&stroke, 100, 100);
    assert!((18..=22).contains(&core), "stroke interior {core}");
    let column: Vec<u8> = (0..200).map(|y| below_white(&stroke, 100, y)).collect();
    let top = column.iter().position(|&v| v > 0).expect("painted column");
    let rim = *column[top..top + 3].iter().max().unwrap();
    assert!(
        rim >= core + 2,
        "rim {rim} is not darker than the core {core}"
    );

    let mut dab = layer_doc(200, 200, white);
    paint(&mut dab, &cfg, &[sample(100.0, 100.0)]).expect("dab");
    let lone = below_white(&dab, 100, 100);
    assert!((24..=28).contains(&lone), "single dab {lone}");

    let mut dry = layer_doc(200, 200, white);
    let dry_cfg = StrokeConfig {
        wet_edges: false,
        ..cfg
    };
    paint(
        &mut dry,
        &dry_cfg,
        &[sample(40.0, 100.0), sample(160.0, 100.0)],
    )
    .expect("dry");
    assert!(
        below_white(&dry, 100, 100) > core * 2,
        "Wet Edges did not lighten the interior"
    );
}

#[test]
fn two_b_pencil_paints_the_reference_profile() {
    // Issue #731's 2B Pencil, measured from the owner's Photoshop pixel-grid
    // screenshot: a ~7 px soft-shouldered stroke whose cross-section integrates
    // to ~3.7 px of full coverage and peaks near 0.8 under the grain.
    let cfg = StrokeConfig {
        color: Rgba {
            r: 0x47,
            g: 0x48,
            b: 0x4a,
            a: 255,
        },
        diameter: 8,
        hardness: 25,
        spacing: SpacingMode::Fixed(25),
        size_jitter: 20,
        grain: 50,
        ..StrokeConfig::default()
    };
    let mut doc = layer_doc(128, 64, (255, 255, 255, 255));
    paint(&mut doc, &cfg, &[sample(10.0, 32.0), sample(118.0, 32.0)]).expect("pencil");
    let cov = |x: usize, y: usize| (255 - chan(&doc, 0, y * 128 + x)) as f32 / 184.0;
    let columns = 20..108;
    let n = columns.len() as f32;
    let (mut footprint, mut width, mut peak) = (0.0, 0.0, 0.0);
    for x in columns {
        let section: Vec<f32> = (0..64).map(|y| cov(x, y)).collect();
        footprint += section.iter().filter(|&&c| c > 0.05).count() as f32;
        width += section.iter().sum::<f32>();
        peak += section.iter().cloned().fold(0.0, f32::max);
    }
    let (footprint, width, peak) = (footprint / n, width / n, peak / n);
    assert!(
        (6.0..=8.0).contains(&footprint),
        "footprint {footprint:.2} px"
    );
    assert!(
        (3.2..=4.4).contains(&width),
        "effective width {width:.2} px"
    );
    assert!((0.70..=0.88).contains(&peak), "mean peak {peak:.2}");
}

#[test]
fn non_photo_blue_pencil_paints_the_reference_profile() {
    // Issue #731's Non-Photo Blue Pencil, measured from the owner's Photoshop
    // pixel-grid screenshot: a ~3.5 px line whose cross-section integrates to
    // ~1.8 px of full coverage and peaks near 0.74 under the grain.
    let cfg = StrokeConfig {
        color: Rgba {
            r: 0x73,
            g: 0xd6,
            b: 0xff,
            a: 255,
        },
        diameter: 4,
        hardness: 0,
        spacing: SpacingMode::Fixed(20),
        grain: 55,
        ..StrokeConfig::default()
    };
    let mut doc = layer_doc(128, 64, (255, 255, 255, 255));
    paint(&mut doc, &cfg, &[sample(10.0, 32.0), sample(118.0, 32.5)]).expect("pencil");
    // Red carries the most range for #73d6ff: 255 − 0x73 = 140 levels.
    let cov = |x: usize, y: usize| (255 - chan(&doc, 0, y * 128 + x)) as f32 / 140.0;
    let columns = 20..108;
    let n = columns.len() as f32;
    let (mut footprint, mut width, mut peak) = (0.0, 0.0, 0.0);
    for x in columns {
        let section: Vec<f32> = (0..64).map(|y| cov(x, y)).collect();
        footprint += section.iter().filter(|&&c| c > 0.05).count() as f32;
        width += section.iter().sum::<f32>();
        peak += section.iter().cloned().fold(0.0, f32::max);
    }
    let (footprint, width, peak) = (footprint / n, width / n, peak / n);
    assert!(
        (2.5..=4.5).contains(&footprint),
        "footprint {footprint:.2} px"
    );
    assert!(
        (1.5..=2.1).contains(&width),
        "effective width {width:.2} px"
    );
    assert!((0.64..=0.84).contains(&peak), "mean peak {peak:.2}");
}
