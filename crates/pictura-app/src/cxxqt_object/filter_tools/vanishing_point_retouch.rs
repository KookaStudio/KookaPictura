//! Filter ▸ Vanishing Point's retouch tools (#349): Paste, the Marquee and
//! Transform, Stamp, Brush and Eyedropper over the session in
//! `vanishing_point.rs`, and the preview the dialog draws. Coordinates are
//! document pixels. Free functions (their own bridge).

use cxx_qt_lib::QImage;
use pictura_filters::{VpHeal, VpPasteError, VpRect, VpTexture};

use super::super::clipboard::clipboard_pixels;
use super::super::helpers_composite::rgba_image;
use super::vanishing_point::{flat, with_session};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The preview of document rect `(x, y, w, h)`: the document without
        /// the layer, the edited layer over it, and any floating selection.
        fn vp_view(x: i32, y: i32, w: i32, h: i32) -> QImage;

        /// The document rect `[x, y, w, h]` the preview must redraw since the
        /// last call; empty when nothing changed.
        fn vp_view_dirty() -> Vec<i32>;

        /// Paste the clipboard as a floating selection in the active plane:
        /// 0 pasted, 1 no valid plane, 2 nothing on the clipboard.
        fn vp_paste_clipboard() -> i32;

        fn vp_has_float() -> bool;
        fn vp_has_marquee() -> bool;
        /// The floating selection's and the marquee's outlines, `x, y` pairs.
        fn vp_float_outline() -> Vec<f64>;
        fn vp_marquee_outline() -> Vec<f64>;
        /// The floating selection's four corners (Transform's handles).
        fn vp_float_corners() -> Vec<f64>;

        /// Marquee press at `(x, y)` with Alt / Ctrl held; false when it does
        /// nothing (outside every plane).
        fn vp_marquee_press(x: f64, y: f64, alt: bool, ctrl: bool) -> bool;
        /// Marquee drag; `square` (Shift) keeps a new marquee square.
        fn vp_marquee_drag(x: f64, y: f64, square: bool);
        /// End a Marquee, Transform, Stamp or Brush gesture: one undo step.
        fn vp_gesture_release();
        /// Double-click with the Marquee: select the plane under `(x, y)`.
        fn vp_select_plane(x: f64, y: f64) -> bool;

        /// Transform press: a corner within `tolerance` px scales, just past
        /// one rotates, inside moves. False elsewhere or without a float.
        fn vp_transform_press(x: f64, y: f64, tolerance: f64) -> bool;
        /// Transform drag; `uniform` (Shift) keeps the aspect.
        fn vp_transform_drag(x: f64, y: f64, uniform: bool);
        /// Flip (`horizontal`) or Flop the floating selection.
        fn vp_mirror(horizontal: bool) -> bool;

        /// Put the floating selection down into the layer.
        fn vp_drop_float() -> bool;
        /// Drop the floating selection and clear the marquee (Ctrl+D).
        fn vp_deselect();

        /// Alt-click with the Stamp: clone from `(x, y)`.
        fn vp_stamp_source(x: f64, y: f64) -> bool;
        fn vp_has_stamp_source() -> bool;
        /// Start a Brush (`brush`) or Stamp stroke; false outside every plane
        /// or for a Stamp with no source.
        fn vp_stroke_press(brush: bool, x: f64, y: f64) -> bool;
        fn vp_stroke_drag(x: f64, y: f64);

        /// The colour the preview shows at `(x, y)`, `[r, g, b]`.
        fn vp_sample(x: f64, y: f64) -> Vec<i32>;

        /// Marquee options: Feather px, Opacity %, Heal (0 Off, 1 Luminance,
        /// 2 On), Move Mode Source.
        fn vp_set_marquee_options(feather: f64, opacity: f64, heal: i32, source: bool);
        /// Stamp (`brush` false) or Brush options: Diameter px, Hardness %,
        /// Opacity %, Heal.
        fn vp_set_paint_options(brush: bool, diameter: f64, hardness: f64, opacity: f64, heal: i32);
        fn vp_set_aligned(aligned: bool);
        fn vp_set_brush_colour(r: i32, g: i32, b: i32);
    }
}

fn vp_view(x: i32, y: i32, w: i32, h: i32) -> QImage {
    if w <= 0 || h <= 0 {
        return QImage::default();
    }
    let r = VpRect {
        x0: x,
        y0: y,
        x1: x + w,
        y1: y + h,
    };
    with_session(QImage::default(), |s| {
        let (r, rgba) = s.editor.compose(r);
        if r.is_empty() {
            return QImage::default();
        }
        rgba_image(rgba, r.width() as i32, r.height() as i32)
    })
}

fn vp_view_dirty() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        s.editor.take_dirty().map_or_else(Vec::new, |r| {
            vec![r.x0, r.y0, r.width() as i32, r.height() as i32]
        })
    })
}

fn vp_paste_clipboard() -> i32 {
    let Some(tex) =
        clipboard_pixels().and_then(|(w, h, rgba)| VpTexture::new(w as usize, h as usize, rgba))
    else {
        return 2;
    };
    with_session(1, |s| match s.editor.paste(tex) {
        Ok(()) => 0,
        Err(VpPasteError::NoPlane) => 1,
        Err(VpPasteError::Empty) => 2,
    })
}

fn vp_has_float() -> bool {
    with_session(false, |s| s.editor.has_float())
}

fn vp_has_marquee() -> bool {
    with_session(false, |s| s.editor.has_marquee())
}

fn vp_float_outline() -> Vec<f64> {
    with_session(Vec::new(), |s| flat(&s.editor.float_outline()))
}

fn vp_marquee_outline() -> Vec<f64> {
    with_session(Vec::new(), |s| flat(&s.editor.marquee_outline()))
}

fn vp_float_corners() -> Vec<f64> {
    with_session(Vec::new(), |s| flat(&s.editor.float_corners()))
}

fn vp_marquee_press(x: f64, y: f64, alt: bool, ctrl: bool) -> bool {
    with_session(false, |s| s.editor.marquee_press((x, y), alt, ctrl))
}

fn vp_marquee_drag(x: f64, y: f64, square: bool) {
    with_session((), |s| s.editor.marquee_drag((x, y), square));
}

fn vp_gesture_release() {
    with_session((), |s| s.editor.gesture_release());
}

fn vp_select_plane(x: f64, y: f64) -> bool {
    with_session(false, |s| s.editor.select_plane((x, y)))
}

fn vp_transform_press(x: f64, y: f64, tolerance: f64) -> bool {
    with_session(false, |s| s.editor.transform_press((x, y), tolerance))
}

fn vp_transform_drag(x: f64, y: f64, uniform: bool) {
    with_session((), |s| s.editor.transform_drag((x, y), uniform));
}

fn vp_mirror(horizontal: bool) -> bool {
    with_session(false, |s| s.editor.mirror(horizontal))
}

fn vp_drop_float() -> bool {
    with_session(false, |s| s.editor.drop_float())
}

fn vp_deselect() {
    with_session((), |s| s.editor.deselect());
}

fn vp_stamp_source(x: f64, y: f64) -> bool {
    with_session(false, |s| s.editor.set_stamp_source((x, y)))
}

fn vp_has_stamp_source() -> bool {
    with_session(false, |s| s.editor.has_stamp_source())
}

fn vp_stroke_press(brush: bool, x: f64, y: f64) -> bool {
    with_session(false, |s| s.editor.stroke_press(brush, (x, y)))
}

fn vp_stroke_drag(x: f64, y: f64) {
    with_session((), |s| s.editor.stroke_drag((x, y)));
}

fn vp_sample(x: f64, y: f64) -> Vec<i32> {
    with_session(Vec::new(), |s| {
        s.editor
            .sample((x, y))
            .map_or_else(Vec::new, |c| c.iter().map(|&v| i32::from(v)).collect())
    })
}

fn percent(v: f64) -> f64 {
    (v / 100.0).clamp(0.0, 1.0)
}

fn vp_set_marquee_options(feather: f64, opacity: f64, heal: i32, source: bool) {
    with_session((), |s| {
        if let Some(r) = s.editor.retouch_mut() {
            r.marquee.feather = feather.max(0.0);
            r.marquee.opacity = percent(opacity);
            r.marquee.heal = VpHeal::from_i32(heal);
            r.marquee.source = source;
        }
        // A float on screen shows its new Feather and Opacity.
        if s.editor.has_float() {
            s.editor.invalidate();
        }
    });
}

fn vp_set_paint_options(brush: bool, diameter: f64, hardness: f64, opacity: f64, heal: i32) {
    with_session((), |s| {
        if let Some(r) = s.editor.retouch_mut() {
            let o = if brush { &mut r.brush } else { &mut r.stamp };
            o.diameter = diameter.clamp(1.0, 5000.0);
            o.hardness = percent(hardness);
            o.opacity = percent(opacity);
            o.heal = VpHeal::from_i32(heal);
        }
    });
}

fn vp_set_aligned(aligned: bool) {
    with_session((), |s| {
        if let Some(r) = s.editor.retouch_mut() {
            r.aligned = aligned;
        }
    });
}

fn vp_set_brush_colour(r: i32, g: i32, b: i32) {
    let byte = |v: i32| v.clamp(0, 255) as u8;
    with_session((), |s| {
        if let Some(t) = s.editor.retouch_mut() {
            t.colour = [byte(r), byte(g), byte(b)];
        }
    });
}
