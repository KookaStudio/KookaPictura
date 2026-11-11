//! The document's composite: whether it is the picture itself, and the
//! allocation-free composite a New Document starts with (#1216).

use crate::{BitDepth, ColorMode, Document, PixelBuffer, Plane};

impl Document {
    /// As [`Document::new`] with a composite of uniform `value` tiles, which
    /// hold no allocation: a New Document's (#1213, #1216).
    pub fn with_uniform_composite(
        width: u32,
        height: u32,
        mode: ColorMode,
        depth: BitDepth,
        value: u8,
    ) -> Self {
        let channels = mode.color_channels();
        let composite = PixelBuffer {
            width,
            height,
            channels,
            data: Plane::filled(
                width as usize,
                height as usize,
                usize::from(channels),
                value,
            ),
        };
        Self::with_composite(width, height, mode, depth, composite)
    }

    /// Whether `composite` is the document's own picture rather than a
    /// derivative of its layers (#1216): a layerless document's embedded
    /// image, a non-RGB mode's colour planes, or a retained native-depth or
    /// merged composite a save re-emits. Only these keep a stored composite;
    /// every other document's composite is computed on demand.
    pub fn composite_is_authoritative(&self) -> bool {
        self.layers.is_empty()
            || self.mode != ColorMode::Rgb
            || self.depth != BitDepth::Eight
            || self.source_depth.is_some()
            || self.source_planes.is_some()
    }
}
