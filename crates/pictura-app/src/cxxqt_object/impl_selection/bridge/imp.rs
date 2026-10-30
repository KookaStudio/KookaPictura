use super::*;

use core::pin::Pin;
use cxx_qt_lib::QString;

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
