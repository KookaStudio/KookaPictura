//! Document-coordinate editing coverage, independent of mask metadata.

use crate::{LayerMask, Plane, PsdRect};

/// A read-only coverage snapshot. Plane clones share sample storage. `None`
/// at an operation's optional constraint means unrestricted editing; a value
/// whose samples are zero is an explicit constraint that protects every pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    rect: PsdRect,
    data: Option<Plane<u8>>,
    outside: u8,
}

impl Coverage {
    /// Check the plane's dimensions without reading or materializing samples.
    pub fn new(rect: PsdRect, data: Plane<u8>, outside: u8) -> Option<Self> {
        let w = usize::try_from(i64::from(rect.right) - i64::from(rect.left)).ok()?;
        let h = usize::try_from(i64::from(rect.bottom) - i64::from(rect.top)).ok()?;
        if data.len() != w.checked_mul(h)?
            || data.geometry().is_some_and(|shape| shape != (w, h, 1))
        {
            return None;
        }
        Some(Self {
            rect,
            data: Some(data),
            outside,
        })
    }

    /// Share a document-sized plane with a zero exterior.
    pub fn for_canvas(data: &Plane<u8>, width: u32, height: u32) -> Option<Self> {
        Self::new(
            PsdRect {
                left: 0,
                top: 0,
                right: i32::try_from(width).ok()?,
                bottom: i32::try_from(height).ok()?,
            },
            data.clone(),
            0,
        )
    }

    /// Constant coverage, including outside any finite operation region.
    pub fn constant(value: u8) -> Self {
        Self {
            rect: PsdRect::default(),
            data: None,
            outside: value,
        }
    }

    pub fn rect(&self) -> PsdRect {
        self.rect
    }
    pub fn outside(&self) -> u8 {
        self.outside
    }

    pub fn sample(&self, x: i32, y: i32) -> u8 {
        if x < self.rect.left || x >= self.rect.right || y < self.rect.top || y >= self.rect.bottom
        {
            return self.outside;
        }
        let w = (i64::from(self.rect.right) - i64::from(self.rect.left)) as usize;
        let index = (i64::from(y) - i64::from(self.rect.top)) as usize * w
            + (i64::from(x) - i64::from(self.rect.left)) as usize;
        self.data
            .as_ref()
            .and_then(|p| p.sample(index))
            .unwrap_or(self.outside)
    }

    /// Initialize the whole output, including the portion outside stored bounds.
    pub fn read_row(&self, left: i32, y: i32, out: &mut [u8]) {
        out.fill(self.outside);
        let Some(data) = &self.data else { return };
        if y < self.rect.top || y >= self.rect.bottom {
            return;
        }
        let x0 = i64::from(left).max(i64::from(self.rect.left));
        let x1 = (i64::from(left) + out.len() as i64).min(i64::from(self.rect.right));
        if x1 <= x0 {
            return;
        }
        let w = (i64::from(self.rect.right) - i64::from(self.rect.left)) as usize;
        let row = (i64::from(y) - i64::from(self.rect.top)) as usize;
        let start = (x0 - i64::from(left)) as usize;
        let cols = (x1 - x0) as usize;
        let local = (x0 - i64::from(self.rect.left)) as usize;
        data.read_rect_into(
            local..local + cols,
            row..row + 1,
            w,
            &mut out[start..start + cols],
        );
    }

    /// The samples of one document-space span, without a contiguous source mirror.
    pub fn row(&self, left: i32, y: i32, len: usize) -> Vec<u8> {
        let mut out = vec![self.outside; len];
        self.read_row(left, y, &mut out);
        out
    }

    /// One value over the entire region, including its exterior, or mixed.
    pub fn uniform(&self, region: PsdRect) -> Option<u8> {
        if region.is_empty() {
            return None;
        }
        let Some(data) = &self.data else {
            return Some(self.outside);
        };
        let Some(inside) = region.intersect(self.rect) else {
            return Some(self.outside);
        };
        let xs = (i64::from(inside.left) - i64::from(self.rect.left)) as usize
            ..(i64::from(inside.right) - i64::from(self.rect.left)) as usize;
        let ys = (i64::from(inside.top) - i64::from(self.rect.top)) as usize
            ..(i64::from(inside.bottom) - i64::from(self.rect.top)) as usize;
        let value = if let Some(value) = data.uniform_over(xs, ys) {
            value
        } else {
            let first = self.sample(inside.left, inside.top);
            let mut row = vec![0; inside.width() as usize];
            for y in inside.top..inside.bottom {
                self.read_row(inside.left, y, &mut row);
                if row.iter().any(|v| *v != first) {
                    return None;
                }
            }
            first
        };
        (inside == region || value == self.outside).then_some(value)
    }

    /// Uniform coverage over a positive document-grid region.
    pub fn uniform_over(
        &self,
        xs: std::ops::Range<usize>,
        ys: std::ops::Range<usize>,
    ) -> Option<u8> {
        self.uniform(PsdRect {
            left: i32::try_from(xs.start).ok()?,
            right: i32::try_from(xs.end).ok()?,
            top: i32::try_from(ys.start).ok()?,
            bottom: i32::try_from(ys.end).ok()?,
        })
    }

    /// Initialize a rectangle-sized output by rows, without a source mirror.
    pub fn read_rect(&self, rect: PsdRect, out: &mut [u8]) {
        let w = rect.width().max(0) as usize;
        if w == 0 {
            return;
        }
        for (y, row) in out
            .chunks_mut(w)
            .take(rect.height().max(0) as usize)
            .enumerate()
        {
            self.read_row(rect.left, rect.top + y as i32, row);
        }
    }

    /// Exact nonzero bounds within a finite operation region. Constant tile
    /// regions are decided without pinning their samples.
    pub fn covered_bounds(&self, within: PsdRect) -> Option<PsdRect> {
        if within.is_empty() {
            return None;
        }
        let region = if self.outside == 0 {
            within.intersect(self.rect)?
        } else {
            within
        };
        let mut found: Option<PsdRect> = None;
        let mut y = region.top;
        while y < region.bottom {
            let bottom = (i64::from(y) + 256).min(i64::from(region.bottom)) as i32;
            let mut x = region.left;
            while x < region.right {
                let right = (i64::from(x) + 256).min(i64::from(region.right)) as i32;
                let at = PsdRect {
                    left: x,
                    top: y,
                    right,
                    bottom,
                };
                let part = match self.uniform(at) {
                    Some(0) => None,
                    Some(_) => Some(at),
                    None => {
                        let mut bounds: Option<PsdRect> = None;
                        for row_y in y..bottom {
                            let row = self.row(x, row_y, at.width() as usize);
                            if let (Some(first), Some(last)) = (
                                row.iter().position(|v| *v != 0),
                                row.iter().rposition(|v| *v != 0),
                            ) {
                                let r = PsdRect {
                                    left: x + first as i32,
                                    right: x + last as i32 + 1,
                                    top: row_y,
                                    bottom: row_y + 1,
                                };
                                bounds = Some(bounds.map_or(r, |b| b.union(r)));
                            }
                        }
                        bounds
                    }
                };
                if let Some(part) = part {
                    found = Some(found.map_or(part, |b| b.union(part)));
                }
                x = right;
            }
            y = bottom;
        }
        found
    }

    /// Translate document coordinates into a local space whose origin is `(x,y)`.
    pub fn rebased(&self, x: i32, y: i32) -> Option<Self> {
        if self.data.is_none() && self.rect.is_empty() {
            return Some(self.clone());
        }
        let shift = |v: i32, origin: i32| i32::try_from(i64::from(v) - i64::from(origin)).ok();
        Some(Self {
            rect: PsdRect {
                left: shift(self.rect.left, x)?,
                right: shift(self.rect.right, x)?,
                top: shift(self.rect.top, y)?,
                bottom: shift(self.rect.bottom, y)?,
            },
            data: self.data.clone(),
            outside: self.outside,
        })
    }

    /// A new output plane over `rect`, for a persistent mask or another output.
    /// The source is never materialized; constant output tiles remain sparse.
    pub fn plane_within(&self, rect: PsdRect) -> Option<Plane<u8>> {
        let w = usize::try_from(i64::from(rect.right) - i64::from(rect.left)).ok()?;
        let h = usize::try_from(i64::from(rect.bottom) - i64::from(rect.top)).ok()?;
        if let Some(data) = &self.data {
            if rect == self.rect {
                return Some(data.clone());
            }
        }
        w.checked_mul(h)?;
        let mut out = Plane::filled(w, h, 1, self.outside);
        if self.data.is_none() {
            return Some(out);
        }
        for tile in 0..out.tile_count() {
            let (x, y, tw, th) = out.tile_rect(tile)?;
            let left = (i64::from(rect.left) + x as i64) as i32;
            let top = (i64::from(rect.top) + y as i64) as i32;
            let bounds = PsdRect {
                left,
                top,
                right: (i64::from(left) + tw as i64) as i32,
                bottom: (i64::from(top) + th as i64) as i32,
            };
            if let Some(value) = self.uniform(bounds) {
                out.set_tile_uniform(tile, value);
                continue;
            }
            let mut samples = vec![self.outside; tw * th];
            self.read_rect(bounds, &mut samples);
            out.write_rect_from(x..x + tw, y..y + th, w, &samples);
        }
        out.settle();
        Some(out)
    }
}

impl crate::Channel {
    /// Interpret an alpha channel on its document grid, refusing a size mismatch.
    pub fn coverage(&self, width: u32, height: u32) -> Option<Coverage> {
        Coverage::for_canvas(&self.data, width, height)
    }
}

impl LayerMask {
    /// Stored raw coverage only. Enabled state, density, feather and native
    /// samples remain the responsibility of the real-mask operation.
    pub fn raw_coverage(&self) -> Option<Coverage> {
        match &self.data {
            Some(data) => Coverage::new(self.rect, data.clone(), self.default_color),
            None if self.rect.left <= self.rect.right && self.rect.top <= self.rect.bottom => {
                Some(Coverage {
                    rect: self.rect,
                    data: None,
                    outside: self.default_color,
                })
            }
            None => None,
        }
    }

    /// Resolve the legacy destructive-edit constraint policy at a real mask.
    pub fn edit_coverage(&self) -> Option<Coverage> {
        if self.disabled {
            Some(Coverage::constant(255))
        } else if self.data.is_none() {
            Some(Coverage::constant(self.default_color))
        } else {
            self.raw_coverage()
        }
    }
}

impl Default for LayerMask {
    fn default() -> Self {
        Self {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            default_color: 0,
            disabled: false,
            flags: 0,
            data: None,
            extra: Plane::default(),
            density: 255,
            feather_bits: 0,
        }
    }
}

#[cfg(test)]
mod tests;
