//! Linearization and black/white normalization of the decoded sensor plane,
//! cropped to the active area.

use rayon::prelude::*;

pub(super) struct Levels {
    pub linearization: Option<Vec<u16>>,
    /// `BlackLevel`, `rows × cols`, row-major.
    pub black: Vec<f64>,
    pub black_rows: usize,
    pub black_cols: usize,
    /// Per active-area row and column; empty when absent.
    pub delta_v: Vec<f64>,
    pub delta_h: Vec<f64>,
    pub white: f64,
}

/// Active-area rectangle in raw-image coordinates.
#[derive(Clone, Copy)]
pub(super) struct Area {
    pub top: usize,
    pub left: usize,
    pub width: usize,
    pub height: usize,
}

/// Normalizes `raw` (`raw_width` wide) into a new active-area plane.
pub(super) fn normalize(raw: &[u16], raw_width: usize, area: Area, levels: &Levels) -> Vec<u16> {
    let mut out = vec![0u16; area.width * area.height];
    if area.width == 0 {
        return out;
    }
    out.par_chunks_mut(area.width)
        .enumerate()
        .for_each(|(y, dst)| {
            let src = &raw[(area.top + y) * raw_width + area.left..][..area.width];
            let row_black =
                &levels.black[(y % levels.black_rows) * levels.black_cols..][..levels.black_cols];
            let delta_v = levels.delta_v.get(y).copied().unwrap_or(0.0);
            for (x, (d, &v)) in dst.iter_mut().zip(src).enumerate() {
                let v = match &levels.linearization {
                    Some(table) => table[(v as usize).min(table.len() - 1)],
                    None => v,
                } as f64;
                let black = row_black[x % levels.black_cols]
                    + delta_v
                    + levels.delta_h.get(x).copied().unwrap_or(0.0);
                let range = levels.white - black;
                let n = if range > 0.0 {
                    ((v - black) * 65535.0 / range).round()
                } else if v >= levels.white {
                    65535.0
                } else {
                    0.0
                };
                *d = n.clamp(0.0, 65535.0) as u16;
            }
        });
    out
}
