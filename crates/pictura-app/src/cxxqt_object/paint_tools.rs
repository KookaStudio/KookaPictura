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
mod imp;

use super::qobject::PictureView;
use imp::*;

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
        /// 0–100 %. With `sample_all` the pickup reads the composite. `opacity`
        /// caps the deposit and `mode` is a Brush mode (`"normal"`, …). False
        /// as for `begin_color_replacement`.
        fn begin_mixer_brush(
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
        /// transparency-locked layer is erased to. `zoom` is the canvas zoom
        /// (screen pixels per document pixel): the Block mode's side is a
        /// screen-space size, so it needs it. With `to_history` it paints
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
            zoom: f64,
        ) -> bool;

        /// The Block eraser's side in document pixels at `zoom`, so the
        /// canvas draws the cursor at the size the block erases.
        fn eraser_block_side_doc_px(zoom: f64) -> u32;

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

        /// Replay a serialized Actions tool stroke (`ToolStroke` JSON) on the
        /// active layer. False, with a named diagnostic on the recorder hook,
        /// for a bad version/payload or an unplayable tool.
        // ponytail: cxx-qt 0.10 has no free qinvokable; the C++ Actions player
        // calls this bridge function directly, as it does the other tool begins.
        fn replay_tool_stroke(view: Pin<&mut PictureView>, payload: &QString) -> bool;
    }
}
