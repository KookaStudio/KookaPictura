use super::*;

use crate::cxxqt_object::helpers::{combine_mode_from, feathered, feathered_within};
use crate::cxxqt_object::picture::{sample_tiles, tile_source};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_select::geometry::{marquee_rect, within_radius, MarqueeStyle};
use pictura_select::Selection;

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
pub(super) fn selection_select_ellipse_aa(
    mut view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    mode: &QString,
    feather: f64,
    anti_alias: bool,
) -> bool {
    let shape = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let shape = if anti_alias {
            Selection::ellipse_aa(doc.width, doc.height, x, y, w, h)
        } else {
            Selection::ellipse(doc.width, doc.height, x, y, w, h)
        };
        feathered(shape, feather)
    };
    let mode = combine_mode_from(&mode.to_string());
    view.as_mut()
        .apply_selection_labeled(shape, mode, "Selection")
}

pub(super) fn selection_end_lasso_aa(
    mut view: Pin<&mut PictureView>,
    feather: f64,
    anti_alias: bool,
) -> bool {
    let shape = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let points = rust.session.live.lasso_points().unwrap_or_default();
        if points.len() < 3 {
            return false;
        }
        let (polygon, bounds) = if anti_alias {
            Selection::polygon_bounded_aa(doc.width, doc.height, points)
        } else {
            Selection::polygon_bounded(doc.width, doc.height, points)
        };
        feathered_within(polygon, bounds, feather)
    };
    let mode = combine_mode_from(view.rust().session.live.lasso_mode().unwrap_or_default());
    if !view.as_mut().apply_selection_boxed(shape, mode) {
        return false;
    }
    view.as_mut()
        .rust_mut()
        .session
        .live
        .take_selection_gesture();
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn selection_magic_wand_aa(
    mut view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    tolerance: i32,
    contiguous: bool,
    mode: &QString,
    anti_alias: bool,
) -> bool {
    let shape = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        if x < 0 || y < 0 {
            return false;
        }
        let tolerance = tolerance.clamp(0, 255) as u8;
        let fetch = sample_tiles(doc, rust);
        match pictura_select::magic_wand_from(
            tile_source(doc, &fetch),
            x as u32,
            y as u32,
            tolerance,
            contiguous,
            anti_alias,
        ) {
            Ok(selection) => selection,
            Err(_) => return false,
        }
    };
    let mode = combine_mode_from(&mode.to_string());
    view.as_mut()
        .apply_selection_labeled(shape, mode, "Selection")
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

pub(super) fn save_selection_to(
    view: Pin<&mut PictureView>,
    target: &QString,
    name: &QString,
    mode: &QString,
) -> bool {
    view.save_selection_to(target, name, mode)
}

pub(super) fn load_selection_target(
    view: Pin<&mut PictureView>,
    target: &QString,
    invert: bool,
    mode: &QString,
) -> bool {
    view.load_selection_target(target, invert, mode)
}

pub(super) fn selection_mask_targets(view: &PictureView) -> QStringList {
    view.selection_mask_targets()
}

pub(super) fn selection_coverage_bytes(view: &PictureView) -> Vec<u8> {
    view.selection_coverage_bytes()
}

pub(super) fn channel_coverage(view: &PictureView, target: &QString) -> Vec<u8> {
    view.channel_coverage(target)
}

pub(super) fn save_selection_coverage(
    view: Pin<&mut PictureView>,
    coverage: &[u8],
    width: i32,
    height: i32,
    target: &QString,
    name: &QString,
    mode: &QString,
) -> bool {
    view.save_selection_coverage(coverage, width, height, target, name, mode)
}

pub(super) fn load_selection_coverage(
    view: Pin<&mut PictureView>,
    coverage: &[u8],
    width: i32,
    height: i32,
    invert: bool,
    mode: &QString,
) -> bool {
    view.load_selection_coverage(coverage, width, height, invert, mode)
}

pub(super) fn new_selection_document(
    view: Pin<&mut PictureView>,
    coverage: &[u8],
    width: i32,
    height: i32,
    name: &QString,
) -> bool {
    view.new_selection_document(coverage, width, height, name)
}
