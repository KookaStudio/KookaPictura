//! Liquify's two resamplers: the commit ([`liquify`]: Catmull-Rom mesh,
//! Catmull-Rom source, rows the mesh leaves alone copied untouched) and the
//! dialog preview ([`LiquifyProxy`]: bilinear mesh, bilinear source, on a
//! reduced premultiplied copy, re-rendered only where a dab changed things).

use pictura_core::{PixelBuffer, PsdRect};
use rayon::prelude::*;

use super::{catmull_rom, LiquifyMesh};
use crate::{validate, FilterError};

/// A preview rectangle this many pixels or larger renders on the pool; a
/// dab's few thousand pixels are faster on the calling thread.
const PARALLEL_PIXELS: usize = 150_000;

/// Resample `buf`'s colour planes through `mesh` (stretched to fit when the
/// sizes differ): each pixel shows the source at `p + d(p)`, edge pixels
/// repeating past the layer. Alpha is untouched.
pub fn liquify(buf: &mut PixelBuffer, mesh: &LiquifyMesh) -> Result<(), FilterError> {
    validate(buf)?;
    if mesh.is_identity() {
        return Ok(());
    }
    let scaled;
    let mesh = if mesh.width != buf.width || mesh.height != buf.height {
        scaled = mesh.scaled_to(buf.width, buf.height);
        &scaled
    } else {
        mesh
    };
    let (w, h) = (buf.width as usize, buf.height as usize);
    let n = w * h;
    let planes = (buf.channels as usize).min(3);
    let src = buf.data[..planes * n].to_vec();
    let moved: Vec<bool> = mesh
        .d
        .chunks_exact(mesh.cols)
        .map(|row| row.iter().any(|v| v[0] != 0.0 || v[1] != 0.0))
        .collect();
    let rows: Vec<(usize, Vec<u8>)> = (0..h)
        .into_par_iter()
        .filter_map(|y| {
            let gy = y as f64 / mesh.cell;
            let iy = gy.floor() as isize;
            let touched =
                (iy - 1..=iy + 2).any(|j| j >= 0 && (j as usize) < mesh.rows && moved[j as usize]);
            if !touched {
                return None;
            }
            let mut out = vec![0u8; planes * w];
            for x in 0..w {
                let d = mesh.sample_cubic(x as f64 / mesh.cell, gy);
                let u = snap(x as f64 + d[0]);
                let v = snap(y as f64 + d[1]);
                let (ix, iy) = (u.floor(), v.floor());
                let (wx, wy) = (catmull_rom(u - ix), catmull_rom(v - iy));
                let (ix, iy) = (ix as i64, iy as i64);
                for c in 0..planes {
                    let plane = &src[c * n..(c + 1) * n];
                    let mut acc = 0.0f64;
                    for (jj, wyj) in wy.iter().enumerate() {
                        let sy = (iy - 1 + jj as i64).clamp(0, h as i64 - 1) as usize;
                        let row = &plane[sy * w..(sy + 1) * w];
                        for (ii, wxi) in wx.iter().enumerate() {
                            let sx = (ix - 1 + ii as i64).clamp(0, w as i64 - 1) as usize;
                            acc += f64::from(row[sx]) * wxi * wyj;
                        }
                    }
                    out[c * w + x] = acc.round().clamp(0.0, 255.0) as u8;
                }
            }
            Some((y, out))
        })
        .collect();
    let data = &mut *buf.data;
    for (y, row) in rows {
        for c in 0..planes {
            data[c * n + y * w..c * n + (y + 1) * w].copy_from_slice(&row[c * w..(c + 1) * w]);
        }
    }
    Ok(())
}

/// Rounds coordinates within float noise of a pixel so an unmoved pixel reads
/// its source exactly.
fn snap(t: f64) -> f64 {
    if (t - t.round()).abs() < 1e-7 {
        t.round()
    } else {
        t
    }
}

/// A box-reduced, premultiplied RGBA8 copy of a layer for the dialog preview.
#[derive(Debug, Clone)]
pub struct LiquifyProxy {
    /// Layer pixels per proxy pixel.
    scale: u32,
    width: usize,
    height: usize,
    px: Vec<[u8; 4]>,
}

impl LiquifyProxy {
    /// Reduce a `width`×`height` layer's planes so the longer side is at most
    /// `max_side`, by a whole factor. A missing alpha plane is opaque.
    pub fn new(
        width: u32,
        height: u32,
        color: [&[u8]; 3],
        alpha: Option<&[u8]>,
        max_side: u32,
    ) -> Self {
        let (lw, lh) = (width.max(1) as usize, height.max(1) as usize);
        let scale = (lw.max(lh) as u32).div_ceil(max_side.max(1)).max(1);
        let s = scale as usize;
        let (pw, ph) = (lw.div_ceil(s), lh.div_ceil(s));
        let mut px = vec![[0u8; 4]; pw * ph];
        px.par_chunks_mut(pw).enumerate().for_each(|(py, row)| {
            for (x, out) in row.iter_mut().enumerate() {
                let mut acc = [0u32; 4];
                let mut count = 0u32;
                for sy in py * s..((py + 1) * s).min(lh) {
                    for sx in x * s..((x + 1) * s).min(lw) {
                        let i = sy * lw + sx;
                        let a = u32::from(alpha.map_or(255, |a| a[i]));
                        for c in 0..3 {
                            acc[c] += u32::from(color[c][i]) * a;
                        }
                        acc[3] += a;
                        count += 1;
                    }
                }
                *out = [
                    ((acc[0] + count * 127) / (count * 255)) as u8,
                    ((acc[1] + count * 127) / (count * 255)) as u8,
                    ((acc[2] + count * 127) / (count * 255)) as u8,
                    ((acc[3] + count / 2) / count) as u8,
                ];
            }
        });
        Self {
            scale,
            width: pw,
            height: ph,
            px,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Layer pixels per proxy pixel.
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// The proxy pixels covering layer rectangle `r`, one pixel of slack
    /// around for the bilinear reads.
    pub fn proxy_rect(&self, r: PsdRect) -> PsdRect {
        let s = self.scale as i32;
        PsdRect {
            left: (r.left.div_euclid(s) - 1).max(0),
            top: (r.top.div_euclid(s) - 1).max(0),
            right: ((r.right + s - 1).div_euclid(s) + 1).min(self.width as i32),
            bottom: ((r.bottom + s - 1).div_euclid(s) + 1).min(self.height as i32),
        }
    }

    /// Render proxy rectangle `r` through `mesh` into `out` (premultiplied
    /// RGBA8 at the proxy's size), tinting frozen areas with `mask`.
    pub fn render(&self, mesh: &LiquifyMesh, r: PsdRect, out: &mut [u8], mask: Option<[u8; 3]>) {
        let x0 = r.left.clamp(0, self.width as i32) as usize;
        let x1 = r.right.clamp(0, self.width as i32) as usize;
        let y0 = r.top.clamp(0, self.height as i32) as usize;
        let y1 = r.bottom.clamp(0, self.height as i32) as usize;
        if x0 >= x1 || y0 >= y1 || out.len() != self.width * self.height * 4 {
            return;
        }
        let s = f64::from(self.scale);
        // The mesh covers the layer; a proxy pixel past its edge (a layer
        // that does not divide by the scale) reads the nearest node.
        let row = |y: usize, line: &mut [u8]| {
            let py = (y as f64 + 0.5) * s;
            for x in x0..x1 {
                let px = (x as f64 + 0.5) * s;
                let d = mesh.displacement(px, py);
                let u = (px + d[0]) / s - 0.5;
                let v = (py + d[1]) / s - 0.5;
                let mut rgba = self.bilinear(u, v);
                if let Some(m) = mask {
                    let t = 0.5 * mesh.freeze_at(px, py);
                    if t > 0.0 {
                        for c in 0..3 {
                            rgba[c] = rgba[c] * (1.0 - t) + f32::from(m[c]) * t;
                        }
                        rgba[3] = rgba[3] * (1.0 - t) + 255.0 * t;
                    }
                }
                for c in 0..4 {
                    line[x * 4 + c] = (rgba[c] + 0.5).clamp(0.0, 255.0) as u8;
                }
            }
        };
        let stride = self.width * 4;
        let rows = &mut out[y0 * stride..y1 * stride];
        if (x1 - x0) * (y1 - y0) >= PARALLEL_PIXELS {
            rows.par_chunks_mut(stride)
                .enumerate()
                .for_each(|(i, line)| row(y0 + i, line));
        } else {
            rows.chunks_mut(stride)
                .enumerate()
                .for_each(|(i, line)| row(y0 + i, line));
        }
    }

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
}
