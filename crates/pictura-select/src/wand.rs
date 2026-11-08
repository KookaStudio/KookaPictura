//! The Magic Wand, Grow and Similar over tiles (#1214): the composite is read
//! one tile at a time as the flood reaches it, and the selections they build
//! are tiled masks.

use pictura_core::mask_ops::{
    coverage_bounds, empty_mask, flood_tiled, match_mask, FloodSource, Grid, MatchTiles, Seeds,
    TileMatch,
};
use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::tiled::{channel_tile, channel_uniform, tiled_view, zip_tiles};
use crate::{chebyshev, matcher, rgb_at, SelectError, Selection};

type Rect = (usize, usize, usize, usize);

fn check_dims(sel: &Selection, img: &PixelBuffer) -> Result<(), SelectError> {
    if sel.width != img.width || sel.height != img.height {
        return Err(SelectError::SizeMismatch(format!(
            "{}x{} vs image {}x{}",
            sel.width, sel.height, img.width, img.height
        )));
    }
    Ok(())
}

/// The colour planes the wand compares: three for RGB, the one grey plane
/// otherwise.
fn colour_planes(img: &PixelBuffer) -> usize {
    if img.channels >= 3 {
        3
    } else {
        1
    }
}

fn colour(planes: &[u8]) -> [u8; 3] {
    match planes.len() {
        3 => [planes[0], planes[1], planes[2]],
        _ => [planes[0]; 3],
    }
}

/// The composite's colour over a tile, when its tiles hold one value.
fn uniform_colour(img: &PixelBuffer, rect: Rect) -> Option<[u8; 3]> {
    let h = img.height as usize;
    let values: Option<Vec<u8>> = (0..colour_planes(img))
        .map(|k| channel_uniform(&img.data, h, k, rect))
        .collect();
    values.map(|v| colour(&v))
}

/// The composite's colour planes over a tile, each row-major.
fn tile_planes(img: &PixelBuffer, rect: Rect) -> Vec<Vec<u8>> {
    let (w, h) = (img.width as usize, img.height as usize);
    (0..colour_planes(img))
        .map(|k| channel_tile(&img.data, w, h, k, rect))
        .collect()
}

fn pixel(planes: &[Vec<u8>], i: usize) -> [u8; 3] {
    match planes.len() {
        3 => [planes[0][i], planes[1][i], planes[2][i]],
        _ => [planes[0][i]; 3],
    }
}

/// The wand's answers for one tile: the pixels within `tolerance` of
/// `reference`.
fn wand_tile(
    img: &PixelBuffer,
    reference: [u8; 3],
    tolerance: u8,
) -> impl Fn(usize, Rect) -> TileMatch + Sync + '_ {
    move |_, rect| {
        if let Some(c) = uniform_colour(img, rect) {
            return if chebyshev(c, reference) <= tolerance {
                TileMatch::All
            } else {
                TileMatch::None
            };
        }
        let planes = tile_planes(img, rect);
        let tw = rect.2;
        TileMatch::from_rows(tw, rect.3, |dy, out| {
            for (dx, o) in out.iter_mut().enumerate() {
                *o = chebyshev(pixel(&planes, dy * tw + dx), reference) <= tolerance;
            }
        })
    }
}

/// Flood/global selection by color tolerance (Magic Wand).
pub fn magic_wand(
    img: &PixelBuffer,
    x: u32,
    y: u32,
    tolerance: u8,
    contiguous: bool,
) -> Result<Selection, SelectError> {
    let (w, h) = (img.width as usize, img.height as usize);
    if w == 0 || h == 0 || x as usize >= w || y as usize >= h {
        return Err(SelectError::InvalidParams(format!(
            "point ({x},{y}) outside {w}x{h}"
        )));
    }
    let reference = rgb_at(img, y as usize * w + x as usize);
    let eval = wand_tile(img, reference, tolerance);
    let data = if contiguous {
        let mut source = MatchTiles::new(w, h, eval);
        flood_tiled(w, h, Seeds::Pixel(x as usize, y as usize), &mut source)
            .unwrap_or_else(|| empty_mask(w, h))
    } else {
        match_mask(w, h, eval)
    };
    Ok(Selection {
        width: img.width,
        height: img.height,
        data,
    })
}

/// Grow's flood: a neighbour joins when its colour is within the tolerance
/// of the pixel it is reached from, and a tile of one colour joins whole.
struct Similar<'a> {
    img: &'a PixelBuffer,
    grid: Grid,
    tolerance: u8,
}

impl FloodSource for Similar<'_> {
    fn matches(&mut self, (fx, fy): (usize, usize), (tx, ty): (usize, usize)) -> bool {
        let w = self.grid.w;
        chebyshev(rgb_at(self.img, ty * w + tx), rgb_at(self.img, fy * w + fx)) <= self.tolerance
    }

    fn whole(&mut self, tile: usize) -> bool {
        uniform_colour(self.img, self.grid.rect(tile)).is_some()
    }
}

/// Expand a selection to include adjacent similar-colored pixels.
pub fn grow(sel: &Selection, img: &PixelBuffer, tolerance: u8) -> Result<Selection, SelectError> {
    check_dims(sel, img)?;
    let (w, h) = (sel.width as usize, sel.height as usize);
    if w == 0 || h == 0 {
        return Ok(sel.clone());
    }
    let mut source = Similar {
        img,
        grid: Grid::new(w, h),
        tolerance,
    };
    let Some(reached) = flood_tiled(w, h, Seeds::Mask(&sel.data), &mut source) else {
        return Ok(sel.clone());
    };
    let data = zip_tiles(&sel.data, &reached, w, h, |s, r| {
        if r != 0 && s == 0 {
            255
        } else {
            s
        }
    });
    Ok(Selection {
        width: sel.width,
        height: sel.height,
        data,
    })
}

/// Add all similar-colored pixels in the image to the selection.
pub fn similar(
    sel: &Selection,
    img: &PixelBuffer,
    tolerance: u8,
) -> Result<Selection, SelectError> {
    check_dims(sel, img)?;
    let (w, h) = (sel.width as usize, sel.height as usize);
    if coverage_bounds(&sel.data, w, h).is_none() {
        return Ok(sel.clone());
    }
    let mask = tiled_view(&sel.data, w, h);
    let grid = Grid::new(w, h);
    // The reference colours, one tile at a time: a selected tile of one
    // colour contributes that colour once.
    let references = (0..grid.count()).flat_map(|t| {
        let rect = grid.rect(t);
        let tile = mask.tile_at(t).expect("a tiled view");
        if tile.uniform() == Some(0) {
            return Vec::new();
        }
        if let Some(c) = uniform_colour(img, rect) {
            return vec![c];
        }
        let planes = tile_planes(img, rect);
        let tw = rect.2;
        (0..rect.3)
            .flat_map(|dy| (0..tw).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| tile.get(dx, dy) > 0)
            .map(|(dx, dy)| pixel(&planes, dy * tw + dx))
            .collect()
    });
    let matcher = matcher::ColorMatcher::new(references, tolerance);
    let mut out = empty_mask(w, h);
    let mut dense = Vec::new();
    for t in 0..grid.count() {
        let s = mask.tile_at(t).and_then(|tile| tile.uniform());
        match (s, uniform_colour(img, grid.rect(t))) {
            (Some(v), _) if v > 0 => {
                out.set_tile_uniform(t, v);
            }
            (Some(_), Some(c)) => {
                out.set_tile_uniform(t, if matcher.contains(c) { 255 } else { 0 });
            }
            _ => dense.push(t),
        }
    }
    if !dense.is_empty() {
        out.tiles_mut_at(&dense)
            .into_par_iter()
            .zip(dense.par_iter())
            .for_each(|(mut tile, &t)| {
                let rect = grid.rect(t);
                let sel_tile = mask.tile_at(t).expect("a tiled view");
                let planes = tile_planes(img, rect);
                let tw = rect.2;
                for dy in 0..tile.h {
                    for (dx, px) in tile.row_mut(dy).iter_mut().enumerate() {
                        let s = sel_tile.get(dx, dy);
                        *px = if s == 0 && matcher.contains(pixel(&planes, dy * tw + dx)) {
                            255
                        } else {
                            s
                        };
                    }
                }
            });
        out.settle();
    }
    Ok(Selection {
        width: sel.width,
        height: sel.height,
        data: out,
    })
}

/// The dense implementations the tiled ones replace, kept as their oracle.
#[cfg(test)]
pub(crate) mod dense {
    use super::*;
    use pictura_core::mask_ops;

    pub(crate) fn magic_wand(
        img: &PixelBuffer,
        x: u32,
        y: u32,
        tolerance: u8,
        contiguous: bool,
    ) -> Vec<u8> {
        let (w, h) = (img.width as usize, img.height as usize);
        let reference = rgb_at(img, y as usize * w + x as usize);
        if contiguous {
            let mut seeds = vec![0u8; w * h];
            seeds[y as usize * w + x as usize] = 1;
            mask_ops::flood(w, h, &seeds, |_, q| {
                chebyshev(rgb_at(img, q), reference) <= tolerance
            })
            .unwrap_or_else(|| vec![0u8; w * h])
        } else {
            (0..w * h)
                .map(|p| u8::from(chebyshev(rgb_at(img, p), reference) <= tolerance) * 255)
                .collect()
        }
    }

    pub(crate) fn grow(sel: &[u8], img: &PixelBuffer, tolerance: u8) -> Vec<u8> {
        let (w, h) = (img.width as usize, img.height as usize);
        let mut data = sel.to_vec();
        let Some(reached) = mask_ops::flood(w, h, sel, |from, to| {
            chebyshev(rgb_at(img, to), rgb_at(img, from)) <= tolerance
        }) else {
            return data;
        };
        for (p, &value) in reached.iter().enumerate() {
            if value != 0 && data[p] == 0 {
                data[p] = 255;
            }
        }
        data
    }

    pub(crate) fn similar(sel: &[u8], img: &PixelBuffer, tolerance: u8) -> Vec<u8> {
        let mut data = sel.to_vec();
        if !sel.iter().any(|&v| v > 0) {
            return data;
        }
        let matcher = matcher::ColorMatcher::new(
            (0..sel.len())
                .filter(|&p| sel[p] > 0)
                .map(|p| rgb_at(img, p)),
            tolerance,
        );
        for (p, slot) in data.iter_mut().enumerate() {
            if *slot == 0 && matcher.contains(rgb_at(img, p)) {
                *slot = 255;
            }
        }
        data
    }
}
