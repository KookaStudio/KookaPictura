//! Filter ▸ Vanishing Point (#349, `FILT-102`): perspective planes and
//! Render Grids. Adobe's plane solver is closed; planes here lift to 3D
//! through a pinhole camera fitted to each root plane's vanishing points
//! (`geometry`). *(approximation: behavioural parity only)*
//!
//! ponytail: the Marquee, Stamp, Brush, Transform and Eyedropper tools and
//! Paste are not here yet, and plane data is not saved in files.

mod editor;
mod geometry;
mod planes;
mod render;
#[cfg(test)]
mod tests;

pub use editor::{DragMode, VpEditor};
pub use geometry::{Validity as VpValidity, P2};
pub use planes::{Part as VpPart, VpPlanes, DEFAULT_GRID as VP_DEFAULT_GRID};

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

/// What a committed session does to the layer, in layer pixels: for now the
/// segments Render Grids draws.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VpEdit {
    pub lines: Vec<[P2; 2]>,
}

impl VpEdit {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Apply a committed Vanishing Point session: draw its grid lines one pixel
/// wide, antialiased, in black over the layer.
// ponytail: CS6's rendered grid colour and weight are not confirmed; black at
// one pixel is assumed.
pub fn vanishing_point(buf: &mut PixelBuffer, edit: &VpEdit) -> Result<(), FilterError> {
    validate(buf)?;
    if edit.is_empty() {
        return Ok(());
    }
    let coverage = render::coverage(buf.width as usize, buf.height as usize, &edit.lines);
    render::darken(buf, &coverage);
    Ok(())
}
