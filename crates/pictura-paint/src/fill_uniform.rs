//! The 8-bit fill over a tiled layer (#1213, #1214): only the tiles the
//! fill's box crosses are visited, and a tile whose inputs are all uniform is
//! blended once and written as a `Uniform` tile, so a whole-canvas bucket on
//! an empty layer costs tile metadata, not pixels.

use crate::fill::{fill_planes, Pass};
use crate::stroke::blend_pixel;
use crate::Rgba;
use pictura_core::{Layer, PsdRect, TileState};

/// What the uniform pass decided about one tile.
enum Decision {
    /// The fill leaves the tile as it is.
    Skip,
    /// Every pixel becomes these R, G, B and alpha samples.
    Write([u8; 4]),
    /// The tile needs the per-pixel pass.
    Dense,
}

/// The plane slot of a channel id: R, G, B, then transparency.
pub(crate) fn slot(id: i16) -> Option<usize> {
    match id {
        0 => Some(0),
        1 => Some(1),
        2 => Some(2),
        -1 => Some(3),
        _ => None,
    }
}

/// Fill the layer's 8-bit planes tile by tile inside `pass.within`. `None`
/// when a colour plane is missing or a plane cannot be tiled, so the caller
/// fills the whole planes instead; otherwise the rectangle changed.
pub(crate) fn fill_tiled_layer(
    layer: &mut Layer,
    n: usize,
    pass: &Pass<'_>,
    source: &(impl Fn(i32, i32, u8) -> Option<(Rgba, f32)> + Sync),
) -> Option<Option<PsdRect>> {
    let rect = layer.rect;
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut planes: [Option<usize>; 4] = [None; 4];
    for (ci, c) in layer.channels.iter().enumerate() {
        let Some(k) = slot(c.id) else {
            continue;
        };
        if c.data.len() == n && planes[k].is_none() {
            planes[k] = Some(ci);
        }
    }
    if pass.gray {
        planes[1] = None;
        planes[2] = None;
    } else if planes[1].is_none() || planes[2].is_none() {
        return None;
    }
    let first = planes[0]?;
    for &ci in planes.iter().flatten() {
        if !layer.channels[ci].data.tile(w, h) {
            return None;
        }
    }
    let region = pass.within;
    let xs = (region.left - rect.left) as usize..(region.right - rect.left) as usize;
    let ys = (region.top - rect.top) as usize..(region.bottom - rect.top) as usize;
    let tiles = layer.channels[first].data.tiles_in(xs, ys);
    let mut changed: Option<PsdRect> = None;
    let mut dense = Vec::new();
    for t in tiles {
        let (lx, ly, tw, th) = layer.channels[first].data.tile_rect(t)?;
        let at = PsdRect {
            top: rect.top + ly as i32,
            left: rect.left + lx as i32,
            bottom: rect.top + (ly + th) as i32,
            right: rect.left + (lx + tw) as i32,
        };
        match decide(layer, &planes, t, at, pass, source) {
            Decision::Skip => {}
            Decision::Dense => dense.push(t),
            Decision::Write(values) => {
                for (k, ci) in planes.iter().enumerate() {
                    if let Some(ci) = *ci {
                        layer.channels[ci].data.set_tile_uniform(t, values[k]);
                    }
                }
                changed = Some(changed.map_or(at, |c| c.union(at)));
            }
        }
    }
    if let Some(dirty) = fill_dense(layer, &planes, &dense, pass, source) {
        changed = Some(changed.map_or(dirty, |c| c.union(dirty)));
    }
    for &ci in planes.iter().flatten() {
        layer.channels[ci].data.settle();
    }
    Some(changed)
}

/// Whether tile `t` (document rectangle `at`) can be filled as one value.
fn decide(
    layer: &Layer,
    planes: &[Option<usize>; 4],
    t: usize,
    at: PsdRect,
    pass: &Pass<'_>,
    source: &(impl Fn(i32, i32, u8) -> Option<(Rgba, f32)> + Sync),
) -> Decision {
    let inside = at.left >= pass.within.left
        && at.top >= pass.within.top
        && at.right <= pass.within.right
        && at.bottom <= pass.within.bottom;
    if !pass.through.solid || pass.mode == crate::PaintMode::Dissolve || !inside {
        return Decision::Dense;
    }
    let mut dst = [0u8, 0, 0, 255];
    for (k, ci) in planes.iter().enumerate() {
        let Some(ci) = *ci else {
            continue;
        };
        match layer.channels[ci].data.tile_state(t) {
            Some(TileState::Empty) => dst[k] = 0,
            Some(TileState::Uniform(v)) => dst[k] = v,
            _ => return Decision::Dense,
        }
    }
    let (xs, ys) = (
        at.left as usize..at.right as usize,
        at.top as usize..at.bottom as usize,
    );
    let over = |mask: Option<&pictura_core::mask_ops::TiledMask>| {
        mask.map_or(Some(255), |m| m.uniform_over(xs.clone(), ys.clone()))
    };
    let (Some(selected), Some(m)) = (over(pass.through.selection), over(pass.through.mask)) else {
        return Decision::Dense;
    };
    if selected == 0 || m == 0 {
        return Decision::Skip;
    }
    let Some(answer) = source(at.left, at.top, m) else {
        return Decision::Skip;
    };
    let dst = if pass.gray {
        (dst[0], dst[0], dst[0], dst[3])
    } else {
        (dst[0], dst[1], dst[2], dst[3])
    };
    let has_alpha = planes[3].is_some();
    match pass.blend_one(
        (at.left, at.top),
        dst,
        has_alpha,
        selected,
        answer,
        blend_pixel,
        &mut None,
    ) {
        Some(out) => Decision::Write([out.0, out.1, out.2, out.3]),
        None => Decision::Skip,
    }
}

/// The per-pixel pass over the tiles at `dense` (ascending), each forked for
/// writing; the rectangle changed.
fn fill_dense(
    layer: &mut Layer,
    planes: &[Option<usize>; 4],
    dense: &[usize],
    pass: &Pass<'_>,
    source: &(impl Fn(i32, i32, u8) -> Option<(Rgba, f32)> + Sync),
) -> Option<PsdRect> {
    if dense.is_empty() {
        return None;
    }
    let rect = layer.rect;
    let mut tiles: [Option<Vec<pictura_core::TileMut<'_, u8>>>; 4] = [None, None, None, None];
    for (ci, c) in layer.channels.iter_mut().enumerate() {
        if let Some(k) = planes.iter().position(|p| *p == Some(ci)) {
            tiles[k] = Some(c.data.tiles_mut_at(dense));
        }
    }
    let mut iters = tiles.map(|p| p.map(Vec::into_iter));
    let mut changed: Option<PsdRect> = None;
    for _ in dense {
        let next = iters
            .each_mut()
            .map(|it| it.as_mut().and_then(Iterator::next));
        let Some((x, y, w, h)) = next.iter().flatten().next().map(|t| (t.x, t.y, t.w, t.h)) else {
            continue;
        };
        let [r, g, b, a] = next.map(|t| t.map(|t| t.data));
        let Some(r) = r else {
            continue;
        };
        let (left, top) = (rect.left + x as i32, rect.top + y as i32);
        let tile_rect = PsdRect {
            top,
            left,
            bottom: top + h as i32,
            right: left + w as i32,
        };
        if let Some(dirty) = fill_planes(r, g, b, a, tile_rect, pass, source, blend_pixel) {
            changed = Some(changed.map_or(dirty, |c| c.union(dirty)));
        }
    }
    changed
}
