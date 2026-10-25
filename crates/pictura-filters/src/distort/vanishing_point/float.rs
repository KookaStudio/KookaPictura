//! Floating selections and the marquee, both living in a family's flat
//! space. A float is a texture placed by centre, per-axis scale (flat units
//! per texel), angle and Flip / Flop; it draws through the perspective of
//! whichever plane of its family lies under each pixel.

use std::sync::Arc;

use super::canvas::{bilinear, Canvas, Rect};
use super::geometry::P2;
use super::space::FamilyMap;

/// Straight RGBA texels.
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub rgba: Arc<Vec<u8>>,
}

impl Texture {
    pub fn new(width: usize, height: usize, rgba: Vec<u8>) -> Option<Self> {
        (width > 0 && height > 0 && rgba.len() == width * height * 4).then(|| Texture {
            width,
            height,
            rgba: Arc::new(rgba),
        })
    }

    /// Bilinear sample at texel point `t` (texel centres at `+0.5`), alpha
    /// `0..=1`; clear outside.
    fn sample(&self, t: P2) -> [f32; 4] {
        bilinear(t, |x, y| {
            if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
                return [0; 4];
            }
            let i = (y as usize * self.width + x as usize) * 4;
            [
                self.rgba[i],
                self.rgba[i + 1],
                self.rgba[i + 2],
                self.rgba[i + 3],
            ]
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Floating {
    pub tex: Texture,
    /// The family's root plane, and the plane the float was placed in.
    pub family: usize,
    pub home: usize,
    pub centre: P2,
    pub scale: P2,
    pub angle: f64,
    pub flip: bool,
    pub flop: bool,
}

impl Floating {
    fn half(&self) -> P2 {
        (self.tex.width as f64 / 2.0, self.tex.height as f64 / 2.0)
    }

    /// Texel point → flat point.
    pub fn to_flat(&self, t: P2) -> P2 {
        let (hx, hy) = self.half();
        let mut d = (t.0 - hx, t.1 - hy);
        if self.flip {
            d.0 = -d.0;
        }
        if self.flop {
            d.1 = -d.1;
        }
        let d = (d.0 * self.scale.0, d.1 * self.scale.1);
        let (s, c) = self.angle.sin_cos();
        (
            self.centre.0 + d.0 * c - d.1 * s,
            self.centre.1 + d.0 * s + d.1 * c,
        )
    }

    /// Flat point → texel point.
    pub fn to_texel(&self, f: P2) -> Option<P2> {
        if self.scale.0.abs() < 1e-9 || self.scale.1.abs() < 1e-9 {
            return None;
        }
        let (s, c) = self.angle.sin_cos();
        let (dx, dy) = (f.0 - self.centre.0, f.1 - self.centre.1);
        let mut d = (
            (dx * c + dy * s) / self.scale.0,
            (-dx * s + dy * c) / self.scale.1,
        );
        if self.flip {
            d.0 = -d.0;
        }
        if self.flop {
            d.1 = -d.1;
        }
        let (hx, hy) = self.half();
        Some((d.0 + hx, d.1 + hy))
    }

    /// The texture's corners in flat space, texel order (0,0) (w,0) (w,h) (0,h).
    pub fn flat_corners(&self) -> [P2; 4] {
        let (w, h) = (self.tex.width as f64, self.tex.height as f64);
        [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)].map(|t| self.to_flat(t))
    }
}

/// A marquee: an axis-aligned rect `a`–`b` in a family's flat space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marquee {
    pub family: usize,
    pub home: usize,
    pub a: P2,
    pub b: P2,
}

impl Marquee {
    pub fn min(&self) -> P2 {
        (self.a.0.min(self.b.0), self.a.1.min(self.b.1))
    }

    pub fn max(&self) -> P2 {
        (self.a.0.max(self.b.0), self.a.1.max(self.b.1))
    }

    pub fn contains(&self, f: P2) -> bool {
        let (lo, hi) = (self.min(), self.max());
        f.0 >= lo.0 && f.0 <= hi.0 && f.1 >= lo.1 && f.1 <= hi.1
    }

    pub fn corners(&self) -> [P2; 4] {
        let (lo, hi) = (self.min(), self.max());
        [lo, (hi.0, lo.1), hi, (lo.0, hi.1)]
    }
}

/// A flat polygon's outline in the image, its edges sampled so they bend
/// across hinges.
pub fn outline(map: &FamilyMap, corners: &[P2; 4], prefer: Option<usize>) -> Vec<P2> {
    const STEPS: usize = 24;
    let mut out = Vec::with_capacity(4 * STEPS + 1);
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        for i in 0..STEPS {
            let t = i as f64 / STEPS as f64;
            if let Some(p) = map.to_image((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), prefer) {
                out.push(p);
            }
        }
    }
    if let Some(&first) = out.first() {
        out.push(first);
    }
    out
}

/// The pixels a flat polygon may cover: its outline's bounding box, padded.
pub fn footprint(map: &FamilyMap, corners: &[P2; 4], prefer: Option<usize>) -> Rect {
    let pts = outline(map, corners, prefer);
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in pts {
        b = [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)];
    }
    // The polygon may bulge past its sampled outline where it wraps.
    let planes = map.image_bounds();
    let b = [
        b[0].max(planes[0]),
        b[1].max(planes[1]),
        b[2].min(planes[2]),
        b[3].min(planes[3]),
    ];
    Rect::around(b, 2)
}

/// What the float puts down over `rect`: the colour per pixel and its cover
/// (texel alpha folded in, times Opacity and the Feather ramp).
pub fn render(
    map: &FamilyMap,
    f: &Floating,
    feather: f64,
    opacity: f64,
    rect: Rect,
) -> (Vec<[f32; 4]>, Vec<f32>) {
    let (w, h) = (rect.width(), rect.height());
    let mut src = vec![[0f32; 4]; w * h];
    let mut mask = vec![0f32; w * h];
    let (tw, th) = (f.tex.width as f64, f.tex.height as f64);
    let unit = f.scale.0.abs().min(f.scale.1.abs()).max(1e-9);
    for y in 0..h {
        for x in 0..w {
            let p = (
                (rect.x0 + x as i32) as f64 + 0.5,
                (rect.y0 + y as i32) as f64 + 0.5,
            );
            let Some((_, flat)) = map.locate(p, Some(f.home)) else {
                continue;
            };
            let Some(t) = f.to_texel(flat) else {
                continue;
            };
            if t.0 < 0.0 || t.1 < 0.0 || t.0 > tw || t.1 > th {
                continue;
            }
            let texel = f.tex.sample(t);
            let edge = t.0.min(tw - t.0).min(t.1).min(th - t.1) * unit;
            let ramp = if feather > 0.0 {
                (edge / feather).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let i = y * w + x;
            src[i] = texel;
            mask[i] = (ramp * opacity) as f32;
        }
    }
    (src, mask)
}

/// The pixels under a marquee as a texture, one texel per flat unit:
/// `offset` shifts where they are read from (Source fills).
pub fn lift(map: &FamilyMap, m: &Marquee, canvas: &Canvas, offset: P2) -> Option<Texture> {
    let (lo, hi) = (m.min(), m.max());
    let (w, h) = (
        (hi.0 - lo.0).round() as usize,
        (hi.1 - lo.1).round() as usize,
    );
    if w == 0 || h == 0 || w * h > 64 << 20 {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let f = (
                lo.0 + x as f64 + 0.5 + offset.0,
                lo.1 + y as f64 + 0.5 + offset.1,
            );
            let Some(p) = map.to_image(f, Some(m.home)) else {
                continue;
            };
            // Pixels outside the family read clear.
            if map.locate(p, Some(m.home)).is_none() {
                continue;
            }
            let s = canvas.sample(p);
            let i = (y * w + x) * 4;
            for c in 0..3 {
                rgba[i + c] = s[c].round().clamp(0.0, 255.0) as u8;
            }
            rgba[i + 3] = (s[3] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Texture::new(w, h, rgba)
}
