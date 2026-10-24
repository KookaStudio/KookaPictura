//! The dialog's preview: the document with the active layer hidden, the
//! working copy of the layer over it, and the floating selection over that,
//! composed for one document rect at a time.
//!
//! ponytail: the layer is composed Normal at full opacity; the commit, which
//! edits the layer itself, honours its blend mode and opacity.

use super::canvas::Rect;
use super::editor::VpEditor;
use super::float;
use super::geometry::P2;

fn over(dst: [f32; 4], src: [f32; 3], a: f32) -> [f32; 4] {
    let out = a + dst[3] * (1.0 - a);
    if out <= 0.0 {
        return [0.0; 4];
    }
    let mut px = [0f32; 4];
    for c in 0..3 {
        px[c] = (src[c] * a + dst[c] * dst[3] * (1.0 - a)) / out;
    }
    px[3] = out;
    px
}

impl VpEditor {
    /// Straight RGBA for document rect `r` (clipped to the document), and
    /// the rect it covers.
    pub fn compose(&self, r: Rect) -> (Rect, Vec<u8>) {
        let (dw, dh) = self.state.planes.size();
        let r = r.intersect(Rect {
            x0: 0,
            y0: 0,
            x1: dw as i32,
            y1: dh as i32,
        });
        let (w, h) = (r.width(), r.height());
        let mut px = vec![[0f32; 4]; w * h];
        if let Some(retouch) = &self.retouch {
            for y in 0..h {
                for x in 0..w {
                    let (dx, dy) = (r.x0 + x as i32, r.y0 + y as i32);
                    let base = retouch
                        .backdrop
                        .as_ref()
                        .map_or([255.0, 255.0, 255.0, 1.0], |b| {
                            let i = (dy as usize * dw as usize + dx as usize) * 4;
                            [
                                b[i] as f32,
                                b[i + 1] as f32,
                                b[i + 2] as f32,
                                b[i + 3] as f32 / 255.0,
                            ]
                        });
                    let l = retouch.canvas.get(dx, dy);
                    px[y * w + x] = over(
                        base,
                        [l[0] as f32, l[1] as f32, l[2] as f32],
                        l[3] as f32 / 255.0,
                    );
                }
            }
            if let Some(f) = &self.state.float {
                if let Some(map) = self.map_of(f.family) {
                    let foot = float::footprint(&map, &f.flat_corners(), Some(f.home)).intersect(r);
                    if !foot.is_empty() {
                        let o = retouch.marquee;
                        let (src, mask) = float::render(&map, f, o.feather, o.opacity, foot);
                        let fw = foot.width();
                        for y in 0..foot.height() {
                            for x in 0..fw {
                                let a = mask[y * fw + x] * src[y * fw + x][3];
                                if a <= 0.0 {
                                    continue;
                                }
                                let s = src[y * fw + x];
                                let i = (foot.y0 - r.y0) as usize * w
                                    + y * w
                                    + (foot.x0 - r.x0) as usize
                                    + x;
                                px[i] = over(px[i], [s[0], s[1], s[2]], a);
                            }
                        }
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(w * h * 4);
        for p in px {
            out.extend_from_slice(&[
                p[0].round().clamp(0.0, 255.0) as u8,
                p[1].round().clamp(0.0, 255.0) as u8,
                p[2].round().clamp(0.0, 255.0) as u8,
                (p[3] * 255.0).round().clamp(0.0, 255.0) as u8,
            ]);
        }
        (r, out)
    }

    /// The Eyedropper: the colour the preview shows at document point `p`.
    pub fn sample(&self, p: P2) -> Option<[u8; 3]> {
        let (x, y) = (p.0.floor() as i32, p.1.floor() as i32);
        let (r, px) = self.compose(Rect {
            x0: x,
            y0: y,
            x1: x + 1,
            y1: y + 1,
        });
        (!r.is_empty()).then(|| [px[0], px[1], px[2]])
    }
}
