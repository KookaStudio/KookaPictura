//! Selection-border transforms: resampling a coverage mask through the
//! session's map, with or without a Warp.
//!
//! The mask is document-sized coverage; a border transform replaces it with a
//! resampled canvas-sized mask. Only the destination box is resampled, so the
//! work follows the selection, not the document. See
//! `docs/08-selection/transform-selection.md` and the `transform-selection`
//! change.

use pictura_core::memory::{self, MemoryError};
use pictura_core::warp::Warp;
use pictura_core::{Plane, PsdRect, Samples, TILE};
use pictura_select::Selection;

use super::crossed_quad::{quad_is_crossed, CrossedMap};
use super::kernel::Interpolation;
use super::transform::{integer_bbox, resample_plane, solve_homography, PlaneMap};
use super::warp::warp_is_valid;
use super::warp_raster::{triangulate, Kind, SrcPlane};

type Point = (f64, f64);
type Quad = [Point; 4];

/// The coverage's canvas rectangle.
fn canvas_rect(width: usize, height: usize) -> PsdRect {
    PsdRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    }
}

/// Source-rect corners in TL, TR, BR, BL order.
fn corners(rect: PsdRect) -> Quad {
    [
        (f64::from(rect.left), f64::from(rect.top)),
        (f64::from(rect.right), f64::from(rect.top)),
        (f64::from(rect.right), f64::from(rect.bottom)),
        (f64::from(rect.left), f64::from(rect.bottom)),
    ]
}

fn rect_of(dest: (i32, i32, i32, i32)) -> PsdRect {
    PsdRect {
        left: dest.0,
        top: dest.1,
        right: dest.2,
        bottom: dest.3,
    }
}

/// `dest` clipped to the canvas, or `None` when nothing lands inside it.
fn clip_dest(
    dest: Option<(i32, i32, i32, i32)>,
    width: usize,
    height: usize,
) -> Option<(i32, i32, i32, i32)> {
    let (left, top, right, bottom) = dest?;
    let left = left.max(0);
    let top = top.max(0);
    let right = right.min(width as i32);
    let bottom = bottom.min(height as i32);
    (right > left && bottom > top).then_some((left, top, right, bottom))
}

/// An empty selection for a valid transform whose result is off-canvas.
fn empty(width: usize, height: usize) -> Selection {
    Selection::none(width as u32, height as u32)
}

/// Whether `quad` differs from the rect's corners.
fn quad_moved(quad: &Quad, rect: PsdRect) -> bool {
    quad.iter()
        .zip(corners(rect))
        .any(|(q, c)| (q.0 - c.0).abs() > 1e-9 || (q.1 - c.1).abs() > 1e-9)
}

/// Map a source point through the same map `transform_selection` draws with:
/// the two-triangle hourglass for a crossed quad, else the box->quad
/// homography. `None` for a singular map.
pub fn selection_transform_point(rect: PsdRect, quad: Quad, x: f64, y: f64) -> Option<(f64, f64)> {
    if quad_is_crossed(&quad) {
        return CrossedMap::new(rect, quad)?.forward(x, y);
    }
    solve_homography(corners(rect), quad)?.forward(x, y)
}

/// The bounds of the coverage inside `rect`, clipped to the canvas.
fn clipped_rect(rect: PsdRect, width: usize, height: usize) -> Option<PsdRect> {
    let canvas = canvas_rect(width, height);
    let hit = rect.intersect(canvas)?;
    (hit.width() > 0 && hit.height() > 0).then_some(hit)
}

/// Copy the `rect` window of a canvas-sized mask into a rect-sized plane.
fn crop(selection: &Selection, rect: PsdRect, width: usize) -> Result<Plane<u8>, MemoryError> {
    let w = rect.width() as usize;
    let h = rect.height() as usize;
    let mut strip = memory::try_buffer(w * TILE.min(h.max(1)), 0u8, "a selection transform")?;
    Plane::build_tiled_strips(w, h, 1, &mut strip, |y0, out| -> Result<(), MemoryError> {
        let rows = out.len() / w.max(1);
        for r in 0..rows {
            let source_row = rect.top as usize + y0 + r;
            let start = source_row * width + rect.left as usize;
            let dst = r * w;
            selection.data.read_into(start, &mut out[dst..dst + w]);
        }
        Ok(())
    })
}

/// Blit a `dest`-sized plane into a canvas-sized tiled mask, zero elsewhere.
fn place_in_canvas(
    width: usize,
    height: usize,
    dest: (i32, i32, i32, i32),
    plane: Plane<u8>,
) -> Result<Selection, MemoryError> {
    let (dl, dt, dr, db) = dest;
    let (dw, dh) = ((dr - dl) as usize, (db - dt) as usize);
    let mut selection = Selection::none(width as u32, height as u32);
    if dw == 0 || dh == 0 {
        return Ok(selection);
    }
    let band_rows = TILE.min(dh);
    let mut strip = memory::try_buffer(dw * band_rows, 0u8, "a selection transform")?;
    for top in (0..dh).step_by(band_rows) {
        let rows = band_rows.min(dh - top);
        let out = &mut strip[..dw * rows];
        plane.read_rect_into(0..dw, top..top + rows, dw, out);
        selection.data.write_rect_from(
            dl as usize..dr as usize,
            dt as usize + top..dt as usize + top + rows,
            width,
            out,
        );
    }
    selection.data.settle();
    Ok(selection)
}

/// Resample `selection` through the homography sending `rect`'s corners to
/// `quad`, clipped to the canvas.
///
/// `None` refuses a singular map. A valid map whose result lies off the canvas
/// returns an empty selection. The source mask is not modified.
pub fn transform_selection(
    selection: &Selection,
    rect: PsdRect,
    quad: Quad,
    interpolation: Interpolation,
) -> Result<Option<Selection>, MemoryError> {
    let (width, height) = (selection.width as usize, selection.height as usize);
    let Some(bbox) = integer_bbox(&quad) else {
        return Ok(None);
    };
    let Some(dest) = clip_dest(Some(bbox), width, height) else {
        return Ok(Some(empty(width, height)));
    };
    let canvas = canvas_rect(width, height);
    let plane = if quad_is_crossed(&quad) {
        let Some(map) = CrossedMap::new(rect, quad) else {
            return Ok(None);
        };
        resample_plane(
            &selection.data,
            width,
            height,
            &map,
            canvas,
            dest,
            interpolation,
        )?
    } else {
        let Some(map) = solve_homography(corners(rect), quad) else {
            return Ok(None);
        };
        resample_plane(
            &selection.data,
            width,
            height,
            &map,
            canvas,
            dest,
            interpolation,
        )?
    };
    Ok(Some(place_in_canvas(width, height, dest, plane)?))
}

/// The effective warp frame's destination box: the warp bounds carried through
/// `warp.map` and then the box's `quad`, with no canvas clipping.
///
/// `None` for an invalid warp or a singular frame; `Some(rect)` for an
/// identity warp with no moved quad.
pub fn warp_dest_rect(rect: PsdRect, warp: &Warp, quad: Option<Quad>) -> Option<PsdRect> {
    if !warp_is_valid(warp) {
        return None;
    }
    let bounds = corners(rect);
    let quad = quad.filter(|q| quad_moved(q, rect));
    if warp.is_identity() && quad.is_none() {
        return Some(rect);
    }
    let frame = match quad {
        Some(q) => Some(solve_homography(bounds, q)?),
        None => None,
    };
    let map = |x: f64, y: f64| {
        let (mx, my) = warp.map(x, y);
        match &frame {
            Some(h) => h.forward(mx, my).unwrap_or((f64::NAN, f64::NAN)),
            None => (mx, my),
        }
    };
    let t = triangulate(rect, &map)?;
    Some(rect_of(t.dest))
}

/// Warp `selection` through `warp.warp` and then the box's `quad`, clipped to
/// the canvas.
///
/// `None` refuses an invalid warp or a singular frame. An identity warp with an
/// unmoved quad returns the mask unchanged. The source mask is not modified.
pub fn warp_selection(
    selection: &Selection,
    rect: PsdRect,
    warp: &Warp,
    quad: Option<Quad>,
    interpolation: Interpolation,
) -> Result<Option<Selection>, MemoryError> {
    let (width, height) = (selection.width as usize, selection.height as usize);
    if !warp_is_valid(warp) {
        return Ok(None);
    }
    let bounds = corners(rect);
    let quad = quad.filter(|q| quad_moved(q, rect));
    if warp.is_identity() && quad.is_none() {
        return Ok(Some(selection.clone()));
    }
    let frame = match quad {
        Some(q) => Some(match solve_homography(bounds, q) {
            Some(frame) => frame,
            None => return Ok(None),
        }),
        None => None,
    };
    let Some(source) = clipped_rect(rect, width, height) else {
        return Ok(Some(empty(width, height)));
    };
    let cropped = crop(selection, source, width)?;
    let map = |x: f64, y: f64| {
        let (mx, my) = warp.map(x, y);
        match &frame {
            Some(h) => h.forward(mx, my).unwrap_or((f64::NAN, f64::NAN)),
            None => (mx, my),
        }
    };
    let Some(mut t) = triangulate(source, &map) else {
        return Ok(None);
    };
    let Some(dest) = clip_dest(Some(t.dest), width, height) else {
        return Ok(Some(empty(width, height)));
    };
    // Rasterize only the part inside the canvas.
    t.dest = dest;
    // The warp map is defined over the session box; pixels pulled from outside
    // the source bounds are not part of the selection. High-quality presets are
    // measured approximations (see `warp.rs`).
    let Samples::U8(warped) = super::warp::warp_gray(
        SrcPlane::U8(&cropped),
        Kind::U8,
        source,
        0.0,
        &t,
        interpolation,
    ) else {
        return Ok(None);
    };
    Ok(Some(place_in_canvas(width, height, dest, warped)?))
}
