//! Shared pyramid result storage and production constants.

use std::sync::Arc;

/// Levels stop halving once the next would be smaller than this on its long side.
pub const SMALLEST_SIDE: u32 = 256;

/// The tile grid a damage update is rounded out to, in level-0 pixels. A dirty
/// rectangle grows outward to a whole number of tiles before the level walk, so
/// adjacent dabs of a frame share tiles and the pyramid is repaired on one
/// stable grid. `level0` itself is never touched here: the caller owns it.
pub const TILE: i32 = 64;

/// One stored pyramid level: interleaved premultiplied RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyramidLevel {
    width: u32,
    height: u32,
    data: Arc<Vec<u8>>,
}

impl PyramidLevel {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: Arc::new(vec![0u8; width as usize * height as usize * 4]),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Interleaved premultiplied RGBA, row-major.
    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// The level's own bytes for writing, copied first when a view shares them.
    pub(crate) fn data_mut(&mut self) -> &mut [u8] {
        Arc::make_mut(&mut self.data).as_mut_slice()
    }
}
