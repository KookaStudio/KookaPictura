//! The interleaved 8-bit straight-alpha RGBA image the photorust filter
//! modules work on, and its round trip through Kooka's planar
//! [`PixelBuffer`].
//!
//! Ported from perfecto25/photorust (`core/src/buffer.rs`, `resample.rs`,
//! `brush.rs`), GPL-3.0. Only the 8-bit path the filters use is kept; the
//! 16/32-bit storage, history stamps, and Qt hand-off stay behind.

use pictura_core::PixelBuffer;

/// A single straight-alpha RGBA pixel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const TRANSPARENT: Rgba8 = Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const BLACK: Rgba8 = Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Rgba8 = Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// An opaque colour from Kooka's `[r, g, b]` parameters.
    pub const fn rgb(c: [u8; 3]) -> Self {
        Self::new(c[0], c[1], c[2], 255)
    }
}

/// An axis-aligned rectangle in image space.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[cfg(test)]
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    #[cfg(test)]
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    #[cfg(test)]
    /// Geometric intersection; empty when they do not overlap.
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        if x1 <= x0 || y1 <= y0 {
            Rect::default()
        } else {
            Rect::new(x0, y0, (x1 - x0) as u32, (y1 - y0) as u32)
        }
    }
}

/// A dense, row-major, 8-bit straight-alpha RGBA image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pixmap {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Pixmap {
    /// A fully transparent pixmap.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; width as usize * height as usize * 4],
        }
    }

    #[cfg(test)]
    pub fn filled(width: u32, height: u32, color: Rgba8) -> Self {
        let mut pm = Self::new(width, height);
        pm.fill(color);
        pm
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    #[cfg(test)]
    pub fn rect(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Bytes per row.
    pub fn stride(&self) -> usize {
        self.width as usize * 4
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Row `y`. Panics if `y` is out of range.
    pub fn row(&self, y: u32) -> &[u8] {
        let start = y as usize * self.stride();
        &self.data[start..start + self.stride()]
    }

    /// The pixel at `(x, y)`, transparent outside the image.
    pub fn get(&self, x: i32, y: i32) -> Rgba8 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return Rgba8::TRANSPARENT;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        Rgba8::new(
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        )
    }

    /// Write the pixel at `(x, y)`; ignored outside the image.
    pub fn set(&mut self, x: i32, y: i32, px: Rgba8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.data[i..i + 4].copy_from_slice(&[px.r, px.g, px.b, px.a]);
    }

    pub fn fill(&mut self, color: Rgba8) {
        for px in self.data.chunks_exact_mut(4) {
            px.copy_from_slice(&[color.r, color.g, color.b, color.a]);
        }
    }

    #[cfg(test)]
    /// Fill only within `rect`, clipped to the pixmap.
    pub fn fill_rect(&mut self, rect: Rect, color: Rgba8) {
        let r = rect.intersect(&self.rect());
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.set(x, y, color);
            }
        }
    }

    /// Convert to premultiplied alpha in place.
    pub fn premultiply(&mut self) {
        for px in self.data.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a == 255 {
                continue;
            }
            for c in &mut px[..3] {
                *c = ((*c as u32 * a + 127) / 255) as u8;
            }
        }
    }

    /// Inverse of [`Pixmap::premultiply`].
    pub fn unpremultiply(&mut self) {
        for px in self.data.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a == 255 || a == 0 {
                continue;
            }
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }

    /// Interleave a planar RGB / RGBA buffer; an RGB buffer reads as opaque.
    pub(crate) fn from_planar(buf: &PixelBuffer) -> Self {
        let n = buf.width as usize * buf.height as usize;
        let mut pm = Self::new(buf.width, buf.height);
        for (i, px) in pm.data.chunks_exact_mut(4).enumerate() {
            px[0] = buf.data[i];
            px[1] = buf.data[n + i];
            px[2] = buf.data[2 * n + i];
            px[3] = if buf.channels == 4 {
                buf.data[3 * n + i]
            } else {
                255
            };
        }
        pm
    }

    /// Write the colour planes back into `buf`. Used by every filter: a value
    /// kernel's result never touches the pixmap's alpha.
    pub(crate) fn write_colour(&self, buf: &mut PixelBuffer) {
        let n = buf.width as usize * buf.height as usize;
        for (i, px) in self.data.chunks_exact(4).enumerate() {
            buf.data[i] = px[0];
            buf.data[n + i] = px[1];
            buf.data[2 * n + i] = px[2];
        }
    }

    /// Write the alpha plane back into a four-channel `buf`, for a moving
    /// kernel whose run over the pixmap is the layer's filtered transparency.
    pub(crate) fn write_alpha(&self, buf: &mut PixelBuffer) {
        debug_assert_eq!(buf.channels, 4);
        let n = buf.width as usize * buf.height as usize;
        for (i, px) in self.data.chunks_exact(4).enumerate() {
            buf.data[3 * n + i] = px[3];
        }
    }
}

/// Straight-alpha source-over of `src` at coverage `alpha` onto `dst`. Ported
/// from perfecto25/photorust (`core/src/brush.rs`), GPL-3.0.
pub(crate) fn source_over(dst: Rgba8, src: Rgba8, alpha: f32) -> Rgba8 {
    let sa = (src.a as f32 / 255.0) * alpha.clamp(0.0, 1.0);
    if sa <= 0.0 {
        return dst;
    }
    let da = dst.a as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return Rgba8::TRANSPARENT;
    }
    // Composite in premultiplied space, then convert back to straight alpha.
    let mix = |s: u8, d: u8| -> u8 {
        let s = s as f32 / 255.0;
        let d = d as f32 / 255.0;
        let v = (s * sa + d * da * (1.0 - sa)) / out_a;
        (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    Rgba8::new(
        mix(src.r, dst.r),
        mix(src.g, dst.g),
        mix(src.b, dst.b),
        (out_a * 255.0 + 0.5) as u8,
    )
}

/// The pixel at `(x, y)` with coordinates clamped to the image.
pub(crate) fn sample_clamped(src: &Pixmap, x: i32, y: i32) -> Rgba8 {
    let cx = x.clamp(0, src.width() as i32 - 1);
    let cy = y.clamp(0, src.height() as i32 - 1);
    src.get(cx, cy)
}

fn to_u8(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

/// Bilinear sample at a fractional position, clamping at the edges.
pub(crate) fn bilinear(src: &Pixmap, fx: f32, fy: f32) -> Rgba8 {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let mut acc = [0.0f32; 4];
    for (dy, wy) in [(0, 1.0 - ty), (1, ty)] {
        for (dx, wx) in [(0, 1.0 - tx), (1, tx)] {
            let p = sample_clamped(src, x0 + dx, y0 + dy);
            let w = wx * wy;
            acc[0] += p.r as f32 * w;
            acc[1] += p.g as f32 * w;
            acc[2] += p.b as f32 * w;
            acc[3] += p.a as f32 * w;
        }
    }
    Rgba8::new(to_u8(acc[0]), to_u8(acc[1]), to_u8(acc[2]), to_u8(acc[3]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planar_round_trip_writes_colour_and_alpha_on_demand() {
        let mut buf = PixelBuffer {
            width: 2,
            height: 1,
            channels: 4,
            data: vec![10, 20, 30, 40, 50, 60, 7, 8].into(),
        };
        let mut pm = Pixmap::from_planar(&buf);
        assert_eq!(pm.get(1, 0), Rgba8::new(20, 40, 60, 8));
        pm.set(0, 0, Rgba8::new(1, 2, 3, 0));
        pm.write_colour(&mut buf);
        assert_eq!(&buf.data[..], &[1, 20, 2, 40, 3, 60, 7, 8]);
        pm.write_alpha(&mut buf);
        assert_eq!(&buf.data[..], &[1, 20, 2, 40, 3, 60, 0, 8]);
    }
}
