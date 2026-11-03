//! Liquify's commit: a Catmull-Rom mesh and Catmull-Rom source, rows the
//! mesh leaves alone copied untouched.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use super::{catmull_rom, LiquifyMesh};
use crate::{validate, FilterError};

/// Resample every plane of `buf` through `mesh` (stretched to fit when the
/// sizes differ): each pixel shows the source at `p + d(p)`, edge pixels
/// repeating past the layer. An alpha plane moves with the colour, as in the
/// dialog's preview; the planes resample straight, not premultiplied as the
/// preview does, so a semi-transparent edge's colour can differ from it.
/// ponytail: straight resampling; a premultiplied commit would close that gap.
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
    let planes = buf.channels as usize;
    let src = buf.data[..planes * n].to_vec();
    let moved: Vec<bool> = mesh
        .d
        .chunks_exact(mesh.cols)
        .map(|row| row.iter().any(|v| v[0] != 0.0 || v[1] != 0.0))
        .collect();
    // Outside the processing rectangle every sample is the source, so only
    // the rows and columns inside it are resampled.
    let (x0, x1, y0, y1) = match mesh.clip {
        Some(c) => (
            c.left.max(0) as usize,
            c.right.min(w as i32).max(0) as usize,
            c.top.max(0) as usize,
            c.bottom.min(h as i32).max(0) as usize,
        ),
        None => (0, w, 0, h),
    };
    let rows: Vec<(usize, Vec<u8>)> = (y0..y1)
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
            for c in 0..planes {
                out[c * w..(c + 1) * w].copy_from_slice(&src[c * n + y * w..c * n + (y + 1) * w]);
            }
            for x in x0..x1 {
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
