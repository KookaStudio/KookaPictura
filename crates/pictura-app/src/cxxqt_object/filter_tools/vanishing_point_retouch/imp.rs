use cxx_qt_lib::QImage;
use pictura_filters::{VpHeal, VpPasteError, VpRect, VpTexture};

use super::super::super::clipboard::clipboard_pixels;
use super::super::super::helpers_composite::rgba_image;
use super::super::vanishing_point::{flat, with_session};

pub(super) fn vp_view(x: i32, y: i32, w: i32, h: i32) -> QImage {
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

pub(super) fn vp_view_dirty() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        s.editor.take_dirty().map_or_else(Vec::new, |r| {
            vec![r.x0, r.y0, r.width() as i32, r.height() as i32]
        })
    })
}

pub(super) fn vp_paste_clipboard() -> i32 {
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

pub(super) fn vp_has_float() -> bool {
    with_session(false, |s| s.editor.has_float())
}

pub(super) fn vp_has_marquee() -> bool {
    with_session(false, |s| s.editor.has_marquee())
}

pub(super) fn vp_float_outline() -> Vec<f64> {
    with_session(Vec::new(), |s| flat(&s.editor.float_outline()))
}

pub(super) fn vp_marquee_outline() -> Vec<f64> {
    with_session(Vec::new(), |s| flat(&s.editor.marquee_outline()))
}

pub(super) fn vp_float_corners() -> Vec<f64> {
    with_session(Vec::new(), |s| flat(&s.editor.float_corners()))
}

pub(super) fn vp_marquee_press(x: f64, y: f64, alt: bool, ctrl: bool) -> bool {
    with_session(false, |s| s.editor.marquee_press((x, y), alt, ctrl))
}

pub(super) fn vp_marquee_drag(x: f64, y: f64, square: bool) {
    with_session((), |s| s.editor.marquee_drag((x, y), square));
}

pub(super) fn vp_gesture_release() {
    with_session((), |s| s.editor.gesture_release());
}

pub(super) fn vp_select_plane(x: f64, y: f64) -> bool {
    with_session(false, |s| s.editor.select_plane((x, y)))
}

pub(super) fn vp_transform_press(x: f64, y: f64, tolerance: f64) -> bool {
    with_session(false, |s| s.editor.transform_press((x, y), tolerance))
}

pub(super) fn vp_transform_drag(x: f64, y: f64, uniform: bool) {
    with_session((), |s| s.editor.transform_drag((x, y), uniform));
}

pub(super) fn vp_mirror(horizontal: bool) -> bool {
    with_session(false, |s| s.editor.mirror(horizontal))
}

pub(super) fn vp_drop_float() -> bool {
    with_session(false, |s| s.editor.drop_float())
}

pub(super) fn vp_deselect() {
    with_session((), |s| s.editor.deselect());
}

pub(super) fn vp_stamp_source(x: f64, y: f64) -> bool {
    with_session(false, |s| s.editor.set_stamp_source((x, y)))
}

pub(super) fn vp_has_stamp_source() -> bool {
    with_session(false, |s| s.editor.has_stamp_source())
}

pub(super) fn vp_stroke_press(brush: bool, x: f64, y: f64) -> bool {
    with_session(false, |s| s.editor.stroke_press(brush, (x, y)))
}

pub(super) fn vp_stroke_drag(x: f64, y: f64) {
    with_session((), |s| s.editor.stroke_drag((x, y)));
}

pub(super) fn vp_sample(x: f64, y: f64) -> Vec<i32> {
    with_session(Vec::new(), |s| {
        s.editor
            .sample((x, y))
            .map_or_else(Vec::new, |c| c.iter().map(|&v| i32::from(v)).collect())
    })
}

pub(super) fn percent(v: f64) -> f64 {
    (v / 100.0).clamp(0.0, 1.0)
}

pub(super) fn vp_set_marquee_options(feather: f64, opacity: f64, heal: i32, source: bool) {
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

pub(super) fn vp_set_paint_options(
    brush: bool,
    diameter: f64,
    hardness: f64,
    opacity: f64,
    heal: i32,
) {
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

pub(super) fn vp_set_aligned(aligned: bool) {
    with_session((), |s| {
        if let Some(r) = s.editor.retouch_mut() {
            r.aligned = aligned;
        }
    });
}

pub(super) fn vp_set_brush_colour(r: i32, g: i32, b: i32) {
    let byte = |v: i32| v.clamp(0, 255) as u8;
    with_session((), |s| {
        if let Some(t) = s.editor.retouch_mut() {
            t.colour = [byte(r), byte(g), byte(b)];
        }
    });
}
