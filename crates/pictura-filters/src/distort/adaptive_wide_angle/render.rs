//! Rendering a solved warp: the fine mesh's triangles are rasterized in
//! output space, each covered pixel centre mapping by barycentric weights to
//! a source point sampled bilinearly (alpha-weighted, edges clamped). The
//! first triangle in grid order owns a pixel; bands of rows run in parallel,
//! each in that order, so the result does not depend on the thread count.
//! ponytail: bilinear sampling, so a strongly minified view aliases.

use pictura_core::{PixelBuffer, Plane};
use rayon::prelude::*;

use super::{edge, AwaWarp, Lens};

/// Rows per parallel band.
const BAND: usize = 16;

/// The layer a warp resamples: its colour planes and optional alpha, each
/// `width * height` samples (a Grayscale layer passes its plane thrice).
#[derive(Debug, Clone)]
pub struct AwaSource {
    width: usize,
    height: usize,
    color: [Plane<u8>; 3],
    alpha: Option<Plane<u8>>,
}

impl AwaSource {
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

    /// Straight colour and alpha (0..255) at source point `p`.
    fn sample(&self, p: [f64; 2]) -> [f64; 4] {
        let (w, h) = (self.width, self.height);
        let u = snap(p[0] - 0.5).clamp(0.0, (w - 1) as f64);
        let v = snap(p[1] - 0.5).clamp(0.0, (h - 1) as f64);
        let (x0, y0) = (u.floor() as usize, v.floor() as usize);
        let (fx, fy) = (u - x0 as f64, v - y0 as f64);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let taps = [
            (y0 * w + x0, (1.0 - fx) * (1.0 - fy)),
            (y0 * w + x1, fx * (1.0 - fy)),
            (y1 * w + x0, (1.0 - fx) * fy),
            (y1 * w + x1, fx * fy),
        ];
        let mut out = [0.0; 4];
        match &self.alpha {
            Some(alpha) => {
                let mut covered = 0.0;
                for &(i, wt) in &taps {
                    let aw = wt * f64::from(alpha[i]);
                    covered += aw;
                    self.add_color(&mut out, i, aw);
                }
                if covered > 0.0 {
                    for v in &mut out[..3] {
                        *v /= covered;
                    }
                } else {
                    for &(i, wt) in &taps {
                        self.add_color(&mut out, i, wt);
                    }
                }
                out[3] = covered;
            }
            None => {
                for &(i, wt) in &taps {
                    self.add_color(&mut out, i, wt);
                }
                out[3] = 255.0;
            }
        }
        out
    }

    fn add_color(&self, out: &mut [f64; 4], i: usize, weight: f64) {
        for (o, plane) in out.iter_mut().zip(&self.color) {
            *o += weight * f64::from(plane[i]);
        }
    }

    /// The layer unwarped: a `w`×`h` premultiplied RGBA8 view whose pixel
    /// `(i, j)` shows layer point `origin + ((i + ½) / zoom, (j + ½) / zoom)`,
    /// clear off the layer.
    pub fn render(&self, origin: [f64; 2], zoom: f64, size: (usize, usize), out: &mut [u8]) {
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
                    px.copy_from_slice(&if inside {
                        premultiplied(self.sample([x, y]))
                    } else {
                        [0; 4]
                    });
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

fn premultiplied(s: [f64; 4]) -> [u8; 4] {
    let a = s[3] / 255.0;
    let byte = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    [byte(s[0] * a), byte(s[1] * a), byte(s[2] * a), byte(s[3])]
}

/// A fine-mesh triangle in view space with its source corners.
struct Tri {
    o: [[f64; 2]; 3],
    s: [[f64; 2]; 3],
    area: f64,
    lo: [f64; 2],
    hi: [f64; 2],
}

/// The warp's triangles mapped to the view `(q − origin)·zoom`, and for each
/// band of `BAND` rows the triangles that reach it, in grid order.
fn setup(
    warp: &AwaWarp,
    origin: [f64; 2],
    zoom: f64,
    size: (usize, usize),
) -> (Vec<Tri>, Vec<Vec<u32>>) {
    let view = |q: [f64; 2]| [(q[0] - origin[0]) * zoom, (q[1] - origin[1]) * zoom];
    let (w, h) = (size.0 as f64, size.1 as f64);
    let mut tris = Vec::new();
    let mut bands = vec![Vec::new(); size.1.div_ceil(BAND)];
    if size.0 == 0 || bands.is_empty() {
        return (tris, bands);
    }
    for t in warp.triangles() {
        let o = t.map(|k| view(warp.fine_out[k]));
        let area = edge(o[0], o[1], o[2]);
        if area.abs() < 1e-12 {
            continue;
        }
        let lo = [
            o[0][0].min(o[1][0]).min(o[2][0]),
            o[0][1].min(o[1][1]).min(o[2][1]),
        ];
        let hi = [
            o[0][0].max(o[1][0]).max(o[2][0]),
            o[0][1].max(o[1][1]).max(o[2][1]),
        ];
        if hi[0] < 0.0 || hi[1] < 0.0 || lo[0] > w || lo[1] > h {
            continue;
        }
        let top = bands.len() - 1;
        let first = (((lo[1] - 0.5).ceil().max(0.0) as usize) / BAND).min(top);
        let last = (((hi[1] - 0.5).floor().max(0.0) as usize) / BAND).min(top);
        let index = tris.len() as u32;
        tris.push(Tri {
            o,
            s: t.map(|k| warp.fine_point(k)),
            area,
            lo,
            hi,
        });
        for band in bands.iter_mut().take(last + 1).skip(first) {
            band.push(index);
        }
    }
    (tris, bands)
}

/// The source point each pixel of rows `y0..y0 + rows` shows, NaN where no
/// triangle covers it or it lands past a fisheye's rim.
fn raster_band(
    lens: &Lens,
    tris: &[Tri],
    list: &[u32],
    y0: usize,
    rows: usize,
    width: usize,
) -> Vec<[f64; 2]> {
    let mut out = vec![[f64::NAN; 2]; rows * width];
    for r in 0..rows {
        let py = (y0 + r) as f64 + 0.5;
        let line = &mut out[r * width..(r + 1) * width];
        for t in list.iter().map(|&i| &tris[i as usize]) {
            if py < t.lo[1] || py > t.hi[1] {
                continue;
            }
            let from = (t.lo[0] - 0.5).ceil().max(0.0) as usize;
            let to = ((t.hi[0] - 0.5).floor()).min(width as f64 - 1.0);
            if to < 0.0 {
                continue;
            }
            for (i, px) in line.iter_mut().enumerate().take(to as usize + 1).skip(from) {
                if !px[0].is_nan() {
                    continue;
                }
                let p = [i as f64 + 0.5, py];
                let la = edge(t.o[1], t.o[2], p) / t.area;
                let lb = edge(t.o[2], t.o[0], p) / t.area;
                let lc = 1.0 - la - lb;
                if la < -1e-9 || lb < -1e-9 || lc < -1e-9 {
                    continue;
                }
                let s = [
                    la * t.s[0][0] + lb * t.s[1][0] + lc * t.s[2][0],
                    la * t.s[0][1] + lb * t.s[1][1] + lc * t.s[2][1],
                ];
                if !lens.clipped(s) {
                    *px = s;
                }
            }
        }
    }
    out
}

impl AwaWarp {
    /// The corrected layer as a `w`×`h` premultiplied RGBA8 view whose pixel
    /// `(i, j)` shows output point `origin + ((i + ½) / zoom, (j + ½) / zoom)`,
    /// clear where nothing lands.
    pub fn render(
        &self,
        source: &AwaSource,
        origin: [f64; 2],
        zoom: f64,
        size: (usize, usize),
        out: &mut [u8],
    ) {
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let (tris, bands) = setup(self, origin, zoom, size);
        out.par_chunks_mut(size.0 * 4 * BAND)
            .take(bands.len())
            .enumerate()
            .for_each(|(b, chunk)| {
                let rows = chunk.len() / (size.0 * 4);
                let coords = raster_band(&self.lens, &tris, &bands[b], b * BAND, rows, size.0);
                for (px, s) in chunk.as_chunks_mut::<4>().0.iter_mut().zip(&coords) {
                    px.copy_from_slice(&if s[0].is_nan() {
                        [0; 4]
                    } else {
                        premultiplied(source.sample(*s))
                    });
                }
            });
    }
}

/// Write the corrected `source` over `buf`'s planes.
pub(super) fn commit(warp: &AwaWarp, source: &AwaSource, buf: &mut PixelBuffer) {
    let (w, h) = (buf.width as usize, buf.height as usize);
    let planes = buf.channels as usize;
    let (tris, bands) = setup(warp, [0.0; 2], 1.0, (w, h));
    let blank: [u8; 4] = if planes == 4 { [0; 4] } else { [255; 4] };
    let results: Vec<Vec<u8>> = (0..bands.len())
        .into_par_iter()
        .map(|b| {
            let rows = BAND.min(h - b * BAND);
            let coords = raster_band(&warp.lens, &tris, &bands[b], b * BAND, rows, w);
            let mut out = vec![0u8; coords.len() * planes];
            for (px, s) in out.chunks_exact_mut(planes).zip(&coords) {
                if s[0].is_nan() {
                    px.copy_from_slice(&blank[..planes]);
                    continue;
                }
                let v = source.sample(*s);
                for (c, o) in px.iter_mut().enumerate() {
                    *o = v[c].round().clamp(0.0, 255.0) as u8;
                }
            }
            out
        })
        .collect();
    let n = w * h;
    let data = &mut *buf.data;
    for (b, band) in results.iter().enumerate() {
        for (k, px) in band.chunks_exact(planes).enumerate() {
            let at = b * BAND * w + k;
            for (c, &v) in px.iter().enumerate() {
                data[c * n + at] = v;
            }
        }
    }
}
