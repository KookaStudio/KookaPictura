//! The paint tool bridges beyond the Brush: Color Replacement, Mixer Brush,
//! the stamps (Clone Stamp, Pattern Stamp, History Brush), the Art History
//! Brush, the erasers, and the retouch tools (Blur, Sharpen, Smudge) and
//! toning tools (Dodge, Burn, Sponge). Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). Each only *begins* a stroke — of its [`StrokeKind`], or
//! one whose colour comes from a [`StampSource`]; the live stroke then runs
//! through the Brush's `paint_dab` / `end_paint` / `cancel_paint`, so the
//! preview and the one history state per stroke (`"Color Replacement Tool"`,
//! `"Mixer Brush Tool"`, `"Clone Stamp"`, `"Pattern Stamp"`, `"History
//! Brush"`, `"Art History Brush"`, `"Eraser"`, `"Background Eraser"`,
//! `"Blur"`, `"Sharpen"`, `"Smudge"`, `"Dodge"`, `"Burn"`, `"Sponge"`) are shared. The Magic Eraser is one click and records `"Magic Eraser"` itself,
//! as do the fill tools in [`fills`].
//!
//! [`PictureView`]: super::qobject::PictureView

pub(super) mod fills;

use super::helpers::{
    active_layer_visible, active_mask_target, active_pixel_layer, mask_stroke_config,
    paint_mode_from, rgba_from_argb,
};
use super::qobject::PictureView;
use super::PictureViewRust;
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

#[cxx_qt::bridge]
pub mod ffi {
    /// The brush tip every paint tool here shares: the Brush panel's Brush Tip
    /// Shape. `diameter` 1–5000 px, `hardness` and `roundness` 0–100 %,
    /// `angle` −180–180°, `spacing` a percentage of the diameter. The
    /// dynamics: `count` dabs per step (1–16), `scatter` % of the diameter,
    /// `size_jitter` / `roundness_jitter` %, `angle_jitter` degrees, and
    /// `grain` % (the pencil's paper texture), and `wet_edges` (Wet Edges).
    #[namespace = "pictura"]
    struct PaintTip {
        diameter: i32,
        hardness: i32,
        roundness: i32,
        angle: i32,
        spacing: i32,
        scatter: i32,
        count: i32,
        size_jitter: i32,
        angle_jitter: i32,
        roundness_jitter: i32,
        grain: i32,
        wet_edges: bool,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Begin a Color Replacement stroke on the active pixel layer, painting
        /// `foreground` (`0xAARRGGBB`); `background` is the Background Swatch
        /// reference. `mode` 0 Hue … 3 Luminosity, `sampling` 0 Continuous /
        /// 1 Once / 2 Background Swatch, `limits` 0 Discontiguous /
        /// 1 Contiguous / 2 Find Edges, `tolerance` 0–100 %. False without a
        /// visible lone pixel layer, when its pixels are locked, or mid-stroke.
        fn begin_color_replacement(
            view: Pin<&mut PictureView>,
            foreground: u32,
            background: u32,
            tip: &PaintTip,
            mode: i32,
            sampling: i32,
            limits: i32,
            tolerance: i32,
            antialias: bool,
        ) -> bool;

        /// Begin a Mixer Brush stroke with `reservoir` (`0xAARRGGBB`; alpha 0
        /// is a clean brush) on the brush. `wet`, `load`, `mix`, and `flow` are
        /// 0–100 %. False as for `begin_color_replacement`.
        fn begin_mixer_brush(
            view: Pin<&mut PictureView>,
            reservoir: u32,
            tip: &PaintTip,
            wet: i32,
            load: i32,
            mix: i32,
            flow: i32,
        ) -> bool;

        /// The paint on the brush of the active Mixer Brush stroke
        /// (`0xAARRGGBB`), or 0 without one. Read before `end_paint` to carry
        /// the brush into the next stroke.
        fn mixer_reservoir(view: &PictureView) -> u32;

        /// Begin a Clone Stamp stroke painting what lies at `(offset_x,
        /// offset_y)` from each pixel, snapshotted now, through the Clone
        /// Source transform about document point `(anchor_x, anchor_y)`: W / H
        /// as `scale_x` / `scale_y` (negative flips) and `angle` degrees
        /// counter-clockwise. `sampling` 0 Current Layer / 1 Current And Below /
        /// 2 All Layers; `ignore_adjustments` applies to All Layers. `mode` is a
        /// Brush mode (`"normal"`, …). False without a visible lone pixel layer,
        /// when its pixels are locked, for an untransformed zero offset or a
        /// zero scale, or mid-stroke.
        fn begin_clone_stamp(
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
        ) -> bool;

        /// Begin a Pattern Stamp stroke painting built-in pattern `pattern`
        /// tiled from document point `(origin_x, origin_y)`. False as for
        /// `begin_clone_stamp`, or for an unknown pattern.
        fn begin_pattern_stamp(
            view: Pin<&mut PictureView>,
            tip: &PaintTip,
            opacity: i32,
            flow: i32,
            mode: &QString,
            pattern: i32,
            origin_x: i32,
            origin_y: i32,
        ) -> bool;

        /// Begin a History Brush stroke painting the active layer as it was in
        /// the brush source state. False as for `begin_clone_stamp`, or when
        /// the source state has no pixel layer at the active path.
        fn begin_history_brush(
            view: Pin<&mut PictureView>,
            tip: &PaintTip,
            opacity: i32,
            flow: i32,
            mode: &QString,
        ) -> bool;

        /// Begin an Art History Brush stroke: stylized strokes coloured from
        /// the active layer as it was in the brush source state. `style` 0
        /// Tight Short … 9 Loose Curl Long, `area` in pixels, `tolerance`
        /// 0–100 %. False as for `begin_history_brush`, or on a 16/32-bit
        /// document.
        fn begin_art_history_brush(
            view: Pin<&mut PictureView>,
            tip: &PaintTip,
            opacity: i32,
            style: i32,
            area: i32,
            tolerance: i32,
        ) -> bool;

        /// The number of Art History Brush styles.
        fn art_history_style_count() -> i32;

        /// Art History Brush style `index`'s name, or empty out of range.
        fn art_history_style_name(index: i32) -> QString;

        /// Begin an Eraser stroke. `mode` 0 Brush / 1 Pencil / 2 Block;
        /// `background` (`0xAARRGGBB`) is what the Background or a
        /// transparency-locked layer is erased to. With `to_history` it paints
        /// the brush source state back (Erase To History). False as for
        /// `begin_clone_stamp`, or with `to_history` when the source state has
        /// no pixel layer at the active path.
        fn begin_eraser(
            view: Pin<&mut PictureView>,
            background: u32,
            tip: &PaintTip,
            mode: i32,
            opacity: i32,
            flow: i32,
            to_history: bool,
        ) -> bool;

        /// Begin a Brush (or, `aliased`, Pencil) stroke painting `foreground`
        /// (`0xAARRGGBB`) with `tip` and its dynamics. `mode` is a Brush mode
        /// (`"normal"`, …); `auto_erase` paints `background` over the
        /// foreground (Pencil). False as for `begin_clone_stamp`.
        fn begin_brush(
            view: Pin<&mut PictureView>,
            foreground: u32,
            background: u32,
            tip: &PaintTip,
            opacity: i32,
            flow: i32,
            mode: &QString,
            aliased: bool,
            auto_erase: bool,
        ) -> bool;

        /// Begin a Background Eraser stroke. `sampling` 0 Continuous / 1 Once
        /// / 2 Background Swatch (`background`, `0xAARRGGBB`), `limits` 0
        /// Discontiguous / 1 Contiguous / 2 Find Edges, `tolerance` 0–100 %;
        /// with `protect_foreground` pixels matching `foreground` are kept. The
        /// Background is turned into a layer by the stroke; Lock Transparency is
        /// overridden. False as for `begin_clone_stamp`.
        fn begin_background_eraser(
            view: Pin<&mut PictureView>,
            foreground: u32,
            background: u32,
            tip: &PaintTip,
            sampling: i32,
            limits: i32,
            tolerance: i32,
            protect_foreground: bool,
        ) -> bool;

        /// Magic Eraser: erase the region the Magic Wand floods from document
        /// point `(x, y)` — `tolerance` 0–255 per channel, `contiguous`, the
        /// active layer or with `sample_all` the composite — at `opacity`
        /// 0–100 %, with a softened edge when `antialias`. A Background is
        /// turned into a layer first; a transparency-locked layer is filled
        /// with `background` instead. One "Magic Eraser" state; false when
        /// nothing changed or the active layer cannot be edited.
        fn magic_erase_at(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            tolerance: i32,
            antialias: bool,
            contiguous: bool,
            sample_all: bool,
            opacity: i32,
            background: u32,
        ) -> bool;

        /// Begin a Blur (or, `sharpen`, Sharpen) stroke working the active
        /// pixel layer by `strength` 0–100 % per dab. `mode` 0 Normal /
        /// 1 Darken / 2 Lighten / 3 Hue / 4 Saturation / 5 Color /
        /// 6 Luminosity; with `sample_all` the neighbourhood is read from the
        /// composite; `protect_detail` holds a sharpened pixel inside its
        /// neighbourhood's range. False as for `begin_clone_stamp`, or on a
        /// 16/32-bit document.
        fn begin_focus(
            view: Pin<&mut PictureView>,
            tip: &PaintTip,
            sharpen: bool,
            strength: i32,
            mode: i32,
            sample_all: bool,
            protect_detail: bool,
        ) -> bool;

        /// Begin a Smudge stroke laying down `strength` 0–100 % of the patch
        /// it carries per dab, in `mode` as for `begin_focus`; with
        /// `sample_all` it picks up from the composite, and with
        /// `finger_painting` it starts loaded with `foreground`
        /// (`0xAARRGGBB`). False as for `begin_focus`.
        fn begin_smudge(
            view: Pin<&mut PictureView>,
            foreground: u32,
            tip: &PaintTip,
            strength: i32,
            mode: i32,
            sample_all: bool,
            finger_painting: bool,
        ) -> bool;

        /// Begin a toning stroke: `tool` 0 Dodge / 1 Burn / 2 Sponge.
        /// `amount` 0–100 % is Dodge and Burn's Exposure and the Sponge's
        /// Flow. Dodge and Burn: `range` 0 Shadows / 1 Midtones /
        /// 2 Highlights, `protect_tones` keeps each pixel's colour. Sponge:
        /// `sponge` 0 Desaturate / 1 Saturate, `vibrance` eases off where
        /// there is little to do. False as for `begin_focus`.
        fn begin_tone(
            view: Pin<&mut PictureView>,
            tip: &PaintTip,
            tool: i32,
            amount: i32,
            range: i32,
            protect_tones: bool,
            sponge: i32,
            vibrance: bool,
        ) -> bool;

        /// A white stroke with `tip` along an S-curve on transparency, packed
        /// RGBA8888 `width` × `height`: the Brush panel's stroke preview.
        fn brush_tip_preview(tip: &PaintTip, width: i32, height: i32) -> Vec<u8>;

        /// One white step of `tip` (its dynamics included) centred on an
        /// `edge`² transparent square, scaled down to fit, packed RGBA8888:
        /// the brush preset picker's thumbnails.
        fn brush_dab_preview(tip: &PaintTip, edge: i32) -> Vec<u8>;

        /// The number of built-in patterns.
        fn stamp_pattern_count() -> i32;

        /// Built-in pattern `index`'s name, or empty out of range.
        fn stamp_pattern_name(index: i32) -> QString;

        /// Built-in pattern `index` as packed RGBA8888, `stamp_pattern_side()`
        /// square; empty out of range.
        fn stamp_pattern_tile(index: i32) -> Vec<u8>;

        /// The side of every built-in pattern tile, in pixels.
        fn stamp_pattern_side() -> i32;

        /// The History panel row of the brush source: the state index (the
        /// oldest state by default), or -1 when the source is a snapshot or a
        /// state the stack has dropped.
        fn history_brush_source_state(view: &PictureView) -> i32;

        /// The snapshot index of the brush source, or -1.
        fn history_brush_source_snapshot(view: &PictureView) -> i32;

        /// Point the History Brush at state (or, with `snapshot`, named
        /// snapshot) `index`. False when it does not exist.
        fn set_history_brush_source(
            view: Pin<&mut PictureView>,
            snapshot: bool,
            index: i32,
        ) -> bool;
    }
}

fn begin_color_replacement(
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
    begin(view, "Color Replacement Tool", |doc, path, _| {
        Stroke::begin_kind(doc, path, cfg, StrokeKind::Replace(options)).ok()
    })
}

fn begin_mixer_brush(
    view: Pin<&mut PictureView>,
    reservoir: u32,
    tip: &PaintTip,
    wet: i32,
    load: i32,
    mix: i32,
    flow: i32,
) -> bool {
    let unit = |v: i32| v.clamp(0, 100) as f32 / 100.0;
    let kind = StrokeKind::Mixer {
        options: MixerOptions {
            wet: unit(wet),
            load: unit(load),
            mix: unit(mix),
            flow: unit(flow),
        },
        reservoir: rgba_from_argb(reservoir),
    };
    let cfg = tip_config(tip);
    begin(view, "Mixer Brush Tool", |doc, path, _| {
        Stroke::begin_kind(doc, path, cfg, kind).ok()
    })
}

fn mixer_reservoir(view: &PictureView) -> u32 {
    view.rust()
        .stroke
        .as_ref()
        .and_then(Stroke::mixer_reservoir)
        .map_or(0, |c| {
            (c.a as u32) << 24 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32
        })
}

fn tip_config(tip: &PaintTip) -> StrokeConfig {
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

fn begin_brush(
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
        return view.begin_mask_paint(&path, mask_stroke_config(cfg), label);
    }
    begin(view, label, |doc, path, _| {
        Stroke::begin_at(doc, path, cfg).ok()
    })
}

fn begin_clone_stamp(
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
    begin(view, "Clone Stamp", |doc, path, _| {
        let image = match sample_scope(doc, path, sampling, ignore_adjustments) {
            Some(scope) => surface_from_composite(&pictura_render::composite_rgba(&scope)),
            None => layer_surface(doc, path)?,
        };
        let anchor = (anchor_x as f32, anchor_y as f32);
        let source = StampSource::transformed(image, (offset_x, offset_y), anchor, transform);
        Stroke::begin_source(doc, path, cfg, source).ok()
    })
}

fn begin_pattern_stamp(
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
    begin(view, "Pattern Stamp", |doc, path, _| {
        let image = tiled(&tile, doc.width, doc.height, (origin_x, origin_y))?;
        Stroke::begin_source(doc, path, cfg, StampSource::new(image, (0, 0))).ok()
    })
}

fn begin_history_brush(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
) -> bool {
    let cfg = stamp_config(tip, opacity, flow, mode);
    begin(view, "History Brush", |doc, path, view| {
        // ponytail: the source layer is matched by panel path (layers carry no
        // stable id), so a reordered stack paints from whatever now sits there.
        let past = view.history.brush_source_doc()?;
        let source = StampSource::new(layer_surface(&past.doc, path)?, (0, 0));
        Stroke::begin_source(doc, path, cfg, source).ok()
    })
}

fn begin_art_history_brush(
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
    begin(view, "Art History Brush", |doc, path, view| {
        let options = ArtHistoryOptions {
            style: ArtStyle::from_i32(style).unwrap_or_default(),
            area: area.clamp(0, 500) as u32,
            tolerance: tolerance.clamp(0, 100) as u8,
            // A new seed per history state: strokes differ, a stroke replays.
            seed: view.history.index() as u64 ^ 0xA27_4157,
        };
        let past = view.history.brush_source_doc()?;
        Stroke::begin_art_history(doc, path, cfg, layer_surface(&past.doc, path)?, options).ok()
    })
}

fn art_history_style_count() -> i32 {
    STYLE_NAMES.len() as i32
}

fn art_history_style_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| STYLE_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

fn begin_eraser(
    view: Pin<&mut PictureView>,
    background: u32,
    tip: &PaintTip,
    mode: i32,
    opacity: i32,
    flow: i32,
    to_history: bool,
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
        return view.begin_mask_paint(&path, mask_stroke_config(mask_cfg), "Eraser");
    }
    begin(view, "Eraser", |doc, path, view| {
        // ponytail: the source layer is matched by panel path, as for the
        // History Brush.
        let history = if to_history {
            let past = view.history.brush_source_doc()?;
            Some(StampSource::new(layer_surface(&past.doc, path)?, (0, 0)))
        } else {
            None
        };
        begin_erase(doc, path, cfg, mode, history).ok()
    })
}

fn begin_background_eraser(
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
        rust.doc
            .as_ref()
            .zip(rust.active_layer.as_deref())
            .and_then(|(doc, path)| unlocked_background(doc, path))
    };
    if let Some(layered) = layered {
        let mut rust = view.as_mut().rust_mut();
        rust.stroke_base = rust.doc.replace(layered);
    }
    let begun = begin(view.as_mut(), "Background Eraser", |doc, path, _| {
        Stroke::begin_kind(doc, path, cfg, StrokeKind::BackgroundErase(options)).ok()
    });
    if !begun {
        let mut rust = view.as_mut().rust_mut();
        if let Some(base) = rust.stroke_base.take() {
            rust.doc = Some(base);
        }
    }
    begun
}

fn begin_focus(
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
    begin(view, label, |doc, path, view| {
        let sampled = sample_all.then(|| composite_surface(doc, view));
        Stroke::begin_focus(doc, path, cfg, options, sampled).ok()
    })
}

fn begin_smudge(
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
    begin(view, "Smudge", |doc, path, view| {
        let sampled = sample_all.then(|| composite_surface(doc, view));
        Stroke::begin_smudge(doc, path, cfg, options, sampled).ok()
    })
}

fn begin_tone(
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
    begin(view, label, |doc, path, _| {
        Stroke::begin_tone(doc, path, cfg, options).ok()
    })
}

/// The document's composite as a document-space surface: what Sample All
/// Layers reads.
fn composite_surface(doc: &Document, view: &PictureViewRust) -> pictura_paint::healing::RgbaImage {
    surface_from_composite(&super::helpers_composite::current_buffer(
        doc,
        view.gpu_compute,
    ))
}

/// A copy of `doc` with the Background at `path` turned into an ordinary
/// layer with an opaque alpha channel, so it can be erased to transparency;
/// `None` when `path` is not the Background.
fn unlocked_background(doc: &Document, path: &str) -> Option<Document> {
    if !pictura_render::resolve_path(doc, path)?.background {
        return None;
    }
    let mut layered = doc.clone();
    pictura_render::layer_from_background(&mut layered, path);
    ensure_alpha(pictura_render::resolve_path_mut(&mut layered, path)?);
    Some(layered)
}

fn magic_erase_at(
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
        let (Some(doc), Some(path)) = (rust.doc.as_ref(), rust.active_layer.as_deref()) else {
            return false;
        };
        if rust.stroke.is_some()
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
            super::helpers_composite::current_buffer(doc, rust.gpu_compute)
        } else {
            let Some(surface) = layer_surface(doc, path) else {
                return false;
            };
            planar(&surface)
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
            flood.data
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
    view.as_mut().rust_mut().doc = Some(doc);
    view.as_mut().recomposite();
    view.as_mut().record("Magic Eraser");
    true
}

/// A document-space surface as the planar RGBA buffer the Magic Wand reads.
fn planar(surface: &pictura_paint::healing::RgbaImage) -> pictura_core::PixelBuffer {
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

fn stamp_pattern_count() -> i32 {
    PATTERN_NAMES.len() as i32
}

fn stamp_pattern_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| PATTERN_NAMES.get(i))
        .map_or_else(QString::default, |name| QString::from(*name))
}

fn stamp_pattern_tile(index: i32) -> Vec<u8> {
    usize::try_from(index)
        .ok()
        .and_then(pattern::tile)
        .map_or_else(Vec::new, |tile| tile.data.concat())
}

fn stamp_pattern_side() -> i32 {
    pattern::TILE
}

fn history_brush_source_state(view: &PictureView) -> i32 {
    match view.rust().history.brush_source() {
        BrushSource::Oldest => 0,
        BrushSource::State(i) => i as i32,
        _ => -1,
    }
}

fn history_brush_source_snapshot(view: &PictureView) -> i32 {
    match view.rust().history.brush_source() {
        BrushSource::Snapshot(i) => i as i32,
        _ => -1,
    }
}

fn set_history_brush_source(mut view: Pin<&mut PictureView>, snapshot: bool, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let source = if snapshot {
        BrushSource::Snapshot(index)
    } else {
        BrushSource::State(index)
    };
    view.as_mut().rust_mut().history.set_brush_source(source)
}

fn stamp_config(tip: &PaintTip, opacity: i32, flow: i32, mode: &QString) -> StrokeConfig {
    StrokeConfig {
        opacity: opacity.clamp(0, 100) as u8,
        flow: flow.clamp(0, 100) as u8,
        mode: paint_mode_from(&mode.to_string()),
        ..tip_config(tip)
    }
}

fn brush_tip_preview(tip: &PaintTip, width: i32, height: i32) -> Vec<u8> {
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

fn brush_dab_preview(tip: &PaintTip, edge: i32) -> Vec<u8> {
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
fn white_preview(cfg: StrokeConfig, w: i32, h: i32, samples: &[StrokeSample]) -> Vec<u8> {
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    let n = (w * h) as usize;
    doc.layers.push(Layer {
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
fn begin(
    mut view: Pin<&mut PictureView>,
    label: &str,
    start: impl FnOnce(&Document, &str, &PictureViewRust) -> Option<Stroke>,
) -> bool {
    let stroke = {
        let rust = view.rust();
        let (Some(doc), Some(path)) = (rust.doc.as_ref(), rust.active_layer.as_deref()) else {
            return false;
        };
        // ponytail: only the Brush, Pencil, and Eraser paint a mask; the other
        // painting tools refuse rather than write the layer's pixels.
        if rust.stroke.is_some()
            || active_mask_target(rust).is_some()
            || active_pixel_layer(doc, Some(path)).is_none()
            || !active_layer_visible(doc, Some(path))
        {
            return false;
        }
        start(doc, path, rust)
    };
    let Some(stroke) = stroke else {
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    rust.stroke = Some(stroke);
    rust.stroke_label = label.to_string();
    // A new stroke opens its own present frame and tile set, exactly as
    // `begin_paint` does, so the color-replacement and mixer commits decompose
    // into dirty tiles rather than the whole bounding box.
    rust.clear_pending_present();
    let dims = rust.doc.as_ref().map(|doc| (doc.width, doc.height));
    if let Some((width, height)) = dims {
        rust.stroke_tiles.reset(width, height);
    }
    rust.preview = None;
    rust.gpu_stroke = None;
    rust.gpu_placer = None;
    true
}
