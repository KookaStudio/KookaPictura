//! SuperCCD RAFs (#1621): photosites on a 45°-turned lattice, repacked onto
//! the rectangular grid LibRaw demosaics, which `demosaic` later turns
//! upright.
//!
//! Ported to Rust from LibRaw 0.22.1 (<https://github.com/LibRaw/LibRaw>,
//! commit b860248a89d9082b8e0a1e202e516f46af9adb29): the `0x0121` size of
//! `parse_fuji` (`src/metadata/fuji.cpp`), the active area and `fuji_width`
//! of `identify` (`src/metadata/identify.cpp`) and the repack of
//! `raw2image` (`src/preprocessing/raw2image.cpp`). Not a clean-room
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

use super::CfaPattern;
use crate::error::PsdError;

/// The upright size record (height, width).
pub(super) const UPRIGHT_SIZE: u16 = 0x0121;

/// The part of the plane that is repacked, and the orientation LibRaw gives
/// a turned plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ActiveArea {
    pub top: usize,
    pub left: usize,
    pub width: usize,
    pub height: usize,
    pub orientation: Option<u16>,
}

/// The repacked grid, raw samples with 0 outside the lattice.
pub(super) struct Grid {
    pub width: usize,
    pub height: usize,
    pub fuji_width: usize,
    pub samples: Vec<u16>,
}

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("RAF SuperCCD: {what}"))
}

/// The active area of a `plane_width × plane_height` plane. `shift` is
/// LibRaw's `fuji_layout`, `inset` the directory's crop (top, left, height,
/// width), `upright` record `0x0121`, `generation` whether a `0xC000`
/// record is present.
pub(super) fn active_area(
    (plane_width, plane_height): (usize, usize),
    shift: bool,
    inset: Option<[usize; 4]>,
    upright: Option<(usize, usize)>,
    generation: bool,
) -> Result<ActiveArea, PsdError> {
    let centred = |height: usize, width: usize, orientation| ActiveArea {
        top: plane_height.saturating_sub(height) / 2,
        left: plane_width.saturating_sub(width) / 2,
        width,
        height,
        orientation,
    };
    if let Some([top, left, height, width]) = inset {
        if height > 0 && width > 0 && height <= plane_height && width <= plane_width {
            return Ok(ActiveArea {
                top,
                left,
                width,
                height,
                orientation: None,
            });
        }
    }
    // The S2 Pro stores its plane turned; LibRaw names its size outright.
    if plane_width == 2944 && !generation {
        return Ok(centred(2144, 2880, Some(6)));
    }
    let (height, mut width) = upright.ok_or_else(|| invalid("missing size record 0x0121"))?;
    if width == 4284 {
        width += 3;
    }
    Ok(centred(height << shift as u32, width >> shift as u32, None))
}

/// LibRaw's `fuji_width` and grid size for `area`.
pub(super) fn grid_size(area: &ActiveArea, shift: bool) -> Result<(usize, usize, usize), PsdError> {
    let fw = area.width >> (!shift) as u32;
    let width = (area.height >> shift as u32)
        .checked_add(fw)
        .filter(|&w| fw > 1 && w > 2)
        .ok_or_else(|| invalid("grid size"))?;
    Ok((fw, width, width - 1))
}

/// Repacks `plane` onto LibRaw's grid.
pub(super) fn repack(
    plane: &[u16],
    (plane_width, plane_height): (usize, usize),
    area: &ActiveArea,
    shift: bool,
) -> Result<Grid, PsdError> {
    let (fw, width, height) = grid_size(area, shift)?;
    let mut samples = vec![0u16; width * height];
    let rows = plane_height.saturating_sub(2 * area.top);
    let cols = fw << (!shift) as u32;
    for row in 0..rows {
        let src = &plane[(row + area.top) * plane_width..][..plane_width];
        for col in 0..cols.min(plane_width.saturating_sub(area.left)) {
            let (r, c) = if shift {
                (fw - 1 - col + (row >> 1), col + ((row + 1) >> 1))
            } else {
                (fw - 1 + row - (col >> 1), row + ((col + 1) >> 1))
            };
            if r < height && c < width {
                samples[r * width + c] = src[col + area.left];
            }
        }
    }
    Ok(Grid {
        width,
        height,
        fuji_width: fw,
        samples,
    })
}

/// The grid's CFA: LibRaw's `0x94949494` for an odd `fuji_width`, else
/// `0x49494949`.
pub(super) fn grid_cfa(fw: usize) -> CfaPattern {
    CfaPattern {
        width: 2,
        height: 2,
        colors: if fw % 2 == 1 {
            vec![0, 1, 1, 2]
        } else {
            vec![1, 2, 0, 1]
        },
    }
}
