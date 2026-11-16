//! The Photoshop CS6 edition whose surface the process presents.
//!
//! One build serves both editions: `PICTURA_EDITION` selects the surface at
//! startup, and the Extended-only tools and commands are hidden in Standard.
//! The gate is presentation-only; document data is never dropped.

/// The edition whose feature surface the application presents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Edition {
    Standard,
    #[default]
    Extended,
}

impl Edition {
    /// Parse a `PICTURA_EDITION` value. Case-insensitive; `None` for an
    /// unrecognized value.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "standard" | "std" => Some(Self::Standard),
            "extended" | "ext" => Some(Self::Extended),
            _ => None,
        }
    }

    /// The edition named by `PICTURA_EDITION`, or Extended when the variable
    /// is unset or unknown.
    pub fn from_env() -> Self {
        std::env::var("PICTURA_EDITION")
            .ok()
            .and_then(|value| Self::parse(&value))
            .unwrap_or_default()
    }

    /// Whether this edition exposes `feature`.
    pub fn allows(self, feature: Feature) -> bool {
        self == Edition::Extended || feature.edition() == Edition::Standard
    }

    pub const fn is_extended(self) -> bool {
        matches!(self, Self::Extended)
    }
}

/// A CS6 capability whose availability differs by edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// 3D tools, menus, panels and layers (Extended only).
    ThreeD,
    /// Measurement, counting, the Measurement Log and data export (Extended only).
    Measurement,
    /// DICOM import and analysis (Extended only).
    Dicom,
    /// 32-bit adjustment layers (Extended only).
    HdrAdjustments,
    /// Image-stack Stack Modes (Extended only).
    StackModes,
    /// Video and the Timeline, which CS6 ships in every edition.
    Timeline,
}

impl Feature {
    /// The edition that first offered this feature. `Timeline` is Standard
    /// because CS6 moved video out of the CS5 Extended-only gate.
    pub const fn edition(self) -> Edition {
        match self {
            Self::Timeline => Edition::Standard,
            Self::ThreeD
            | Self::Measurement
            | Self::Dicom
            | Self::HdrAdjustments
            | Self::StackModes => Edition::Extended,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_and_extended_parse_case_insensitively() {
        assert_eq!(Edition::parse("standard"), Some(Edition::Standard));
        assert_eq!(Edition::parse(" Standard "), Some(Edition::Standard));
        assert_eq!(Edition::parse("STD"), Some(Edition::Standard));
        assert_eq!(Edition::parse("extended"), Some(Edition::Extended));
        assert_eq!(Edition::parse("EXT"), Some(Edition::Extended));
    }

    #[test]
    fn unknown_value_defaults_to_extended() {
        assert_eq!(Edition::parse("trial"), None);
        assert_eq!(Edition::default(), Edition::Extended);
    }

    #[test]
    fn extended_only_features_require_extended() {
        for feature in [
            Feature::ThreeD,
            Feature::Measurement,
            Feature::Dicom,
            Feature::HdrAdjustments,
            Feature::StackModes,
        ] {
            assert!(!Edition::Standard.allows(feature));
            assert!(Edition::Extended.allows(feature));
        }
    }

    #[test]
    fn timeline_is_all_editions() {
        assert!(Edition::Standard.allows(Feature::Timeline));
        assert!(Edition::Extended.allows(Feature::Timeline));
    }
}
