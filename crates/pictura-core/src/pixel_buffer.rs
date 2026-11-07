//! The planar pixel buffer a document composite is stored in.

use crate::{Plane, TileSample};

/// A planar, row-major pixel buffer holding one sample type per channel.
///
/// `data.len() == width * height * channels`. Planar means channel `c` for the
/// whole image comes first, matching the PSD image-data layout. A bare
/// `PixelBuffer` is `PixelBuffer<u8>`, so 8-bit code is unchanged.
#[derive(Debug, Clone)]
pub struct PixelBuffer<T = u8> {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub data: Plane<T>,
}

impl PixelBuffer<u8> {
    /// As [`PixelBuffer::new`] without touching the memory, so a mostly empty
    /// buffer costs only the pages written to it.
    pub fn zeroed(width: u32, height: u32, channels: u8) -> Self {
        let len = width as usize * height as usize * usize::from(channels);
        Self {
            width,
            height,
            channels,
            data: Plane::build(len, |_| {}),
        }
    }
}

impl<T: Clone + Default> PixelBuffer<T> {
    pub fn new(width: u32, height: u32, channels: u8) -> Self {
        Self {
            width,
            height,
            channels,
            data: vec![T::default(); width as usize * height as usize * channels as usize].into(),
        }
    }
}

impl<T> PixelBuffer<T> {
    /// Number of pixels (not samples).
    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

impl<T: TileSample> PartialEq for PixelBuffer<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.width, self.height, self.channels) == (other.width, other.height, other.channels)
            && self.data == other.data
    }
}

impl<T: TileSample + Eq> Eq for PixelBuffer<T> {}
