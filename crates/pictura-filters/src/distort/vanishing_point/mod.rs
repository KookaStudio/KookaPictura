//! Filter ▸ Vanishing Point (#349, `FILT-102`): perspective planes, Render
//! Grids, and editing in perspective (Paste, Marquee, Transform, Stamp,
//! Brush, Eyedropper). Adobe's plane solver and retouch blends are closed;
//! planes here lift to 3D through a pinhole camera fitted to each root
//! plane's vanishing points (`geometry`), and edits live in each family's
//! unfolded flat space (`space`). *(approximation: behavioural parity only)*
//!
//! ponytail: plane data is not saved in files (#918).

mod canvas;
mod editor;
mod float;
mod geometry;
mod heal;
mod persist;
mod planes;
mod render;
mod space;
mod stroke;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_retouch;
mod tools;
mod view;

use std::sync::Arc;

pub use canvas::{Canvas as VpCanvas, Rect as VpRect};
pub use editor::{DragMode, VpEditor};
pub use float::Texture as VpTexture;
pub use geometry::{Validity as VpValidity, P2};
pub use heal::HealMode as VpHeal;
pub use planes::{Part as VpPart, VpPlanes, DEFAULT_GRID as VP_DEFAULT_GRID};
pub use stroke::PaintOptions as VpPaintOptions;
pub use tools::{MarqueeOptions as VpMarqueeOptions, PasteError as VpPasteError};

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

/// Replacement pixels for a rect of the layer, straight RGBA.
#[derive(Clone, Debug, PartialEq)]
pub struct VpPatch {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub rgba: Arc<Vec<u8>>,
}

/// What a committed session does to the layer, in layer pixels: the
/// session's changed pixels, then the segments Render Grids draws.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VpEdit {
    pub patch: Option<VpPatch>,
    pub lines: Vec<[P2; 2]>,
}

impl VpEdit {
    pub fn is_empty(&self) -> bool {
        self.patch.is_none() && self.lines.is_empty()
    }
}

/// Apply a committed Vanishing Point session: write its changed pixels,
/// then draw its grid lines one pixel wide, antialiased, in black.
// ponytail: CS6's rendered grid colour and weight are not confirmed; black at
// one pixel is assumed. A patch replayed on a layer of another size lands at
// the same layer offset, clipped.
pub fn vanishing_point(buf: &mut PixelBuffer, edit: &VpEdit) -> Result<(), FilterError> {
    validate(buf)?;
    if let Some(patch) = &edit.patch {
        write_patch(buf, patch);
    }
    if !edit.lines.is_empty() {
        let coverage = render::coverage(buf.width as usize, buf.height as usize, &edit.lines);
        render::darken(buf, &coverage);
    }
    Ok(())
}

fn write_patch(buf: &mut PixelBuffer, patch: &VpPatch) {
    let (w, h) = (buf.width as usize, buf.height as usize);
    let n = w * h;
    let alpha = buf.channels == 4;
    let data = buf.data.as_mut_slice();
    for py in 0..patch.height {
        let y = patch.y + py;
        if y >= h {
            break;
        }
        for px in 0..patch.width {
            let x = patch.x + px;
            if x >= w {
                break;
            }
            let s = (py * patch.width + px) * 4;
            let d = y * w + x;
            for c in 0..3 {
                data[c * n + d] = patch.rgba[s + c];
            }
            if alpha {
                data[3 * n + d] = patch.rgba[s + 3];
            }
        }
    }
}
