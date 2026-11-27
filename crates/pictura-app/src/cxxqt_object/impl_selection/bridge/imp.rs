use super::*;

use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::{combine_mode_from, feathered, feathered_within};
use crate::cxxqt_object::picture::{sample_tiles, tile_source};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_select::clip_selection_shape_to_canvas;
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

pub(super) fn selection_content_movable(view: &PictureView, duplicate: bool) -> bool {
    view.selection_content_movable(duplicate)
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

fn invalid_clip_result() -> ffi::SelectionClipResult {
    ffi::SelectionClipResult {
        valid: false,
        loop_lengths: Vec::new(),
        points: Vec::new(),
    }
}

pub(super) fn selection_clip_shape_to_canvas(
    loop_lengths: &[u32],
    points: &[f64],
    canvas_width: f64,
    canvas_height: f64,
) -> ffi::SelectionClipResult {
    if !points.len().is_multiple_of(2) {
        return invalid_clip_result();
    }
    let pair_count = points.len() / 2;
    let mut remaining = pair_count;
    let mut consumed = 0usize;
    let mut loops = Vec::with_capacity(loop_lengths.len());
    for &length in loop_lengths {
        let count = length as usize;
        if count > remaining {
            return invalid_clip_result();
        }
        let Some(end) = consumed.checked_add(count) else {
            return invalid_clip_result();
        };
        loops.push(
            (consumed..end)
                .map(|index| (points[index * 2], points[index * 2 + 1]))
                .collect(),
        );
        consumed = end;
        remaining -= count;
    }
    if remaining != 0 {
        return invalid_clip_result();
    }

    let clipped = clip_selection_shape_to_canvas(&loops, (canvas_width, canvas_height));
    let mut result = ffi::SelectionClipResult {
        valid: true,
        loop_lengths: Vec::with_capacity(clipped.len()),
        points: Vec::with_capacity(consumed),
    };
    for loop_points in clipped {
        result.loop_lengths.push(loop_points.len() as u32);
        for (x, y) in loop_points {
            result.points.push(x);
            result.points.push(y);
        }
    }
    result
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

pub(super) fn selection_below_half(view: &PictureView) -> bool {
    let rust = view.rust();
    let Some(sel) = rust.session.selection.as_ref() else {
        return false;
    };
    let Some(bounds) = rust.selection_mask_bounds() else {
        return false;
    };
    let xs = bounds.left as usize..bounds.right as usize;
    let ys = bounds.top as usize..bounds.bottom as usize;
    let mut nonzero = false;
    let mut above = false;
    sel.data.for_each_tile(xs.clone(), ys.clone(), sel.width as usize, |tile| {
        if above {
            return;
        }
        match tile.uniform() {
            Some(v) => {
                nonzero |= v > 0;
                above |= v > 127;
            }
            None => {
                let cols = xs.start.max(tile.x)..xs.end.min(tile.x + tile.w);
                let rows = ys.start.max(tile.y)..ys.end.min(tile.y + tile.h);
                for y in rows {
                    let row = tile.row(y - tile.y).unwrap_or(&[]);
                    for &v in &row[cols.start - tile.x..cols.end - tile.x] {
                        nonzero |= v > 0;
                        above |= v > 127;
                    }
                }
            }
        }
    });
    nonzero && !above
}

pub(super) fn selection_coverage_bytes(view: &PictureView) -> Vec<u8> {
    view.selection_coverage_bytes()
}

pub(super) fn channel_coverage(view: &PictureView, target: &QString) -> Vec<u8> {
    view.channel_coverage(target)
}

pub(super) fn color_row_coverage(view: &PictureView, row: i32, mask: i32) -> Vec<u8> {
    view.color_row_coverage(row, mask)
}

pub(super) fn transfer_duplicate_channel(
    view: Pin<&mut PictureView>,
    source: &PictureView,
    source_target: &QString,
    source_mask: i32,
    name: &QString,
    invert: bool,
) -> bool {
    if view.rust().session.read_only()
        || !view
            .rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| doc.can_add_channel())
    {
        return false;
    }
    let spec = source_target.to_string();
    let Some(shape) = source.rust().transfer_source(&spec, source_mask as u8) else {
        return false;
    };
    let name = name.to_string();
    let outcome = view.edit("Duplicate Channel", |rust| {
        rust.transfer_channel(shape, &name, invert)
    });
    !matches!(outcome, Edit::None)
}

pub(super) fn transfer_save_selection(
    view: Pin<&mut PictureView>,
    source: &PictureView,
    source_target: &QString,
    source_mask: i32,
    target: &QString,
    name: &QString,
    mode: &QString,
) -> bool {
    if view.rust().session.read_only() {
        return false;
    }
    let spec = source_target.to_string();
    let Some(shape) = source.rust().transfer_source(&spec, source_mask as u8) else {
        return false;
    };
    let target = target.to_string();
    let name = name.to_string();
    let mode = combine_mode_from(&mode.to_string());
    let outcome = view.edit("Save Selection", |rust| {
        rust.transfer_save(shape, &target, &name, mode)
    });
    !matches!(outcome, Edit::None)
}

pub(super) fn transfer_load_selection(
    view: Pin<&mut PictureView>,
    source: &PictureView,
    source_target: &QString,
    source_mask: i32,
    invert: bool,
    fit: bool,
    mode: &QString,
) -> bool {
    if view.rust().session.read_only() {
        return false;
    }
    let spec = source_target.to_string();
    let Some(shape) = source.rust().transfer_source(&spec, source_mask as u8) else {
        return false;
    };
    let mode = combine_mode_from(&mode.to_string());
    let outcome = view.edit("Load Selection", |rust| {
        rust.transfer_load(shape, invert, fit, mode)
    });
    !matches!(outcome, Edit::None)
}

pub(super) fn transfer_new_document(
    mut view: Pin<&mut PictureView>,
    source: &PictureView,
    source_target: &QString,
    source_mask: i32,
    name: &QString,
    invert: bool,
) -> bool {
    let spec = source_target.to_string();
    let Some(shape) = source.rust().transfer_source(&spec, source_mask as u8) else {
        return false;
    };
    let name = name.to_string();
    view.as_mut()
        .rust_mut()
        .transfer_document(shape, &name, invert)
}

pub(super) fn duplicate_channel_in_view(
    view: Pin<&mut PictureView>,
    source_target: &QString,
    source_mask: i32,
    name: &QString,
    invert: bool,
) -> bool {
    if view.rust().session.read_only()
        || !view
            .rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| doc.can_add_channel())
    {
        return false;
    }
    let spec = source_target.to_string();
    let Some(shape) = view.rust().transfer_source(&spec, source_mask as u8) else {
        return false;
    };
    let name = name.to_string();
    let outcome = view.edit("Duplicate Channel", |rust| {
        rust.transfer_channel(shape, &name, invert)
    });
    !matches!(outcome, Edit::None)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_clip_shape_to_canvas_rejects_malformed_payloads() {
        let cases = vec![
            (&[1u32][..], vec![0.0, 0.0, 1.0]),
            (&[u32::MAX, 1][..], vec![0.0, 0.0, 1.0, 1.0]),
            (&[3u32][..], vec![0.0, 0.0, 1.0, 1.0]),
            (&[1u32][..], vec![0.0, 0.0, 1.0, 1.0]),
        ];
        for (lengths, points) in cases {
            let result = selection_clip_shape_to_canvas(lengths, &points, 64.0, 64.0);
            assert!(!result.valid);
            assert!(result.loop_lengths.is_empty());
            assert!(result.points.is_empty());
        }
    }

    #[test]
    fn selection_clip_shape_to_canvas_round_trips_valid_payloads() {
        let points = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = selection_clip_shape_to_canvas(&[1, 2], &points, 0.0, 64.0);
        assert!(result.valid);
        assert_eq!(result.loop_lengths, vec![1u32, 2u32]);
        assert_eq!(result.points, points.to_vec());
    }
}
