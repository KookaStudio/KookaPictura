//! The adjustment and fill-layer kinds shared by the catalogue, defaults,
//! direct commands, presets, and GPU routing.

use crate::{Adjustment, AutoKind, FillContent};

macro_rules! adjustment_kinds {
    ($($variant:ident),+ $(,)?) => {
        /// Every catalogue kind in CS6 order: three fills, sixteen adjustment
        /// layers, Shadows/Highlights as a dialog-only entry, and five
        /// direct-only commands. Invert is both direct and a layer.
        ///
        /// Replace Color and Match Color stay outside the catalogue.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum AdjustmentKind {
            $($variant),+
        }

        impl AdjustmentKind {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
    };
}

adjustment_kinds! {
    SolidColor,
    Gradient,
    Pattern,
    BrightnessContrast,
    Levels,
    Curves,
    Exposure,
    Vibrance,
    HueSaturation,
    ColorBalance,
    BlackWhite,
    PhotoFilter,
    ChannelMixer,
    ColorLookup,
    Invert,
    Posterize,
    Threshold,
    GradientMap,
    SelectiveColor,
    ShadowsHighlights,
    Desaturate,
    Equalize,
    AutoTone,
    AutoContrast,
    AutoColor,
}

impl AdjustmentKind {
    /// The catalogue's kind id.
    pub const fn name(self) -> &'static str {
        match self {
            Self::SolidColor => "solid-color",
            Self::Gradient => "gradient",
            Self::Pattern => "pattern",
            Self::BrightnessContrast => "brightness-contrast",
            Self::Levels => "levels",
            Self::Curves => "curves",
            Self::Exposure => "exposure",
            Self::Vibrance => "vibrance",
            Self::HueSaturation => "hue-saturation",
            Self::ColorBalance => "color-balance",
            Self::BlackWhite => "black-white",
            Self::PhotoFilter => "photo-filter",
            Self::ChannelMixer => "channel-mixer",
            Self::ColorLookup => "color-lookup",
            Self::Invert => "invert",
            Self::Posterize => "posterize",
            Self::Threshold => "threshold",
            Self::GradientMap => "gradient-map",
            Self::SelectiveColor => "selective-color",
            Self::ShadowsHighlights => "shadows-highlights",
            Self::Desaturate => "desaturate",
            Self::Equalize => "equalize",
            Self::AutoTone => "auto-tone",
            Self::AutoContrast => "auto-contrast",
            Self::AutoColor => "auto-color",
        }
    }

    /// The kind with id `name`, or `None`.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.name() == name)
    }

    /// A fill layer (Solid Color, Gradient, Pattern) rather than an adjustment.
    pub const fn is_fill(self) -> bool {
        matches!(self, Self::SolidColor | Self::Gradient | Self::Pattern)
    }
}

impl Adjustment {
    /// The catalogue kind of this adjustment, or `None` for Replace Color and
    /// Match Color, which stay outside the catalogue.
    pub fn kind(&self) -> Option<AdjustmentKind> {
        Some(match self {
            Self::Levels(_) => AdjustmentKind::Levels,
            Self::Curves(_) => AdjustmentKind::Curves,
            Self::BrightnessContrast(_) => AdjustmentKind::BrightnessContrast,
            Self::Exposure(_) => AdjustmentKind::Exposure,
            Self::HueSaturation(_) => AdjustmentKind::HueSaturation,
            Self::BlackWhite(_) => AdjustmentKind::BlackWhite,
            Self::PhotoFilter(_) => AdjustmentKind::PhotoFilter,
            Self::ChannelMixer(_) => AdjustmentKind::ChannelMixer,
            Self::Vibrance(_) => AdjustmentKind::Vibrance,
            Self::ShadowsHighlights(_) => AdjustmentKind::ShadowsHighlights,
            Self::ColorBalance(_) => AdjustmentKind::ColorBalance,
            Self::SelectiveColor(_) => AdjustmentKind::SelectiveColor,
            Self::Auto(auto) | Self::ConfiguredAuto(crate::AutoOptions { algorithm: auto, .. }) => match auto {
                AutoKind::Tone => AdjustmentKind::AutoTone,
                AutoKind::Contrast | AutoKind::BrightnessContrast => AdjustmentKind::AutoContrast,
                AutoKind::Color => AdjustmentKind::AutoColor,
            },
            Self::Invert => AdjustmentKind::Invert,
            Self::Posterize(_) => AdjustmentKind::Posterize,
            Self::Threshold(_) => AdjustmentKind::Threshold,
            Self::Desaturate => AdjustmentKind::Desaturate,
            Self::Equalize => AdjustmentKind::Equalize,
            Self::GradientMap(_) => AdjustmentKind::GradientMap,
            Self::ColorLookup(_) => AdjustmentKind::ColorLookup,
            Self::ReplaceColor(_) | Self::MatchColor(_) => return None,
        })
    }
}

impl FillContent {
    pub const fn kind(&self) -> AdjustmentKind {
        match self {
            Self::Solid(_) => AdjustmentKind::SolidColor,
            Self::Gradient(_) => AdjustmentKind::Gradient,
            Self::Pattern(_) => AdjustmentKind::Pattern,
        }
    }
}

pub const fn direct_invert() -> Adjustment {
    Adjustment::Invert
}

pub const fn direct_desaturate() -> Adjustment {
    Adjustment::Desaturate
}

pub const fn direct_equalize() -> Adjustment {
    Adjustment::Equalize
}

pub const fn direct_auto_tone() -> Adjustment {
    Adjustment::Auto(AutoKind::Tone)
}

pub const fn direct_auto_contrast() -> Adjustment {
    Adjustment::Auto(AutoKind::Contrast)
}

pub const fn direct_auto_color() -> Adjustment {
    Adjustment::Auto(AutoKind::Color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_name_round_trips_and_is_unique() {
        for (i, kind) in AdjustmentKind::ALL.iter().enumerate() {
            assert_eq!(AdjustmentKind::from_name(kind.name()), Some(*kind));
            assert!(
                AdjustmentKind::ALL[i + 1..]
                    .iter()
                    .all(|other| other != kind),
                "{} appears twice",
                kind.name()
            );
        }
        assert_eq!(AdjustmentKind::ALL.len(), 25);
        assert_eq!(AdjustmentKind::from_name("match-color"), None);
        assert_eq!(
            AdjustmentKind::ALL.iter().filter(|k| k.is_fill()).count(),
            3
        );
    }

    #[test]
    fn adjustment_kinds_cover_every_non_catalogue_variant() {
        let replace = Adjustment::ReplaceColor(crate::ReplaceColorParams {
            samples: Vec::new(),
            fuzziness: 0.0,
            localized: false,
            hue: 0.0,
            saturation: 0.0,
            lightness: 0.0,
        });
        assert_eq!(replace.kind(), None);
        let matched = Adjustment::MatchColor(crate::MatchColorParams {
            target: crate::MatchColorStats::ZERO,
            source: crate::MatchColorStats::ZERO,
            neutralize: false,
            luminance: 0.0,
            color_intensity: 0.0,
            fade: 0.0,
        });
        assert_eq!(matched.kind(), None);
        for auto in [AutoKind::Tone, AutoKind::Contrast, AutoKind::Color] {
            assert!(Adjustment::Auto(auto).kind().is_some());
        }
    }
}
