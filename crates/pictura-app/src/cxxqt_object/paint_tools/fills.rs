//! The fill tool bridges: the Gradient and the Paint Bucket. Free functions
//! over a [`PictureView`], as in the parent module. One gesture is one edit,
//! so each applies it through the selection, recomposites, and records its own
//! history state (`"Gradient"`, `"Paint Bucket"`). The shape tools' Fill
//! Pixels mode shares [`apply`].
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
pub(crate) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The number of built-in gradients.
        fn gradient_preset_count() -> i32;

        /// Built-in gradient `index`'s name, or empty out of range.
        fn gradient_preset_name(index: i32) -> QString;

        /// Built-in gradient `index` between `foreground` and `background`
        /// (`0xAARRGGBB`) as a horizontal strip, packed straight RGBA8888
        /// `width` × `height`; empty out of range.
        fn gradient_preset_strip(
            index: i32,
            foreground: u32,
            background: u32,
            width: i32,
            height: i32,
        ) -> Vec<u8>;

        /// Gradient tool: draw built-in gradient `preset` over the active
        /// pixel layer along the drag from `(x0, y0)` to `(x1, y1)` (document
        /// space), inside the selection when there is one. `style` 0 Linear /
        /// 1 Radial / 2 Angle / 3 Reflected / 4 Diamond; `mode` is a Brush mode
        /// (`"normal"`, …); `opacity` 0–100 %; `transparency` honours the
        /// gradient's own alpha. One "Gradient" state; false when nothing
        /// changed (a drag with no length), without a visible lone pixel
        /// layer, when its pixels are locked, or mid-stroke.
        fn draw_gradient(
            view: Pin<&mut PictureView>,
            preset: i32,
            foreground: u32,
            background: u32,
            style: i32,
            mode: &QString,
            opacity: i32,
            reverse: bool,
            dither: bool,
            transparency: bool,
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
        ) -> bool;

        /// Paint Bucket: fill the pixels that match the one at document point
        /// `(x, y)` — `tolerance` 0–255 per channel, `contiguous`, read from
        /// the active layer or with `all_layers` the composite — with
        /// `foreground` (`0xAARRGGBB`), or built-in pattern `pattern` when it
        /// is not negative, at `opacity` 0–100 % in Brush mode `mode`, with a
        /// softened edge when `antialias`, inside the selection when there is
        /// one. One "Paint Bucket" state; false as for `draw_gradient`, for a
        /// point off the canvas, or for an unknown pattern.
        fn bucket_fill_at(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            foreground: u32,
            pattern: i32,
            mode: &QString,
            opacity: i32,
            tolerance: i32,
            antialias: bool,
            contiguous: bool,
            all_layers: bool,
        ) -> bool;

        /// `Edit ▸ Fill…`: fill the active pixel layer (its selection when there
        /// is one) with `foreground` (`0xAARRGGBB`), or built-in pattern
        /// `pattern` when it is not negative, at `opacity` 0–100 % in Brush mode
        /// `mode`. `preserve_transparency` confines the fill to pixels that are
        /// already opaque. One "Fill" state; false without a visible lone pixel
        /// layer, when its pixels are locked, or when nothing changed.
        fn edit_fill(
            view: Pin<&mut PictureView>,
            foreground: u32,
            pattern: i32,
            mode: &QString,
            opacity: i32,
            preserve_transparency: bool,
        ) -> bool;

        /// `Edit ▸ Stroke…`: outline the active selection on the active pixel
        /// layer with a solid `color` (`0xAARRGGBB`) band `width` px wide,
        /// `position` 0 Inside / 1 Center / 2 Outside, at `opacity` 0–100 % in
        /// Brush mode `mode`. One "Stroke" state; false without a pixel
        /// selection, without a visible lone pixel layer, or when nothing
        /// changed.
        fn edit_stroke(
            view: Pin<&mut PictureView>,
            color: u32,
            width: i32,
            position: i32,
            mode: &QString,
            opacity: i32,
        ) -> bool;
    }
}
