//! Resampling through a [`LensMap`]: premultiplied Catmull-Rom at each
//! channel's own source point, the vignette gain, then the Edge fill blended
//! by how much of the pixel lands on the layer. The preview and the commit
//! share [`shade`], so the dialog shows exactly what OK writes.

use pictura_core::{PixelBuffer, Plane};
use rayon::prelude::*;

use super::super::liquify::catmull_rom;
use super::{LensEdge, LensMap};

/// The layer a correction resamples: its colour planes and optional alpha,
/// each `width * height` samples (a Grayscale layer passes its plane thrice).
#[derive(Debug, Clone)]
pub struct LensSource {
    width: usize,
    height: usize,
    color: [Plane<u8>; 3],
    alpha: Option<Plane<u8>>,
}

impl LensSource {
    pub fn new(width: u32, height: u32, color: [Plane<u8>; 3], alpha: Option<Plane<u8>>) -> Self {
        Self {
            width: width.max(1) as usize,
            height: height.max(1) as usize,
            color,
            alpha,
        }
    }

    pub(super) fn from_buffer(buf: &PixelBuffer) -> Self {
        let n = buf.pixel_count();
        let plane = |c: usize| -> Plane<u8> { buf.data[c * n..(c + 1) * n].to_vec().into() };
        Self::new(
            buf.width,
            buf.height,
            [plane(0), plane(1), plane(2)],
            (buf.channels == 4).then(|| plane(3)),
        )
    }

    /// Premultiplied colour and alpha (0..255) at source point `p`, the taps
    /// clamped to the layer.
    fn sample(&self, p: [f64; 2]) -> [f64; 4] {
        let (w, h) = (self.width, self.height);
        let u = snap(p[0] - 0.5);
        let v = snap(p[1] - 0.5);
        let (x0, y0) = (u.floor(), v.floor());
        let (wx, wy) = (catmull_rom(u - x0), catmull_rom(v - y0));
        let tap = |base: f64, k: usize, len: usize| {
            (base as i64 + k as i64 - 1).clamp(0, len as i64 - 1) as usize
        };
        let mut out = [0.0; 4];
        for (j, &ky) in wy.iter().enumerate() {
            let row = tap(y0, j, h) * w;
            for (i, &kx) in wx.iter().enumerate() {
                let at = row + tap(x0, i, w);
                let k = kx * ky;
                let a = self.alpha.as_ref().map_or(255.0, |al| f64::from(al[at]));
                for (o, plane) in out.iter_mut().zip(&self.color) {
                    *o += k * f64::from(plane[at]) * a / 255.0;
                }
                out[3] += k * a;
            }
        }
        let alpha = out[3].clamp(0.0, 255.0);
        out[3] = alpha;
        for v in &mut out[..3] {
            *v = v.clamp(0.0, alpha);
        }
        out
    }

    /// The layer unchanged as a `w`×`h` premultiplied RGBA8 view whose
    /// pixel `(i, j)` shows layer point `origin + ((i + ½) / zoom,
    /// (j + ½) / zoom)`, or corrected through `map`; clear off the layer and,
    /// with Transparency, where the correction leaves a blank.
    pub fn render(
        &self,
        map: Option<&LensMap>,
        origin: [f64; 2],
        zoom: f64,
        size: (usize, usize),
        out: &mut [u8],
    ) {
        if size.0 == 0 {
            return;
        }
        let (lw, lh) = (self.width as f64, self.height as f64);
        out.par_chunks_mut(size.0 * 4)
            .take(size.1)
            .enumerate()
            .for_each(|(j, row)| {
                let y = origin[1] + (j as f64 + 0.5) / zoom;
                for (i, px) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let x = origin[0] + (i as f64 + 0.5) / zoom;
                    let inside = (0.0..lw).contains(&x) && (0.0..lh).contains(&y);
                    let v = match (inside, map) {
                        (false, _) => [0.0; 4],
                        (true, None) => self.sample([x, y]),
                        (true, Some(m)) => shade(m, self, [x, y], true),
                    };
                    for (o, s) in px.iter_mut().zip(v) {
                        *o = s.round().clamp(0.0, 255.0) as u8;
                    }
                }
            });
    }
}

fn snap(v: f64) -> f64 {
    let r = v.round();
    if (v - r).abs() < 1e-6 {
        r
    } else {
        v
    }
}

/// The premultiplied fill for a blank pixel.
fn blank(edge: LensEdge, has_alpha: bool) -> [f64; 4] {
    match edge {
        LensEdge::Transparency if has_alpha => [0.0; 4],
        LensEdge::Black => [0.0, 0.0, 0.0, 255.0],
        _ => [255.0; 4],
    }
}

/// Output point `p` corrected, premultiplied (0..255). `clear` lets
/// Transparency clear the pixel even with no alpha plane, as the preview
/// shows it.
fn shade(map: &LensMap, src: &LensSource, p: [f64; 2], clear: bool) -> [f64; 4] {
    let fill = blank(map.edge, clear);
    let Some(s) = map.source(p) else {
        return fill;
    };
    let (w, h) = (src.width as f64, src.height as f64);
    let (s, cover) = if map.edge == LensEdge::Extension {
        ([s[0].clamp(0.0, w), s[1].clamp(0.0, h)], 1.0)
    } else {
        let d = s[0].min(w - s[0]).min(s[1]).min(h - s[1]);
        (s, (d + 0.5).clamp(0.0, 1.0))
    };
    if cover <= 0.0 {
        return fill;
    }
    let mut v = src.sample(map.channel(s, 1));
    if map.has_fringe() {
        // Each colour comes from its own point, re-weighted to the alpha
        // at the green point.
        for c in [0, 2] {
            let o = src.sample(map.channel(s, c));
            v[c] = if o[3] > 0.0 { o[c] * v[3] / o[3] } else { 0.0 };
        }
    }
    let (gain, alpha) = (map.gain(s), v[3]);
    for c in &mut v[..3] {
        *c = (*c * gain).clamp(0.0, alpha);
    }
    if cover < 1.0 {
        for (o, f) in v.iter_mut().zip(fill) {
            *o = *o * cover + f * (1.0 - cover);
        }
    }
    v
}

/// Write `source` corrected through `map` over `buf`'s planes.
pub(super) fn commit(map: &LensMap, source: &LensSource, buf: &mut PixelBuffer) {
    let (w, h) = (buf.width as usize, buf.height as usize);
    let planes = buf.channels as usize;
    let rows: Vec<Vec<u8>> = (0..h)
        .into_par_iter()
        .map(|y| {
            let mut out = vec![0u8; w * planes];
            for (x, px) in out.chunks_exact_mut(planes).enumerate() {
                let v = shade(map, source, [x as f64 + 0.5, y as f64 + 0.5], planes == 4);
                let a = v[3];
                for (c, o) in px.iter_mut().enumerate() {
                    let straight = match c {
                        3 => a,
                        _ if planes == 4 && a > 0.0 => v[c] * 255.0 / a,
                        _ if planes == 4 => 0.0,
                        _ => v[c],
                    };
                    *o = straight.round().clamp(0.0, 255.0) as u8;
                }
            }
            out
        })
        .collect();
    let n = w * h;
    let data = &mut *buf.data;
    for (y, row) in rows.iter().enumerate() {
        for (x, px) in row.chunks_exact(planes).enumerate() {
            for (c, &v) in px.iter().enumerate() {
                data[c * n + y * w + x] = v;
            }
        }
    }
}
