//! The document's composite: whether it is the picture itself, and the
//! allocation-free composite a New Document starts with (#1216).

use crate::{BitDepth, ColorMode, Document, DocumentStateError, PixelBuffer, Plane};

/// Storage and merged-image output are independent. An empty plane represents
/// detached history metadata or an open that skipped the merged image.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeState {
    buffer: PixelBuffer,
    merged_output: bool,
    signed_merged_alpha: bool,
}

impl CompositeState {
    pub fn new(buffer: PixelBuffer) -> Result<Self, DocumentStateError> {
        Self::validate(&buffer)?;
        Ok(Self {
            buffer,
            merged_output: true,
            signed_merged_alpha: false,
        })
    }

    fn validate(buffer: &PixelBuffer) -> Result<(), DocumentStateError> {
        let expected = (buffer.width as usize)
            .checked_mul(buffer.height as usize)
            .and_then(|n| n.checked_mul(usize::from(buffer.channels)))
            .ok_or(DocumentStateError("composite geometry overflow"))?;
        if !buffer.data.is_empty() && buffer.data.len() != expected {
            return Err(DocumentStateError(
                "composite data length does not match geometry",
            ));
        }
        Ok(())
    }

    pub fn buffer(&self) -> &PixelBuffer {
        &self.buffer
    }
    pub fn into_buffer(self) -> PixelBuffer {
        self.buffer
    }
    pub fn merged_output(&self) -> bool {
        self.merged_output
    }
    pub fn set_merged_output(&mut self, enabled: bool) {
        self.merged_output = enabled;
    }

    /// Whether a decoded PSD identified its first extra plane as merged alpha.
    pub fn signed_merged_alpha(&self) -> bool {
        self.signed_merged_alpha
    }
    pub fn set_signed_merged_alpha(&mut self, present: bool) {
        self.signed_merged_alpha = present;
    }

    pub fn replace(&mut self, buffer: PixelBuffer) -> Result<(), DocumentStateError> {
        Self::validate(&buffer)?;
        self.buffer = buffer;
        Ok(())
    }

    pub fn replace_data(&mut self, data: Plane<u8>) -> Result<(), DocumentStateError> {
        self.replace(PixelBuffer {
            data,
            ..self.buffer.clone()
        })
    }

    pub fn edit<R>(
        &mut self,
        edit: impl FnOnce(&mut PixelBuffer) -> R,
    ) -> Result<R, DocumentStateError> {
        let mut candidate = self.buffer.clone();
        let result = edit(&mut candidate);
        self.replace(candidate)?;
        Ok(result)
    }

    pub fn edit_data<R>(
        &mut self,
        edit: impl FnOnce(&mut Plane<u8>) -> R,
    ) -> Result<R, DocumentStateError> {
        self.edit(|buffer| edit(&mut buffer.data))
    }

    /// Edit a fixed-length contiguous view without cloning a uniquely owned buffer.
    pub fn edit_data_slice<R>(&mut self, edit: impl FnOnce(&mut [u8]) -> R) -> R {
        edit(self.buffer.data.as_mut_slice())
    }

    /// Keep geometry and the history traversal's composite slot when detaching.
    pub fn detach_plane(&mut self) -> Plane<u8> {
        std::mem::take(&mut self.buffer.data)
    }

    pub fn attach_plane(&mut self, data: Plane<u8>) -> Result<(), DocumentStateError> {
        self.replace_data(data)
    }
}

impl std::ops::Deref for CompositeState {
    type Target = PixelBuffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl Default for CompositeState {
    fn default() -> Self {
        Self {
            buffer: PixelBuffer::new(0, 0, 3),
            merged_output: true,
            signed_merged_alpha: false,
        }
    }
}

impl PartialEq<PixelBuffer> for CompositeState {
    fn eq(&self, other: &PixelBuffer) -> bool {
        self.buffer == *other
    }
}

impl PartialEq<CompositeState> for PixelBuffer {
    fn eq(&self, other: &CompositeState) -> bool {
        *self == other.buffer
    }
}

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
            || self.source.depth().is_some()
            || self.source.planes().is_some()
    }

    pub fn has_stored_composite(&self) -> bool {
        self.composite.width == self.width
            && self.composite.height == self.height
            && !self.composite.data.is_empty()
    }

    pub fn composite_shape_is_valid(&self) -> bool {
        self.mode == ColorMode::Rgb
            && self.composite.channels == 4
            && self.composite.width == self.width
            && self.composite.height == self.height
    }

    pub fn document_color_mode(&self) -> ColorMode {
        self.source.mode().unwrap_or(self.mode)
    }

    pub fn document_bit_depth(&self) -> BitDepth {
        if self.source.mode() == Some(ColorMode::Bitmap) {
            return BitDepth::One;
        }
        if self.retains_source_depth() {
            self.source.depth().unwrap_or(BitDepth::Eight)
        } else {
            BitDepth::Eight
        }
    }
}
