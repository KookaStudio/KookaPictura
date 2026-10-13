//! Pixel translation of a selection mask.

use crate::Selection;

impl Selection {
    /// A new document-sized mask with each pixel's coverage moved by `(dx, dy)`.
    ///
    /// Clipped to the canvas: pixels shifted outside are dropped and vacated
    /// pixels become zero. `(0, 0)` is the identity.
    pub fn translate(&self, dx: i32, dy: i32) -> Selection {
        if dx == 0 && dy == 0 {
            return self.clone();
        }
        let (w, h) = (self.width as i64, self.height as i64);
        let mut out = Selection::none(self.width, self.height);
        // Destination columns whose source column is on the canvas.
        let (x0, x1) = ((dx as i64).clamp(0, w), (w + dx as i64).clamp(0, w));
        if x0 >= x1 {
            return out;
        }
        for y in 0..h {
            let sy = y - dy as i64;
            if sy < 0 || sy >= h {
                continue;
            }
            let (dst, src) = ((y * w) as usize, (sy * w) as usize);
            let (sx0, sx1) = ((x0 - dx as i64) as usize, (x1 - dx as i64) as usize);
            out.data[dst + x0 as usize..dst + x1 as usize]
                .copy_from_slice(&self.data[src + sx0..src + sx1]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Selection {
        let mut s = Selection::none(w, h);
        for y in y0..y1 {
            for x in x0..x1 {
                s.data[(y * w + x) as usize] = 255;
            }
        }
        s
    }

    #[test]
    fn zero_is_identity() {
        let a = rect(6, 6, 1, 1, 4, 3);
        assert_eq!(a.translate(0, 0), a);
    }

    #[test]
    fn shifts_right_and_down() {
        let a = rect(6, 6, 1, 1, 3, 3);
        let t = a.translate(2, 1);
        assert_eq!(t.data[(2 * 6 + 3) as usize], 255);
        assert_eq!(t.data[(3 * 6 + 4) as usize], 255);
        assert_eq!(t.data[7], 0, "vacated pixel is cleared");
        assert_eq!(t.data.iter().filter(|&&v| v > 0).count(), 4);
    }

    #[test]
    fn clips_at_the_edge() {
        let a = rect(8, 8, 0, 0, 4, 4);
        let t = a.translate(-3, -3);
        assert_eq!(t.data.iter().filter(|&&v| v > 0).count(), 1);
        assert_eq!(t.data[0], 255);
        assert_eq!(
            a.translate(10, 10).data.iter().filter(|&&v| v > 0).count(),
            0
        );
    }
}
