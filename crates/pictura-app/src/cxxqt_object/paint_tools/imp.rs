use super::*;

use super::super::helpers::{
    active_layer_visible, active_mask_target, active_pixel_layer, mask_stroke_config,
    paint_mode_from, rgba_from_argb,
};
use super::super::impl_paint::tool_recording;
use super::super::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use crate::history::BrushSource;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use ffi::PaintTip;
use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, PsdRect};
use pictura_paint::art_history::{ArtHistoryOptions, ArtStyle, STYLE_NAMES};
use pictura_paint::eraser::{
    antialias_mask, begin_erase, ensure_alpha, magic_erase, BackgroundEraseOptions, EraserMode,
};
use pictura_paint::focus::{Focus, FocusOptions, RetouchMode};
use pictura_paint::mixer::MixerOptions;
use pictura_paint::pattern::{self, PATTERN_NAMES};
use pictura_paint::replace::{Limits, ReplaceMode, ReplaceOptions, Sampling};
use pictura_paint::smudge::SmudgeOptions;
use pictura_paint::spacing::SpacingMode;
use pictura_paint::stamp::{
    layer_surface, sample_scope, surface_from_composite, tiled, CloneSampling, SourceTransform,
    StampSource,
};
use pictura_paint::tone::{SpongeMode, Tone, ToneOptions, ToneRange};
use pictura_paint::{paint_stroke, Rgba, Stroke, StrokeConfig, StrokeKind, StrokeSample};

pub(super) fn begin_color_replacement(
    view: Pin<&mut PictureView>,
    foreground: u32,
    background: u32,
    tip: &PaintTip,
    mode: i32,
    sampling: i32,
    limits: i32,
    tolerance: i32,
    antialias: bool,
) -> bool {
    let cfg = StrokeConfig {
        color: rgba_from_argb(foreground),
        background: rgba_from_argb(background),
        ..tip_config(tip)
    };
    let options = ReplaceOptions {
        mode: ReplaceMode::from_i32(mode).unwrap_or_default(),
        sampling: Sampling::from_i32(sampling).unwrap_or_default(),
        limits: Limits::from_i32(limits).unwrap_or_default(),
        // The bar shows a percentage; the match is per channel in 0–255.
        tolerance: (tolerance.clamp(0, 100) as f32 * 2.55).round() as u8,
        antialias,
    };
    begin(view, "Color Replacement Tool", cfg, |doc, path, _| {
        Stroke::begin_kind(doc, path, cfg, StrokeKind::Replace(options)).ok()
    })
}

pub(super) fn begin_mixer_brush(
    view: Pin<&mut PictureView>,
    reservoir: u32,
    tip: &PaintTip,
    wet: i32,
    load: i32,
    mix: i32,
    flow: i32,
    sample_all: bool,
    opacity: i32,
    mode: &QString,
) -> bool {
    let unit = |v: i32| v.clamp(0, 100) as f32 / 100.0;
    let options = MixerOptions {
        wet: unit(wet),
        load: unit(load),
        mix: unit(mix),
        flow: unit(flow),
    };
    let reservoir = rgba_from_argb(reservoir);
    let cfg = StrokeConfig {
        opacity: opacity.clamp(0, 100) as u8,
        mode: paint_mode_from(&mode.to_string()),
        ..tip_config(tip)
    };
    begin(view, "Mixer Brush Tool", cfg, |doc, path, view| {
        let sampled = sample_all.then(|| composite_surface(doc, view));
        Stroke::begin_mixer(doc, path, cfg, options, reservoir, sampled).ok()
    })
}

pub(super) fn mixer_reservoir(view: &PictureView) -> u32 {
    view.rust()
        .session
        .live
        .stroke()
        .and_then(Stroke::mixer_reservoir)
        .map_or(0, |c| {
            (c.a as u32) << 24 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32
        })
}

pub(super) fn tip_config(tip: &PaintTip) -> StrokeConfig {
    StrokeConfig {
        diameter: tip.diameter.max(0) as u32,
        hardness: tip.hardness.clamp(0, 100) as u8,
        roundness: tip.roundness.clamp(0, 100) as u8,
        angle_deg: tip.angle,
        spacing: SpacingMode::Fixed(tip.spacing.clamp(1, 1000) as u16),
        scatter: tip.scatter.clamp(0, 1000) as u16,
        count: tip.count.clamp(1, 16) as u8,
        size_jitter: tip.size_jitter.clamp(0, 100) as u8,
        angle_jitter: tip.angle_jitter.clamp(0, 180) as u16,
        roundness_jitter: tip.roundness_jitter.clamp(0, 100) as u8,
        grain: tip.grain.clamp(0, 100) as u8,
        wet_edges: tip.wet_edges,
        ..StrokeConfig::default()
    }
    .sanitized()
}

pub(super) fn begin_brush(
    view: Pin<&mut PictureView>,
    foreground: u32,
    background: u32,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
    aliased: bool,
    auto_erase: bool,
) -> bool {
    let cfg = StrokeConfig {
        color: rgba_from_argb(foreground),
        background: rgba_from_argb(background),
        aliased,
        auto_erase,
        ..stamp_config(tip, opacity, flow, mode)
    };
    let label = if aliased { "Pencil" } else { "Brush" };
    if let Some(path) = active_mask_target(view.rust()) {
        let mask_cfg = mask_stroke_config(cfg);
        let begun = view.begin_mask_paint(&path, mask_cfg, label);
        if begun {
            tool_recording::begin(
                label,
                &mask_cfg.sanitized(),
                tool_recording::ToolStrokeKind::Mask,
            );
        }
        return begun;
    }
    begin_kind(
        view,
        label,
        cfg,
        tool_recording::ToolStrokeKind::Paint,
        |doc, path, _| Stroke::begin_at(doc, path, cfg).ok(),
    )
}

pub(super) fn begin_clone_stamp(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
    offset_x: i32,
    offset_y: i32,
    anchor_x: f64,
    anchor_y: f64,
    scale_x: f64,
    scale_y: f64,
    angle: f64,
    sampling: i32,
    ignore_adjustments: bool,
) -> bool {
    let transform = SourceTransform {
        scale_x: scale_x as f32,
        scale_y: scale_y as f32,
        angle_deg: angle as f32,
    };
    // Sampling where it paints would copy each pixel onto itself; a zero scale
    // has no inverse.
    if ((offset_x, offset_y) == (0, 0) && transform.is_identity())
        || transform.scale_x == 0.0
        || transform.scale_y == 0.0
    {
        return false;
    }
    let sampling = CloneSampling::from_i32(sampling).unwrap_or_default();
    let cfg = stamp_config(tip, opacity, flow, mode);
    begin(view, "Clone Stamp", cfg, |doc, path, _| {
        let image = match sample_scope(doc, path, sampling, ignore_adjustments) {
            Some(scope) => surface_from_composite(&pictura_render::composite_rgba(&scope)),
            None => layer_surface(doc, path)?,
        };
        let anchor = (anchor_x as f32, anchor_y as f32);
        let source = StampSource::transformed(image, (offset_x, offset_y), anchor, transform);
        Stroke::begin_source(doc, path, cfg, source).ok()
    })
}

pub(super) fn begin_pattern_stamp(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
    pattern: i32,
    origin_x: i32,
    origin_y: i32,
) -> bool {
    let Some(tile) = usize::try_from(pattern).ok().and_then(pattern::tile) else {
        return false;
    };
    let cfg = stamp_config(tip, opacity, flow, mode);
    begin(view, "Pattern Stamp", cfg, |doc, path, _| {
        let image = tiled(&tile, doc.width, doc.height, (origin_x, origin_y))?;
        Stroke::begin_source(doc, path, cfg, StampSource::new(image, (0, 0))).ok()
    })
}

pub(super) fn begin_history_brush(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
) -> bool {
    let cfg = stamp_config(tip, opacity, flow, mode);
    begin(view, "History Brush", cfg, |doc, path, view| {
        // ponytail: the source layer is matched by panel path (layers carry no
        // stable id), so a reordered stack paints from whatever now sits there.
        let past = view.session.history.brush_source_doc()?;
        let source = StampSource::new(layer_surface(&past.doc, path)?, (0, 0));
        Stroke::begin_source(doc, path, cfg, source).ok()
    })
}

pub(super) fn begin_art_history_brush(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    style: i32,
    area: i32,
    tolerance: i32,
) -> bool {
    let cfg = StrokeConfig {
        opacity: opacity.clamp(0, 100) as u8,
        ..tip_config(tip)
    };
    begin(view, "Art History Brush", cfg, |doc, path, view| {
        let options = ArtHistoryOptions {
            style: ArtStyle::from_i32(style).unwrap_or_default(),
            area: area.clamp(0, 500) as u32,
            tolerance: tolerance.clamp(0, 100) as u8,
            // A new seed per history state: strokes differ, a stroke replays.
            seed: view.session.history.index() as u64 ^ 0xA27_4157,
        };
        let past = view.session.history.brush_source_doc()?;
        Stroke::begin_art_history(doc, path, cfg, layer_surface(&past.doc, path)?, options).ok()
    })
}

pub(super) fn art_history_style_count() -> i32 {
    STYLE_NAMES.len() as i32
}

pub(super) fn art_history_style_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| STYLE_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

pub(super) fn begin_eraser(
    view: Pin<&mut PictureView>,
    background: u32,
    tip: &PaintTip,
    mode: i32,
    opacity: i32,
    flow: i32,
    to_history: bool,
    zoom: f64,
) -> bool {
    let cfg = StrokeConfig {
        background: rgba_from_argb(background),
        opacity: opacity.clamp(0, 100) as u8,
        flow: flow.clamp(0, 100) as u8,
        ..tip_config(tip)
    };
    let mode = EraserMode::from_i32(mode).unwrap_or_default();
    // Erasing a mask paints the background colour's coverage, as on a
    // Background layer. ponytail: Block mode erases with the brush tip, and
    // Erase To History has no mask source, so it is refused.
    if let Some(path) = active_mask_target(view.rust()) {
        if to_history {
            return false;
        }
        let mask_cfg = StrokeConfig {
            color: cfg.background,
            aliased: mode == EraserMode::Pencil,
            ..cfg
        };
        let mask_cfg = mask_stroke_config(mask_cfg);
        let begun = view.begin_mask_paint(&path, mask_cfg, "Eraser");
        if begun {
            tool_recording::begin(
                "Eraser",
                &mask_cfg.sanitized(),
                tool_recording::ToolStrokeKind::Mask,
            );
        }
        return begun;
    }
    begin(view, "Eraser", cfg, |doc, path, view| {
        // ponytail: the source layer is matched by panel path, as for the
        // History Brush.
        let history = if to_history {
            let past = view.session.history.brush_source_doc()?;
            Some(StampSource::new(layer_surface(&past.doc, path)?, (0, 0)))
        } else {
            None
        };
        begin_erase(doc, path, cfg, mode, history, zoom).ok()
    })
}

/// The Block eraser's side in document pixels at `zoom`, so the canvas draws
/// the cursor at the size the block erases.
pub(super) fn eraser_block_side_doc_px(zoom: f64) -> u32 {
    pictura_paint::eraser::block_side_doc_px(zoom)
}

pub(super) fn begin_background_eraser(
    mut view: Pin<&mut PictureView>,
    foreground: u32,
    background: u32,
    tip: &PaintTip,
    sampling: i32,
    limits: i32,
    tolerance: i32,
    protect_foreground: bool,
) -> bool {
    let cfg = StrokeConfig {
        color: rgba_from_argb(foreground),
        background: rgba_from_argb(background),
        ..tip_config(tip)
    };
    let options = BackgroundEraseOptions {
        sampling: Sampling::from_i32(sampling).unwrap_or_default(),
        limits: Limits::from_i32(limits).unwrap_or_default(),
        tolerance: (tolerance.clamp(0, 100) as f32 * 2.55).round() as u8,
        protect_foreground,
    };
    // The stroke paints the live document, so a Background is layered there
    // first; `stroke_base` keeps the original for a cancel or a refusal.
    let layered = {
        let rust = view.rust();
        rust.session
            .doc
            .as_ref()
            .zip(rust.layer_target.single())
            .and_then(|(doc, path)| unlocked_background(doc, path))
    };
    let mut base = None;
    if let Some(layered) = layered {
        let mut rust = view.as_mut().rust_mut();
        base = rust.session.doc.replace(layered);
    }
    let begun = begin(view.as_mut(), "Background Eraser", cfg, |doc, path, _| {
        Stroke::begin_kind(doc, path, cfg, StrokeKind::BackgroundErase(options)).ok()
    });
    if !begun {
        let mut rust = view.as_mut().rust_mut();
        if let Some(base) = base {
            rust.session.doc = Some(base);
        }
        return false;
    }
    // The stroke is live, so the pre-stroke document belongs to its session: a
    // cancel or a refused commit puts the Background back.
    let mut rust = view.as_mut().rust_mut();
    match base {
        Some(base) => rust.session.live.set_stroke_base(base),
        None => rust.session.live.clear_stroke_base(),
    }
    true
}

pub(super) fn begin_focus(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    sharpen: bool,
    strength: i32,
    mode: i32,
    sample_all: bool,
    protect_detail: bool,
) -> bool {
    let options = FocusOptions {
        focus: if sharpen { Focus::Sharpen } else { Focus::Blur },
        strength: strength.clamp(0, 100) as f32 / 100.0,
        mode: RetouchMode::from_i32(mode).unwrap_or_default(),
        protect_detail,
    };
    let cfg = tip_config(tip);
    let label = if sharpen { "Sharpen" } else { "Blur" };
    begin(view, label, cfg, |doc, path, view| {
        let sampled = sample_all.then(|| composite_surface(doc, view));
        Stroke::begin_focus(doc, path, cfg, options, sampled).ok()
    })
}

pub(super) fn begin_smudge(
    view: Pin<&mut PictureView>,
    foreground: u32,
    tip: &PaintTip,
    strength: i32,
    mode: i32,
    sample_all: bool,
    finger_painting: bool,
) -> bool {
    let options = SmudgeOptions {
        strength: strength.clamp(0, 100) as f32 / 100.0,
        mode: RetouchMode::from_i32(mode).unwrap_or_default(),
        finger_painting,
    };
    let cfg = StrokeConfig {
        color: rgba_from_argb(foreground),
        ..tip_config(tip)
    };
    begin(view, "Smudge", cfg, |doc, path, view| {
        let sampled = sample_all.then(|| composite_surface(doc, view));
        Stroke::begin_smudge(doc, path, cfg, options, sampled).ok()
    })
}

pub(super) fn begin_tone(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    tool: i32,
    amount: i32,
    range: i32,
    protect_tones: bool,
    sponge: i32,
    vibrance: bool,
) -> bool {
    let (tone, label) = match tool {
        1 => (Tone::Burn, "Burn"),
        2 => (Tone::Sponge, "Sponge"),
        _ => (Tone::Dodge, "Dodge"),
    };
    let options = ToneOptions {
        tone,
        range: ToneRange::from_i32(range).unwrap_or_default(),
        amount: amount.clamp(0, 100) as f32 / 100.0,
        protect_tones,
        sponge: SpongeMode::from_i32(sponge).unwrap_or_default(),
        vibrance,
    };
    let cfg = tip_config(tip);
    begin(view, label, cfg, |doc, path, _| {
        Stroke::begin_tone(doc, path, cfg, options).ok()
    })
}

/// The document's composite as a document-space surface: what Sample All
/// Layers reads.
pub(super) fn composite_surface(
    doc: &Document,
    view: &PictureViewRust,
) -> pictura_paint::healing::RgbaImage {
    surface_from_composite(&super::super::picture::picture(doc, view))
}

/// A copy of `doc` with the Background at `path` turned into an ordinary
/// layer with an opaque alpha channel, so it can be erased to transparency;
/// `None` when `path` is not the Background.
pub(super) fn unlocked_background(doc: &Document, path: &str) -> Option<Document> {
    if !pictura_render::resolve_path(doc, path)?.background {
        return None;
    }
    let mut layered = doc.clone();
    pictura_render::layer_from_background(&mut layered, path);
    ensure_alpha(pictura_render::resolve_path_mut(&mut layered, path)?);
    Some(layered)
}

pub(super) fn magic_erase_at(
    mut view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    tolerance: i32,
    antialias: bool,
    contiguous: bool,
    sample_all: bool,
    opacity: i32,
    background: u32,
) -> bool {
    let erased = {
        let rust = view.rust();
        let (Some(doc), Some(path)) = (rust.session.doc.as_ref(), rust.layer_target.single())
        else {
            return false;
        };
        if rust.session.live.stroke().is_some()
            || active_mask_target(rust).is_some()
            || x < 0
            || y < 0
            || active_pixel_layer(doc, Some(path)).is_none()
            || !active_layer_visible(doc, Some(path))
            || pictura_render::resolve_path(doc, path).is_none_or(pictura_core::layer_pixel_locked)
        {
            return false;
        }
        let sampled = if sample_all {
            super::super::picture::picture(doc, rust)
        } else {
            let Some(surface) = layer_surface(doc, path) else {
                return false;
            };
            std::borrow::Cow::Owned(planar(&surface))
        };
        let tolerance = tolerance.clamp(0, 255) as u8;
        let Ok(flood) =
            pictura_select::magic_wand(&sampled, x as u32, y as u32, tolerance, contiguous)
        else {
            return false;
        };
        let mask = if antialias {
            antialias_mask(&flood.data, flood.width as usize)
        } else {
            flood.data.to_vec()
        };
        let mut edited = unlocked_background(doc, path).unwrap_or_else(|| doc.clone());
        let opacity = opacity.clamp(0, 100) as f32 / 100.0;
        magic_erase(
            &mut edited,
            path,
            &mask,
            opacity,
            rgba_from_argb(background),
        )
        .map(|_| edited)
    };
    let Some(doc) = erased else {
        return false;
    };
    view.as_mut().rust_mut().session.doc = Some(doc);
    view.as_mut().edit("Magic Eraser", |_| Edit::Full);
    true
}

/// A document-space surface as the planar RGBA buffer the Magic Wand reads.
pub(super) fn planar(surface: &pictura_paint::healing::RgbaImage) -> pictura_core::PixelBuffer {
    let n = surface.data.len();
    let mut data = vec![0u8; n * 4];
    for (i, px) in surface.data.iter().enumerate() {
        for (c, v) in px.iter().enumerate() {
            data[c * n + i] = *v;
        }
    }
    pictura_core::PixelBuffer {
        width: surface.width as u32,
        height: surface.height as u32,
        channels: 4,
        data: data.into(),
    }
}

pub(super) fn stamp_pattern_count() -> i32 {
    PATTERN_NAMES.len() as i32
}

pub(super) fn stamp_pattern_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| PATTERN_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

pub(super) fn stamp_pattern_tile(index: i32) -> Vec<u8> {
    usize::try_from(index)
        .ok()
        .and_then(pattern::tile)
        .map_or_else(Vec::new, |tile| tile.data.concat())
}

pub(super) fn stamp_pattern_side() -> i32 {
    pattern::TILE
}

pub(super) fn history_brush_source_state(view: &PictureView) -> i32 {
    match view.rust().session.history.brush_source() {
        BrushSource::Oldest => 0,
        BrushSource::State(i) => i as i32,
        _ => -1,
    }
}

pub(super) fn history_brush_source_snapshot(view: &PictureView) -> i32 {
    match view.rust().session.history.brush_source() {
        BrushSource::Snapshot(i) => i as i32,
        _ => -1,
    }
}

pub(super) fn set_history_brush_source(
    mut view: Pin<&mut PictureView>,
    snapshot: bool,
    index: i32,
) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let source = if snapshot {
        BrushSource::Snapshot(index)
    } else {
        BrushSource::State(index)
    };
    view.as_mut()
        .rust_mut()
        .session
        .history
        .set_brush_source(source)
}

pub(super) fn replay_tool_stroke(mut view: Pin<&mut PictureView>, payload: &QString) -> bool {
    let mut rust = view.as_mut().rust_mut();
    let rust = &mut *rust;
    tool_recording::replay_tool_stroke_json(rust, &payload.to_string()).is_ok()
}

pub(super) fn stamp_config(
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
) -> StrokeConfig {
    StrokeConfig {
        opacity: opacity.clamp(0, 100) as u8,
        flow: flow.clamp(0, 100) as u8,
        mode: paint_mode_from(&mode.to_string()),
        ..tip_config(tip)
    }
}

pub(super) fn brush_tip_preview(tip: &PaintTip, width: i32, height: i32) -> Vec<u8> {
    let (w, h) = (width.clamp(1, 2048), height.clamp(1, 2048));
    let cfg = tip_config(tip);
    // One period of a sine across the middle, inset by the tip's radius.
    let inset = (cfg.diameter as f32 / 2.0).min(w as f32 / 4.0);
    let samples: Vec<StrokeSample> = (0..=64)
        .map(|i| {
            let t = i as f32 / 64.0;
            StrokeSample {
                x: inset + t * (w as f32 - 2.0 * inset),
                y: h as f32 / 2.0 - (t * std::f32::consts::TAU).sin() * h as f32 / 5.0,
                pressure: 1.0,
            }
        })
        .collect();
    white_preview(cfg, w, h, &samples)
}

pub(super) fn brush_dab_preview(tip: &PaintTip, edge: i32) -> Vec<u8> {
    let edge = edge.clamp(1, 512);
    let mut cfg = tip_config(tip);
    // The whole cluster (the tip plus its scatter either side) fits the square.
    let span = cfg.diameter as f32 * (1.0 + 2.0 * cfg.scatter as f32 / 100.0);
    let fit = (edge as f32 - 2.0) / span;
    if fit < 1.0 {
        cfg.diameter = ((cfg.diameter as f32 * fit).round() as u32).max(1);
    }
    let centre = StrokeSample {
        x: edge as f32 / 2.0,
        y: edge as f32 / 2.0,
        pressure: 1.0,
    };
    white_preview(cfg, edge, edge, &[centre])
}

/// `samples` painted white with `cfg` on a `w` × `h` transparent layer,
/// packed RGBA8888.
pub(super) fn white_preview(
    cfg: StrokeConfig,
    w: i32,
    h: i32,
    samples: &[StrokeSample],
) -> Vec<u8> {
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    let n = (w * h) as usize;
    {
        let _ = doc.layers.push(Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h,
                right: w,
            },
            channels: [0, 1, 2, -1]
                .map(|id| Channel {
                    id,
                    data: vec![0; n].into(),
                })
                .into(),
            ..Default::default()
        });
    };
    let cfg = StrokeConfig {
        color: Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        ..cfg
    };
    paint_stroke(&mut doc, "0", &cfg, samples);
    let plane = |id: i16| {
        doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == id)
            .map_or(&[][..], |c| c.data.as_slice())
    };
    let (r, g, b, a) = (plane(0), plane(1), plane(2), plane(-1));
    (0..n).flat_map(|i| [r[i], g[i], b[i], a[i]]).collect()
}

/// Begin a stroke on the visible lone active pixel layer; `start` builds it
/// from the document, the active path, and the view. False mid-stroke or when
/// `start` refuses.
pub(super) fn begin(
    view: Pin<&mut PictureView>,
    label: &str,
    cfg: StrokeConfig,
    start: impl FnOnce(&Document, &str, &PictureViewRust) -> Option<Stroke>,
) -> bool {
    begin_kind(
        view,
        label,
        cfg,
        tool_recording::ToolStrokeKind::Native,
        start,
    )
}

/// As [`begin`], tagging the capture with the replay path that can drive it.
pub(super) fn begin_kind(
    mut view: Pin<&mut PictureView>,
    label: &str,
    cfg: StrokeConfig,
    kind: tool_recording::ToolStrokeKind,
    start: impl FnOnce(&Document, &str, &PictureViewRust) -> Option<Stroke>,
) -> bool {
    let stroke = {
        let rust = view.rust();
        let (Some(doc), Some(path)) = (rust.session.doc.as_ref(), rust.layer_target.single())
        else {
            return false;
        };
        // ponytail: only the Brush, Pencil, and Eraser paint a mask; the other
        // painting tools refuse rather than write the layer's pixels.
        if !rust.slot_allows(super::super::document_session::SessionKind::Paint)
            || rust.session.live.stroke().is_some()
            || active_mask_target(rust).is_some()
            || active_pixel_layer(doc, Some(path)).is_none()
            || !active_layer_visible(doc, Some(path))
        {
            return false;
        }
        // Opened only once the stroke can start, so a refused call never leaves
        // a session for the next tool to report under its title.
        super::super::helpers::paint_timing::start(label);
        start(doc, path, rust)
    };
    let Some(stroke) = stroke else {
        super::super::helpers::paint_timing::report();
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    let paint_pad = rust
        .session
        .doc
        .as_ref()
        .and_then(|doc| {
            rust.layer_target
                .single()
                .map(|path| super::super::helpers::paint_pad_for(doc, path))
        })
        .unwrap_or(0);
    let session = super::super::document_session::StrokeSession {
        stroke: Some(stroke),
        stroke_label: label.to_string(),
        paint_pad,
        ..Default::default()
    };
    rust.session.live.set_stroke_session(session);
    // A new stroke opens its own present frame and tile set, exactly as
    // `begin_paint` does, so the color-replacement and mixer commits decompose
    // into dirty tiles rather than the whole bounding box.
    rust.clear_pending_present();
    let dims = rust.session.doc.as_ref().map(|doc| (doc.width, doc.height));
    if let Some((width, height)) = dims {
        rust.session.live.reset_stroke_tiles(width, height);
    }
    tool_recording::begin(label, &cfg.sanitized(), kind);
    true
}

#[cfg(test)]
mod tests {
    use super::composite_surface;
    use crate::cxxqt_object::document_session::DocumentSession;
    use crate::cxxqt_object::picture::composites_so_far;
    use crate::cxxqt_object::state::PictureViewRust;
    use pictura_core::Document;

    #[test]
    fn a_read_only_consumer_composites_nothing() {
        let rgba = [40u8, 80, 120, 255].repeat(16 * 16);
        let doc = Document::from_rgba("0", 16, 16, &rgba);
        let view = PictureViewRust {
            session: DocumentSession::with_document(doc),
            ..Default::default()
        };
        let doc = view.session.doc.as_ref().expect("document");
        let before = composites_so_far();
        let surface = composite_surface(doc, &view);
        assert_eq!(
            composites_so_far(),
            before,
            "a fresh document is served from its composite"
        );
        assert_eq!((surface.width, surface.height), (16, 16));
        assert_eq!(surface.data[0], [40, 80, 120, 255]);
    }
}
