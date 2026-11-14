//! The Magic Wand, Grow and Similar over tiles (#1214): the composite is read
//! one tile at a time as the flood reaches it, and the selections they build
//! are tiled masks.

use pictura_core::mask_ops::{
    coverage_bounds, empty_mask, flood_tiled, match_mask, FloodSource, Grid, MatchTiles, Seeds,
    TileMatch, TiledMask,
};
use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::source::{ImageSource, PixelReader};
use crate::tiled::{tiled_view, zip_tiles};
use crate::{chebyshev, matcher, SelectError, Selection};

type Rect = (usize, usize, usize, usize);

fn check_dims(sel: &Selection, img: ImageSource<'_>) -> Result<(), SelectError> {
    if sel.width != img.width() || sel.height != img.height() {
        return Err(SelectError::SizeMismatch(format!(
            "{}x{} vs image {}x{}",
            sel.width,
            sel.height,
            img.width(),
            img.height()
        )));
    }
    Ok(())
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
    img: ImageSource<'_>,
    reference: [u8; 3],
    tolerance: u8,
) -> impl Fn(usize, Rect) -> TileMatch + Sync + '_ {
    move |_, rect| {
        if let Some(c) = img.uniform_colour(rect) {
            return if chebyshev(c, reference) <= tolerance {
                TileMatch::All
            } else {
                TileMatch::None
            };
        }
        let planes = img.tile_planes(rect);
        let tw = rect.2;
        TileMatch::from_rows(tw, rect.3, |dy, out| {
            for (dx, o) in out.iter_mut().enumerate() {
                *o = chebyshev(pixel(&planes, dy * tw + dx), reference) <= tolerance;
            }
        })
    }
}

/// Flood/global selection by color tolerance (Magic Wand). With
/// `anti_alias`, unselected boundary pixels get the inferred colour-distance
/// fringe described by [`soften_boundary`].
pub fn magic_wand(
    img: &PixelBuffer,
    x: u32,
    y: u32,
    tolerance: u8,
    contiguous: bool,
    anti_alias: bool,
) -> Result<Selection, SelectError> {
    magic_wand_from(ImageSource::Buffer(img), x, y, tolerance, contiguous)
}

/// [`magic_wand`] over any [`ImageSource`].
pub fn magic_wand_from(
    img: ImageSource<'_>,
    x: u32,
    y: u32,
    tolerance: u8,
    contiguous: bool,
) -> Result<Selection, SelectError> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 || x as usize >= w || y as usize >= h {
        return Err(SelectError::InvalidParams(format!(
            "point ({x},{y}) outside {w}x{h}"
        )));
    }
    let reference = PixelReader::new(img).rgb(x as usize, y as usize);
    let eval = wand_tile(img, reference, tolerance);
    let mut data = if contiguous {
        let mut source = MatchTiles::new(w, h, eval);
        flood_tiled(w, h, Seeds::Pixel(x as usize, y as usize), &mut source)
            .unwrap_or_else(|| empty_mask(w, h))
    } else {
        match_mask(w, h, eval)
    };
    if anti_alias && tolerance > 0 {
        soften_boundary(&mut data, img, reference, tolerance, w, h);
    }
    Ok(Selection {
        width: img.width(),
        height: img.height(),
        data,
    })
}

/// The inferred anti-alias fringe on the binary wand result.
///
/// Every unselected pixel four-adjacent to a selected one whose Chebyshev
/// distance `d` from the seed satisfies `tolerance < d < tolerance + width`
/// gets coverage `clamp(1 - (d - tolerance) / width, 0, 1)`, where
/// `width = max(0.5 * tolerance, 1.5)`. The `1.5` floor keeps Tolerance 1 and
/// 2 able to produce a fringe. Pixels already selected keep 255.
///
/// ponytail: the GIMP-style 1.5× window is inferred; Adobe's exact ramp is
/// unpublished. The pass reads the binary mask, so it does not chain fringes.
fn soften_boundary(
    data: &mut TiledMask,
    img: &PixelBuffer,
    reference: [u8; 3],
    tolerance: u8,
    w: usize,
    h: usize,
) {
    let Some((x0, y0, x1, y1)) = coverage_bounds(data, w, h) else {
        return;
    };
    let top = y0.saturating_sub(1);
    let bottom = (y1 + 1).min(h);
    let left = x0.saturating_sub(1);
    let right = (x1 + 1).min(w);
    let t = f64::from(tolerance);
    let width = (0.5 * t).max(1.5);
    let mut rows: Vec<(usize, Vec<(usize, u8)>)> = Vec::new();
    let mut previous = vec![0u8; w];
    let mut current = vec![0u8; w];
    let mut next = vec![0u8; w];
    for y in top..bottom {
        if y > 0 {
            let _ = data.read_row_into(y - 1, w, &mut previous);
        } else {
            previous.fill(0);
        }
        let _ = data.read_row_into(y, w, &mut current);
        if y + 1 < h {
            let _ = data.read_row_into(y + 1, w, &mut next);
        } else {
            next.fill(0);
        }
        let mut changes = Vec::new();
        for x in left..right {
            if current[x] != 0 {
                continue;
            }
            let near = (x > 0 && current[x - 1] != 0)
                || (x + 1 < w && current[x + 1] != 0)
                || previous[x] != 0
                || next[x] != 0;
            if !near {
                continue;
            }
            let d = f64::from(chebyshev(rgb_at(img, y * w + x), reference));
            if d <= t || d >= t + width {
                continue;
            }
            let coverage = ((1.0 - (d - t) / width).clamp(0.0, 1.0) * 255.0).round() as u8;
            if coverage > 0 {
                changes.push((x, coverage));
            }
        }
        if !changes.is_empty() {
            rows.push((y, changes));
        }
    }
    for (y, changes) in rows {
        let _ = data.read_row_into(y, w, &mut current);
        for (x, value) in changes {
            current[x] = value;
        }
        let _ = data.write_row_from(y, w, &current);
    }
    data.settle();
}

/// Grow's flood: a neighbour joins when its colour is within the tolerance
/// of the pixel it is reached from, and a tile of one colour joins whole.
struct Similar<'a> {
    img: ImageSource<'a>,
    reader: PixelReader<'a>,
    grid: Grid,
    tolerance: u8,
}

impl FloodSource for Similar<'_> {
    fn matches(&mut self, (fx, fy): (usize, usize), (tx, ty): (usize, usize)) -> bool {
        let to = self.reader.rgb(tx, ty);
        chebyshev(to, self.reader.rgb(fx, fy)) <= self.tolerance
    }

    fn whole(&mut self, tile: usize) -> bool {
        self.img.uniform_colour(self.grid.rect(tile)).is_some()
    }
}

/// Expand a selection to include adjacent similar-colored pixels.
pub fn grow(sel: &Selection, img: &PixelBuffer, tolerance: u8) -> Result<Selection, SelectError> {
    grow_from(sel, ImageSource::Buffer(img), tolerance)
}

/// [`grow`] over any [`ImageSource`].
pub fn grow_from(
    sel: &Selection,
    img: ImageSource<'_>,
    tolerance: u8,
) -> Result<Selection, SelectError> {
    check_dims(sel, img)?;
    let (w, h) = (sel.width as usize, sel.height as usize);
    if w == 0 || h == 0 {
        return Ok(sel.clone());
    }
    let mut source = Similar {
        img,
        reader: PixelReader::new(img),
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
    similar_from(sel, ImageSource::Buffer(img), tolerance)
}

/// [`similar`] over any [`ImageSource`].
pub fn similar_from(
    sel: &Selection,
    img: ImageSource<'_>,
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
        if let Some(c) = img.uniform_colour(rect) {
            return vec![c];
        }
        let planes = img.tile_planes(rect);
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
        match (s, img.uniform_colour(grid.rect(t))) {
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
                let planes = img.tile_planes(rect);
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
    use crate::rgb_at;
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

#[cfg(test)]
mod aa_tests {
    use super::*;

    fn gradient(w: u32, h: u32, value: impl Fn(u32) -> u8) -> PixelBuffer {
        let n = w as usize * h as usize;
        let mut img = PixelBuffer::new(w, h, 3);
        for y in 0..h {
            for x in 0..w {
                let p = (y * w + x) as usize;
                let v = value(x);
                img.data[p] = v;
                img.data[n + p] = 0;
                img.data[2 * n + p] = 0;
            }
        }
        img
    }

    fn partial(selection: &Selection) -> bool {
        selection.data.as_slice().iter().any(|&v| v > 0 && v < 255)
    }

    fn binary(selection: &Selection) -> bool {
        selection
            .data
            .as_slice()
            .iter()
            .all(|&v| v == 0 || v == 255)
    }

    #[test]
    fn wand_aa_grades_boundary() {
        let img = gradient(16, 4, |x| (x * 12) as u8);
        let selection = magic_wand(&img, 0, 2, 10, true, true).expect("wand");
        assert_eq!(selection.data[2 * 16], 255);
        let fringe = selection.data[2 * 16 + 1];
        assert!(fringe > 0 && fringe < 255, "fringe {fringe}");
        assert_eq!(selection.data[2 * 16 + 2], 0);
    }

    #[test]
    fn wand_aa_off_is_binary() {
        let img = gradient(16, 4, |x| (x * 12) as u8);
        let selection = magic_wand(&img, 0, 2, 10, true, false).expect("wand");
        assert!(binary(&selection));
    }

    #[test]
    fn wand_aa_zero_tolerance_is_binary() {
        let img = gradient(16, 4, |x| x as u8);
        let selection = magic_wand(&img, 0, 2, 0, true, true).expect("wand");
        assert!(binary(&selection));
    }

    #[test]
    fn wand_aa_low_tolerance_fringes() {
        let img = gradient(8, 2, |x| x as u8);
        let selection = magic_wand(&img, 0, 1, 1, true, true).expect("wand");
        assert_eq!(selection.data[8], 255);
        let fringe = selection.data[10];
        assert!(fringe > 0 && fringe < 255, "fringe {fringe}");
        assert_eq!(selection.data[11], 0);
    }

    #[test]
    fn wand_aa_global_fringes_each_region() {
        let img = gradient(32, 4, |x| {
            if x < 8 {
                (x * 4) as u8
            } else if (16..24).contains(&x) {
                ((x - 16) * 4) as u8
            } else {
                255
            }
        });
        let selection = magic_wand(&img, 0, 2, 10, false, true).expect("wand");
        assert!(partial(&selection));
        assert!(selection.data[2 * 32 + 3] > 0 && selection.data[2 * 32 + 3] < 255);
        assert!(selection.data[2 * 32 + 19] > 0 && selection.data[2 * 32 + 19] < 255);
        assert_eq!(selection.data[2 * 32 + 12], 0);
    }
}
