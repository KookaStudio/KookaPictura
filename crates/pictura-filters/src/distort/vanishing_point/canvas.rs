//! The session's working copy of the active layer, and the pixel steps the
//! dialog's undo swaps in and out of it.

use std::sync::Arc;

use super::geometry::P2;

/// A rectangle of document pixels, `x0..x1 × y0..y1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    pub fn width(&self) -> usize {
        (self.x1 - self.x0).max(0) as usize
    }

    pub fn height(&self) -> usize {
        (self.y1 - self.y0).max(0) as usize
    }

    pub fn union(self, o: Rect) -> Rect {
        if self.is_empty() {
            return o;
        }
        if o.is_empty() {
            return self;
        }
        Rect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    pub fn intersect(self, o: Rect) -> Rect {
        Rect {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    /// The whole pixels touching `[x0, y0, x1, y1]`, grown by `pad`.
    pub fn around(b: [f64; 4], pad: i32) -> Rect {
        if !b.iter().all(|v| v.is_finite()) {
            return Rect::default();
        }
        Rect {
            x0: b[0].floor() as i32 - pad,
            y0: b[1].floor() as i32 - pad,
            x1: b[2].ceil() as i32 + pad,
            y1: b[3].ceil() as i32 + pad,
        }
    }
}

/// Straight RGBA pixels of the active layer, its top-left at document
/// `(ox, oy)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub ox: i32,
    pub oy: i32,
    pub rgba: Vec<u8>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, ox: i32, oy: i32, rgba: Vec<u8>) -> Option<Self> {
        (rgba.len() == width * height * 4).then_some(Canvas {
            width,
            height,
            ox,
            oy,
            rgba,
        })
    }

    /// The layer's extent in document pixels.
    pub fn bounds(&self) -> Rect {
        Rect {
            x0: self.ox,
            y0: self.oy,
            x1: self.ox + self.width as i32,
            y1: self.oy + self.height as i32,
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (lx, ly) = (x - self.ox, y - self.oy);
        (lx >= 0 && ly >= 0 && (lx as usize) < self.width && (ly as usize) < self.height)
            .then(|| (ly as usize * self.width + lx as usize) * 4)
    }

    /// The pixel at document `(x, y)`, clear outside the layer.
    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        self.index(x, y).map_or([0; 4], |i| {
            [
                self.rgba[i],
                self.rgba[i + 1],
                self.rgba[i + 2],
                self.rgba[i + 3],
            ]
        })
    }

    pub fn set(&mut self, x: i32, y: i32, px: [u8; 4]) {
        if let Some(i) = self.index(x, y) {
            self.rgba[i..i + 4].copy_from_slice(&px);
        }
    }

    /// Bilinear sample at document point `p`.
    pub fn sample(&self, p: P2) -> [f32; 4] {
        bilinear(p, |x, y| self.get(x as i32, y as i32))
    }

    pub fn read(&self, r: Rect) -> Vec<u8> {
        let mut out = Vec::with_capacity(r.width() * r.height() * 4);
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                out.extend_from_slice(&self.get(x, y));
            }
        }
        out
    }

    fn write(&mut self, r: Rect, bytes: &[u8]) {
        let mut i = 0;
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                self.set(x, y, [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
                i += 4;
            }
        }
    }
}

/// Bilinear sample at point `p` (pixel centres at `+0.5`) of the pixels
/// `get` returns, colour weighted by alpha so clear pixels do not darken
/// edges; alpha comes back `0..=1`.
pub fn bilinear(p: P2, get: impl Fn(i64, i64) -> [u8; 4]) -> [f32; 4] {
    let (x, y) = (p.0 - 0.5, p.1 - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let mut acc = [0f32; 4];
    for (dx, dy, w) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let px = get(x0 + dx, y0 + dy);
        let a = px[3] as f32 / 255.0 * w;
        for c in 0..3 {
            acc[c] += px[c] as f32 * a;
        }
        acc[3] += a;
    }
    let total = acc[3];
    if total > 1e-6 {
        for c in acc.iter_mut().take(3) {
            *c /= total;
        }
    }
    acc
}

/// A rect's bytes from before (or after) an edit. Swapping them with the
/// canvas undoes the edit and leaves the step ready to redo it.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelStep {
    pub rect: Rect,
    bytes: Arc<Vec<u8>>,
}

impl PixelStep {
    /// The current contents of `rect` (clipped to the layer).
    pub fn capture(canvas: &Canvas, rect: Rect) -> Self {
        let rect = rect.intersect(canvas.bounds());
        PixelStep {
            rect,
            bytes: Arc::new(canvas.read(rect)),
        }
    }

    pub fn from_bytes(rect: Rect, bytes: Vec<u8>) -> Self {
        PixelStep {
            rect,
            bytes: Arc::new(bytes),
        }
    }

    pub fn swap(&mut self, canvas: &mut Canvas) {
        let current = canvas.read(self.rect);
        canvas.write(self.rect, &self.bytes);
        self.bytes = Arc::new(current);
    }
}
