//! Source representation and profile provenance, changed as checked values.

use crate::{BitDepth, ColorMode, Plane, Samples, SourcePlanes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentStateError(pub(crate) &'static str);

impl std::fmt::Display for DocumentStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for DocumentStateError {}

/// Recorded provenance is independent of the current working pixels. Native
/// stores can be absent while layers still retain samples at the recorded depth.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceState {
    mode: Option<ColorMode>,
    depth: Option<BitDepth>,
    planes: Option<SourcePlanes>,
    palette: Option<[u8; 768]>,
    multichannel_plates: bool,
    original_profile: Option<Plane<u8>>,
    working_profile: Option<Plane<u8>>,
}

impl SourceState {
    pub fn new(
        mode: Option<ColorMode>,
        depth: Option<BitDepth>,
        planes: Option<SourcePlanes>,
        palette: Option<[u8; 768]>,
    ) -> Result<Self, DocumentStateError> {
        let state = Self {
            mode,
            depth,
            planes,
            palette,
            ..Self::default()
        };
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<(), DocumentStateError> {
        if (self.mode == Some(ColorMode::Indexed)) != self.palette.is_some() {
            return Err(DocumentStateError(
                "Indexed source mode requires its palette",
            ));
        }
        if let Some(store) = &self.planes {
            let matches = matches!(
                (&store.samples, store.depth),
                (Samples::U8(_), BitDepth::One | BitDepth::Eight)
                    | (Samples::U16(_), BitDepth::Sixteen)
                    | (Samples::F32(_), BitDepth::ThirtyTwo)
            );
            if !matches {
                return Err(DocumentStateError(
                    "source sample representation does not match depth",
                ));
            }
            if self.depth.is_some_and(|depth| depth != store.depth) {
                return Err(DocumentStateError(
                    "retained source depth does not match recorded depth",
                ));
            }
        }
        Ok(())
    }

    pub fn mode(&self) -> Option<ColorMode> {
        self.mode
    }
    /// Whether mode 7 stores every file plane in the document channel list.
    pub fn multichannel_plates(&self) -> bool {
        self.multichannel_plates
    }

    pub fn set_multichannel_plates(&mut self) {
        self.mode = Some(ColorMode::Multichannel);
        self.palette = None;
        self.multichannel_plates = true;
    }

    pub fn depth(&self) -> Option<BitDepth> {
        self.depth
    }
    pub fn planes(&self) -> &Option<SourcePlanes> {
        &self.planes
    }
    pub fn palette(&self) -> Option<[u8; 768]> {
        self.palette
    }
    pub fn palette_ref(&self) -> Option<&[u8; 768]> {
        self.palette.as_ref()
    }
    pub fn original_profile(&self) -> &Option<Plane<u8>> {
        &self.original_profile
    }
    pub fn working_profile(&self) -> &Option<Plane<u8>> {
        &self.working_profile
    }

    /// Mode and palette form one representation; neither is published alone.
    pub fn set_encoding(
        &mut self,
        mode: Option<ColorMode>,
        palette: Option<[u8; 768]>,
    ) -> Result<(), DocumentStateError> {
        if (mode == Some(ColorMode::Indexed)) != palette.is_some() {
            return Err(DocumentStateError(
                "Indexed source mode requires its palette",
            ));
        }
        self.mode = mode;
        self.palette = palette;
        self.multichannel_plates = false;
        Ok(())
    }

    /// Change the recorded depth and retained planes together. Layer stores
    /// are checked against this provenance when they are consumed.
    pub fn set_native(
        &mut self,
        depth: Option<BitDepth>,
        planes: Option<SourcePlanes>,
    ) -> Result<(), DocumentStateError> {
        let candidate = Self {
            depth,
            planes,
            ..self.clone()
        };
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn replace_planes(
        &mut self,
        planes: Option<SourcePlanes>,
    ) -> Result<(), DocumentStateError> {
        self.set_native(self.depth, planes)
    }

    /// Replace samples without changing the store's geometry or representation.
    pub fn replace_samples(&mut self, samples: Samples) -> Result<(), DocumentStateError> {
        let Some(store) = self.planes.as_mut() else {
            return Err(DocumentStateError("no source planes"));
        };
        let same_variant = matches!(
            (&store.samples, &samples),
            (Samples::U8(_), Samples::U8(_))
                | (Samples::U16(_), Samples::U16(_))
                | (Samples::F32(_), Samples::F32(_))
        );
        if !same_variant || store.samples.len() != samples.len() {
            return Err(DocumentStateError(
                "source sample shape or representation changed",
            ));
        }
        store.samples = samples;
        Ok(())
    }

    pub fn set_original_profile(&mut self, profile: Option<Plane<u8>>) {
        self.original_profile = profile;
    }
    pub fn set_working_profile(&mut self, profile: Option<Plane<u8>>) {
        self.working_profile = profile;
    }
}

#[cfg(test)]
mod tests;
