//! Placing a tiled plane inside a larger one (#1609).

use super::{Tile, TileSample, TiledPlane, TILE};

impl<T: TileSample> TiledPlane<T> {
    /// This plane placed at `(x0, y0)` in an otherwise empty `new_w × new_h`
    /// plane with as many channels. The work is per tile that holds something:
    /// an empty tile costs nothing, and a tile that lands exactly on a tile of
    /// the new grid (a whole-tile offset, and the same clipped size there) is
    /// shared rather than copied. A layer growing right or down therefore
    /// copies at most its old right and bottom edge tiles.
    pub(crate) fn padded(&self, new_w: usize, new_h: usize, x0: usize, y0: usize) -> Self {
        let mut out = Self::filled(new_w, new_h, self.channels, T::default());
        let aligned = x0.is_multiple_of(TILE) && y0.is_multiple_of(TILE);
        let mut buf = Vec::new();
        for (i, tile) in self.tiles.iter().enumerate() {
            if matches!(tile, Tile::Empty) {
                continue;
            }
            let (x, gy, w, h) = self.tile_rect(i);
            let (c, y) = (gy / self.height, gy % self.height);
            let (nx, ny) = (x0 + x, y0 + y);
            if aligned {
                let j = (c * out.rows as usize + ny / TILE) * out.cols as usize + nx / TILE;
                let (_, _, nw, nh) = out.tile_rect(j);
                if (nw, nh) == (w, h) {
                    out.tiles[j] = tile.clone();
                    out.gens[j] = self.gens[i];
                    out.dirty[j] = self.dirty[i];
                    continue;
                }
            }
            let src = self.tile_ref(i);
            buf.resize(w * h, T::default());
            for (r, row) in buf.chunks_mut(w).enumerate() {
                src.copy_row(r, 0..w, row);
            }
            drop(src);
            let top = c * new_h + ny;
            out.write_rect_from(nx..nx + w, top..top + h, &buf);
        }
        out.settle();
        out
    }
}
