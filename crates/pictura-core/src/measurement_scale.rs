use std::sync::Arc;

#[cfg(test)]
mod tests;

/// A mapping between a pixel distance and a distance in a user-supplied unit.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementScale {
    pixel_length: i32,
    logical_length: f64,
    units: Arc<str>,
}

impl MeasurementScale {
    pub fn new(pixel_length: i32, logical_length: f64, units: &str) -> Option<Self> {
        let factor = logical_length / f64::from(pixel_length);
        if pixel_length <= 0 || !factor.is_finite() || factor <= 0.0 || units.trim().is_empty() {
            return None;
        }
        Some(Self {
            pixel_length,
            logical_length,
            units: Arc::from(units),
        })
    }

    pub fn pixel_length(&self) -> i32 {
        self.pixel_length
    }

    pub fn logical_length(&self) -> f64 {
        self.logical_length
    }

    pub fn units(&self) -> &str {
        &self.units
    }

    pub fn logical_distance(&self, pixels: f64) -> f64 {
        pixels * (self.logical_length / f64::from(self.pixel_length))
    }

    pub fn logical_area(&self, square_pixels: f64) -> f64 {
        self.logical_distance(self.logical_distance(square_pixels))
    }
}

impl Default for MeasurementScale {
    fn default() -> Self {
        Self {
            pixel_length: 1,
            logical_length: 1.0,
            units: Arc::from("px"),
        }
    }
}

/// The active calibration and whether saving should replay the original resource.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum DocumentMeasurementScale {
    #[default]
    Absent,
    Opaque,
    Imported(MeasurementScale),
    Applied(MeasurementScale),
    Reset,
    Resampled,
    OpaqueResampled,
}

impl DocumentMeasurementScale {
    pub fn custom(&self) -> Option<&MeasurementScale> {
        match self {
            Self::Imported(scale) | Self::Applied(scale) => Some(scale),
            _ => None,
        }
    }

    pub fn apply(&mut self, scale: MeasurementScale) -> bool {
        if self.custom() == Some(&scale) {
            return false;
        }
        *self = Self::Applied(scale);
        true
    }

    pub fn reset(&mut self) -> bool {
        if matches!(self, Self::Absent | Self::Reset | Self::Resampled) {
            return false;
        }
        *self = Self::Reset;
        true
    }

    /// Opaque bytes stay available for round-tripping even though a resize can
    /// make the calibration they carry invalid in a reader that recognizes it.
    pub fn invalidate_for_resample(&mut self) {
        match self {
            Self::Imported(_) | Self::Applied(_) => *self = Self::Resampled,
            Self::Opaque => *self = Self::OpaqueResampled,
            _ => {}
        }
    }

    pub fn needs_recalibration(&self) -> bool {
        matches!(self, Self::Resampled | Self::OpaqueResampled)
    }
}
