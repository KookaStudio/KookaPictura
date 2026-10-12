//! The Liquify dialog's preview: the layer through the mesh, rendered at the
//! screen's resolution for the view the dialog shows. Magnified, each screen
//! pixel samples the full-resolution layer with the commit's Catmull-Rom mesh
//! and source, so a zoomed-in preview is smooth and matches OK; minified, it
//! reads a box-reduced level so it does not alias.

use pictura_core::Plane;
use rayon::prelude::*;

use super::{catmull_rom, LiquifyMesh};

/// A render this many pixels or larger runs on the pool; a dab's patch is
/// faster on the calling thread.
const PARALLEL_PIXELS: usize = 60_000;

/// A box-halved, premultiplied RGBA8 level.
#[derive(Debug, Clone)]
struct Level {
    width: usize,
    height: usize,
    px: Vec<[u8; 4]>,
}

impl Level {
    fn bilinear(&self, u: f64, v: f64) -> [f32; 4] {
        let (iu, iv) = (u.floor(), v.floor());
        let (fu, fv) = ((u - iu) as f32, (v - iv) as f32);
        let at = |x: f64, y: f64| {
            let x = x.clamp(0.0, (self.width - 1) as f64) as usize;
            let y = y.clamp(0.0, (self.height - 1) as f64) as usize;
            self.px[y * self.width + x]
        };
        let (a, b, c, e) = (
            at(iu, iv),
            at(iu + 1.0, iv),
            at(iu, iv + 1.0),
            at(iu + 1.0, iv + 1.0),
        );
        std::array::from_fn(|k| {
            (f32::from(a[k]) * (1.0 - fu) + f32::from(b[k]) * fu) * (1.0 - fv)
                + (f32::from(c[k]) * (1.0 - fu) + f32::from(e[k]) * fu) * fv
        })
    }

    fn halved(&self) -> Self {
        let (w, h) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let mut px = vec![[0u8; 4]; w * h];
        px.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, out) in row.iter_mut().enumerate() {
                let mut acc = [0u32; 4];
                let mut n = 0u32;
                for sy in 2 * y..(2 * y + 2).min(self.height) {
                    for sx in 2 * x..(2 * x + 2).min(self.width) {
                        let p = self.px[sy * self.width + sx];
                        for c in 0..4 {
                            acc[c] += u32::from(p[c]);
                        }
                        n += 1;
                    }
                }
                *out = acc.map(|v| ((v + n / 2) / n) as u8);
            }
        });
        Self {
            width: w,
            height: h,
            px,
        }
    }
}

/// The layer the dialog distorts: its full-resolution planes (shared, not
/// copied) and box-reduced levels built as zooming out needs them.
#[derive(Debug, Clone)]
pub struct LiquifySource {
    width: usize,
    height: usize,
    color: [Plane<u8>; 3],
    alpha: Option<Plane<u8>>,
    /// `levels[k]` is reduced by `2^(k+1)`.
    levels: Vec<Level>,
}

impl LiquifySource {
    /// A `width`×`height` layer's colour planes and optional alpha, each
    /// `width * height` samples (a Grayscale layer passes its plane thrice).
    pub fn new(width: u32, height: u32, color: [Plane<u8>; 3], alpha: Option<Plane<u8>>) -> Self {
        Self {
            width: width.max(1) as usize,
            height: height.max(1) as usize,
            color,
            alpha,
            levels: Vec::new(),
        }
    }

    fn premultiplied(&self, i: usize) -> [f32; 4] {
        let a = self.alpha.as_ref().map_or(255.0, |a| f32::from(a[i]));
        let k = a / 255.0;
        [
            f32::from(self.color[0][i]) * k,
            f32::from(self.color[1][i]) * k,
            f32::from(self.color[2][i]) * k,
            a,
        ]
    }

    fn full_bilinear(&self, u: f64, v: f64) -> [f32; 4] {
        let (iu, iv) = (u.floor(), v.floor());
        let (fu, fv) = ((u - iu) as f32, (v - iv) as f32);
        let at = |x: f64, y: f64| {
            let x = x.clamp(0.0, (self.width - 1) as f64) as usize;
            let y = y.clamp(0.0, (self.height - 1) as f64) as usize;
            self.premultiplied(y * self.width + x)
        };
        let (a, b, c, e) = (
            at(iu, iv),
            at(iu + 1.0, iv),
            at(iu, iv + 1.0),
            at(iu + 1.0, iv + 1.0),
        );
        std::array::from_fn(|k| {
            (a[k] * (1.0 - fu) + b[k] * fu) * (1.0 - fv) + (c[k] * (1.0 - fu) + e[k] * fu) * fv
        })
    }

    fn full_bicubic(&self, u: f64, v: f64) -> [f32; 4] {
        let (iu, iv) = (u.floor(), v.floor());
        let (wx, wy) = (catmull_rom(u - iu), catmull_rom(v - iv));
        let (iu, iv) = (iu as i64, iv as i64);
        let mut acc = [0.0f64; 4];
        for (j, wyj) in wy.iter().enumerate() {
            let y = (iv - 1 + j as i64).clamp(0, self.height as i64 - 1) as usize;
            for (i, wxi) in wx.iter().enumerate() {
                let x = (iu - 1 + i as i64).clamp(0, self.width as i64 - 1) as usize;
                let p = self.premultiplied(y * self.width + x);
                let k = wxi * wyj;
                for c in 0..4 {
                    acc[c] += f64::from(p[c]) * k;
                }
            }
        }
        // Catmull-Rom overshoots; keep the result a valid premultiplied pixel.
        let a = acc[3].clamp(0.0, 255.0);
        [
            acc[0].clamp(0.0, a) as f32,
            acc[1].clamp(0.0, a) as f32,
            acc[2].clamp(0.0, a) as f32,
            a as f32,
        ]
    }

    /// Build the levels down to `2^count` reduction.
    fn ensure_levels(&mut self, count: usize) {
        while self.levels.len() < count {
            let next = match self.levels.last() {
                Some(level) => level.halved(),
                None => {
                    let (w, h) = (self.width, self.height);
                    let full = Level {
                        width: w,
                        height: h,
                        px: (0..w * h)
                            .into_par_iter()
                            .map(|i| self.premultiplied(i).map(|v| (v + 0.5) as u8))
                            .collect(),
                    };
                    full.halved()
                }
            };
            let done = next.width == 1 && next.height == 1;
            self.levels.push(next);
            if done {
                break;
            }
        }
    }

    /// Render `w`×`h` premultiplied RGBA8 pixels into `out`: pixel `(i, j)`
    /// shows the distorted layer at layer point
    /// `origin + ((i + 0.5) / zoom, (j + 0.5) / zoom)`, transparent off the
    /// layer. Frozen areas are tinted with `mask` at half strength.
    pub fn render(
        &mut self,
        mesh: &LiquifyMesh,
        origin: [f64; 2],
        zoom: f64,
        (w, h): (usize, usize),
        out: &mut [u8],
        mask: Option<[u8; 3]>,
    ) {
        if w == 0 || h == 0 || out.len() != w * h * 4 || !(zoom > 0.0 && zoom.is_finite()) {
            return;
        }
        // Minified past 2:1, read the level whose pixels are about a screen
        // pixel each.
        let level = if zoom >= 0.5 {
            0
        } else {
            ((1.0 / zoom).log2().floor() as usize).max(1)
        };
        if level > 0 {
            self.ensure_levels(level);
        }
        let this = &*self;
        let lv = (level > 0).then(|| &this.levels[(level - 1).min(this.levels.len() - 1)]);
        let lv_scale = lv.map_or(1.0, |l| this.width as f64 / l.width as f64);
        let (lw, lh) = (this.width as f64, this.height as f64);
        let row = |j: usize, line: &mut [u8]| {
            let y = origin[1] + (j as f64 + 0.5) / zoom;
            for i in 0..w {
                let x = origin[0] + (i as f64 + 0.5) / zoom;
                let px = &mut line[i * 4..i * 4 + 4];
                if !(0.0..lw).contains(&x) || !(0.0..lh).contains(&y) {
                    px.fill(0);
                    continue;
                }
                let mut rgba = match lv {
                    None => {
                        let (gx, gy) = mesh.to_grid(x, y);
                        let d = mesh.sample_cubic(gx, gy);
                        let (u, v) = (x + d[0] - 0.5, y + d[1] - 0.5);
                        if zoom >= 1.0 {
                            this.full_bicubic(u, v)
                        } else {
                            this.full_bilinear(u, v)
                        }
                    }
                    Some(l) => {
                        let d = mesh.displacement(x, y);
                        l.bilinear((x + d[0]) / lv_scale - 0.5, (y + d[1]) / lv_scale - 0.5)
                    }
                };
                if let Some(m) = mask {
                    let t = 0.5 * mesh.freeze_at(x, y);
                    if t > 0.0 {
                        for c in 0..3 {
                            rgba[c] = rgba[c] * (1.0 - t) + f32::from(m[c]) * t;
                        }
                        rgba[3] = rgba[3] * (1.0 - t) + 255.0 * t;
                    }
                }
                for c in 0..4 {
                    px[c] = (rgba[c] + 0.5).clamp(0.0, 255.0) as u8;
                }
            }
        };
        let stride = w * 4;
        if w * h >= PARALLEL_PIXELS {
            out.par_chunks_mut(stride)
                .enumerate()
                .for_each(|(j, line)| row(j, line));
        } else {
            out.chunks_mut(stride)
                .enumerate()
                .for_each(|(j, line)| row(j, line));
        }
    }
}
