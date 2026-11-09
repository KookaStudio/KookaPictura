use super::*;

use core::pin::Pin;
use cxx_qt_lib::QString;
use pictura_select::geometry::{marquee_rect, within_radius, MarqueeStyle};

pub(super) fn begin_selection_move(view: Pin<&mut PictureView>) -> bool {
    view.begin_selection_move()
}

pub(super) fn preview_selection_move(view: Pin<&mut PictureView>, dx: i32, dy: i32) -> bool {
    view.preview_selection_move(dx, dy)
}

pub(super) fn commit_selection_move(view: Pin<&mut PictureView>) -> bool {
    view.commit_selection_move()
}

pub(super) fn cancel_selection_move(view: Pin<&mut PictureView>) -> bool {
    view.cancel_selection_move()
}

pub(super) fn move_selection_content(
    view: Pin<&mut PictureView>,
    dx: i32,
    dy: i32,
    duplicate: bool,
    background: u32,
) -> bool {
    view.move_selection_content(dx, dy, duplicate, background)
}

pub(super) fn quick_select(
    view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    tolerance: i32,
    mode: &QString,
) -> bool {
    view.quick_select(x, y, tolerance, mode)
}

pub(super) fn quick_select_begin(view: Pin<&mut PictureView>) -> bool {
    view.quick_select_begin()
}

pub(super) fn quick_select_preview(
    view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    tolerance: i32,
    mode: &QString,
    is_move: bool,
) -> bool {
    view.quick_select_preview(x, y, tolerance, mode, is_move)
}

pub(super) fn quick_select_commit(view: Pin<&mut PictureView>) -> bool {
    view.quick_select_commit()
}

pub(super) fn quick_select_cancel(view: Pin<&mut PictureView>) -> bool {
    view.quick_select_cancel()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn selection_marquee_rect(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    shift: bool,
    mirror: bool,
    style: i32,
    ratio_w: f64,
    ratio_h: f64,
    fixed_w: i32,
    fixed_h: i32,
) -> Vec<i32> {
    let style = MarqueeStyle::from_i32(style).unwrap_or(MarqueeStyle::Normal);
    let (x, y, width, height) = marquee_rect(
        (x0, y0),
        (x1, y1),
        shift,
        mirror,
        style,
        (ratio_w, ratio_h),
        (fixed_w, fixed_h),
    );
    vec![x, y, width, height]
}

pub(super) fn selection_within_radius(x0: f64, y0: f64, x1: f64, y1: f64, radius: f64) -> bool {
    within_radius((x0, y0), (x1, y1), radius)
}
