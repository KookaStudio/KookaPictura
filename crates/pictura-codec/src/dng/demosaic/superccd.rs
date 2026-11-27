//! Turns a demosaiced SuperCCD grid upright (#1621), sampling it bilinearly
//! along the sensor's 45° axes.
//!
//! Ported to Rust from LibRaw 0.22.1's `fuji_rotate`
//! (`src/postprocessing/aspect_ratio.cpp`, <https://github.com/LibRaw/LibRaw>,
//! commit b860248a89d9082b8e0a1e202e516f46af9adb29). Not a clean-room
//! implementation: the owner chose an open, attributed port.
//!
//! Copyright 2019-2025 LibRaw LLC; LibRaw uses code from dcraw.c, copyright
//! 1997-2018 Dave Coffin.
//!
//! LibRaw is licensed under the GNU Lesser General Public License, version
//! 2.1, or the Common Development and Distribution License, version 1.0.
//! This port takes the LGPL-2.1 and, as its section 3 permits, is distributed
//! under the GNU General Public License, version 3 or later, like the rest of
//! Kooka Pictura. See `assets/PROVENANCE.md`.

use rayon::prelude::*;

/// LibRaw's `step`: the grid's cells are `√½` of an upright pixel apart.
const STEP: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// The upright size of a grid `grid_height` high with LibRaw's `fuji_width`
/// `fw`; `None` when the grid cannot hold it.
pub fn rotated_size(fw: usize, grid_height: usize) -> Option<(usize, usize)> {
    let f = fw.checked_sub(1)?;
    let rest = grid_height.checked_sub(f)?;
    let (wide, high) = ((f as f64 / STEP) as usize, (rest as f64 / STEP) as usize);
    (wide > 0 && high > 0).then_some((wide, high))
}

/// `rgb` (`width × height × 3`) turned upright into `wide × high × 3`.
pub(super) fn rotate(
    rgb: &[u16],
    (width, height): (usize, usize),
    fw: usize,
    (wide, high): (usize, usize),
) -> Vec<u16> {
    let f = (fw - 1) as f64;
    let mut out = vec![0u16; wide * high * 3];
    out.par_chunks_mut(wide * 3)
        .enumerate()
        .for_each(|(row, dst)| {
            for (col, px) in dst.as_chunks_mut::<3>().0.iter_mut().enumerate() {
                // LibRaw computes the position in double and keeps it as float.
                let r = (f + (row as f64 - col as f64) * STEP) as f32;
                let c = ((row + col) as f64 * STEP) as f32;
                if r < 0.0 {
                    continue;
                }
                let (ur, uc) = (r as usize, c as usize);
                if ur + 2 > height || uc + 2 > width {
                    continue;
                }
                let (fr, fc) = (r - ur as f32, c - uc as f32);
                let at = |y: usize, x: usize, i: usize| f32::from(rgb[(y * width + x) * 3 + i]);
                for (i, p) in px.iter_mut().enumerate() {
                    let top = at(ur, uc, i) * (1.0 - fc) + at(ur, uc + 1, i) * fc;
                    let bottom = at(ur + 1, uc, i) * (1.0 - fc) + at(ur + 1, uc + 1, i) * fc;
                    *p = (top * (1.0 - fr) + bottom * fr) as u16;
                }
            }
        });
    out
}
