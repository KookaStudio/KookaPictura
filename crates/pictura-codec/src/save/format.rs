//! The authoritative Save As format catalog: identity, wire code, aliases,
//! canonical extension, writer route, and the policy flags callers branch on.
//!
//! [`SaveKind`](super::SaveKind) stays the wire-code set the dispatch recognizes;
//! this layer adds the formats Save As offers that the engine does not encode
//! (PSD, PSB, WebP, PDF) and the route each one takes.

use super::SaveKind;

/// The row is a native PSD/PSB document written through the save worker.
pub const FLAG_NATIVE: u32 = 1 << 0;
/// The row is Photoshop PDF.
pub const FLAG_PDF: u32 = 1 << 1;
/// The row is a flat raster save (the feature-warning set).
pub const FLAG_RASTER: u32 = 1 << 2;
/// The row keeps layers without a dialog or a warning (TIFF).
pub const FLAG_KEEPS_LAYERS: u32 = 1 << 3;
/// The row offers the Embed Color Profile prompt.
pub const FLAG_OFFERS_PROFILE: u32 = 1 << 4;

/// Where a Save As write goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveRoute {
    /// The native PSD/PSB worker.
    Native,
    /// Photoshop PDF export.
    Pdf,
    /// The app's indexed-source GIF encoder, not `SaveKind::Gif`.
    AppGif,
    /// An engine writer for this [`SaveKind`].
    Engine(SaveKind),
    /// Qt's image writer under this format name.
    QtRaster(&'static str),
}

/// One displayed Save As format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFormat {
    Psd,
    Psb,
    Png,
    Jpeg,
    Tiff,
    Webp,
    Bmp,
    Pdf,
    Gif,
    Eps,
    Dcs,
    Raw,
    Tga,
    Cineon,
    Pict,
}

/// The facts behind a [`SaveFormat`], in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveFormatFacts {
    /// The catalog ordinal; never the wire code.
    pub id: i32,
    /// The stable source/session name (`"psd"`, ..., `"dcs"`).
    pub name: &'static str,
    /// The options wire code, or 0 when the format opens no options dialog.
    pub code: i32,
    /// Lowercase suffixes that name this format; empty for DCS.
    pub aliases: &'static [&'static str],
    /// The extension appended when the path has none.
    pub canonical: &'static str,
    pub route: SaveRoute,
    pub flags: u32,
}

const FORMATS: [SaveFormatFacts; 15] = [
    SaveFormatFacts {
        id: 0,
        name: "psd",
        code: 0,
        aliases: &["psd"],
        canonical: ".psd",
        route: SaveRoute::Native,
        flags: FLAG_NATIVE | FLAG_OFFERS_PROFILE,
    },
    SaveFormatFacts {
        id: 1,
        name: "psb",
        code: 0,
        aliases: &["psb"],
        canonical: ".psb",
        route: SaveRoute::Native,
        flags: FLAG_NATIVE | FLAG_OFFERS_PROFILE,
    },
    SaveFormatFacts {
        id: 2,
        name: "png",
        code: 2,
        aliases: &["png"],
        canonical: ".png",
        route: SaveRoute::Engine(SaveKind::Png),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 3,
        name: "jpeg",
        code: 1,
        aliases: &["jpg", "jpeg", "jpe"],
        canonical: ".jpg",
        route: SaveRoute::QtRaster("JPG"),
        flags: FLAG_RASTER | FLAG_OFFERS_PROFILE,
    },
    SaveFormatFacts {
        id: 4,
        name: "tiff",
        code: 4,
        aliases: &["tif", "tiff"],
        canonical: ".tif",
        route: SaveRoute::Engine(SaveKind::Tiff),
        flags: FLAG_RASTER | FLAG_KEEPS_LAYERS | FLAG_OFFERS_PROFILE,
    },
    SaveFormatFacts {
        id: 5,
        name: "webp",
        code: 0,
        aliases: &["webp"],
        canonical: ".webp",
        route: SaveRoute::QtRaster("WEBP"),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 6,
        name: "bmp",
        code: 3,
        aliases: &["bmp"],
        canonical: ".bmp",
        route: SaveRoute::Engine(SaveKind::Bmp),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 7,
        name: "pdf",
        code: 0,
        aliases: &["pdf", "pdp"],
        canonical: ".pdf",
        route: SaveRoute::Pdf,
        flags: FLAG_PDF,
    },
    SaveFormatFacts {
        id: 8,
        name: "gif",
        code: 5,
        aliases: &["gif"],
        canonical: ".gif",
        route: SaveRoute::AppGif,
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 9,
        name: "eps",
        code: 6,
        aliases: &["eps"],
        canonical: ".eps",
        route: SaveRoute::Engine(SaveKind::Eps),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 10,
        name: "dcs",
        code: 7,
        aliases: &[],
        canonical: ".eps",
        route: SaveRoute::Engine(SaveKind::Dcs),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 11,
        name: "raw",
        code: 8,
        aliases: &["raw"],
        canonical: ".raw",
        route: SaveRoute::Engine(SaveKind::Raw),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 12,
        name: "tga",
        code: 9,
        aliases: &["tga", "vda", "icb", "vst"],
        canonical: ".tga",
        route: SaveRoute::Engine(SaveKind::Tga),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 13,
        name: "cineon",
        code: 10,
        aliases: &["cin"],
        canonical: ".cin",
        route: SaveRoute::Engine(SaveKind::Cineon),
        flags: FLAG_RASTER,
    },
    SaveFormatFacts {
        id: 14,
        name: "pict",
        code: 11,
        aliases: &["pct", "pic", "pict"],
        canonical: ".pct",
        route: SaveRoute::Engine(SaveKind::Pict),
        flags: FLAG_RASTER,
    },
];

impl SaveFormat {
    /// Every displayed Save As format, in ordinal/display order.
    pub const ALL: [Self; 15] = [
        Self::Psd,
        Self::Psb,
        Self::Png,
        Self::Jpeg,
        Self::Tiff,
        Self::Webp,
        Self::Bmp,
        Self::Pdf,
        Self::Gif,
        Self::Eps,
        Self::Dcs,
        Self::Raw,
        Self::Tga,
        Self::Cineon,
        Self::Pict,
    ];

    /// The catalog ordinal (0..13); the bridge's `i32` id.
    pub fn id(self) -> i32 {
        self as i32
    }

    pub fn facts(self) -> &'static SaveFormatFacts {
        &FORMATS[self as usize]
    }

    pub fn name(self) -> &'static str {
        self.facts().name
    }

    pub fn code(self) -> i32 {
        self.facts().code
    }

    pub fn aliases(self) -> &'static [&'static str] {
        self.facts().aliases
    }

    pub fn canonical(self) -> &'static str {
        self.facts().canonical
    }

    pub fn route(self) -> SaveRoute {
        self.facts().route
    }

    pub fn flags(self) -> u32 {
        self.facts().flags
    }

    /// The format for a catalog ordinal, or `None` for `-1` and out-of-range ids.
    pub fn from_id(id: i32) -> Option<Self> {
        if (0..Self::ALL.len() as i32).contains(&id) {
            Some(Self::ALL[id as usize])
        } else {
            None
        }
    }

    /// The format a typed path suffix names, any case. Alias-only: `dcs` is not
    /// a suffix, so it stays unknown.
    pub fn from_suffix(suffix: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| {
            format
                .aliases()
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(suffix))
        })
    }

    /// The format a remembered source name or path suffix names, any case. Unlike
    /// [`Self::from_suffix`], this accepts the identity names, so `"dcs"` finds
    /// DCS.
    pub fn from_source_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| {
            format.name().eq_ignore_ascii_case(name)
                || format
                    .aliases()
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(name))
        })
    }

    /// The format's lowercase aliases as one space-separated string.
    pub fn aliases_text(self) -> String {
        self.aliases().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_format_catalog_matches_wire_codes() {
        assert_eq!(SaveFormat::ALL.len(), FORMATS.len());
        for (index, format) in SaveFormat::ALL.into_iter().enumerate() {
            let facts = format.facts();
            assert_eq!(facts.id, index as i32);
            assert_eq!(SaveFormat::from_id(facts.id), Some(format));
            assert_eq!(SaveFormat::from_source_name(facts.name), Some(format));
            match facts.route {
                SaveRoute::Engine(kind) => assert_eq!(kind.code(), facts.code, "{:?}", format),
                SaveRoute::AppGif => assert_eq!(facts.code, SaveKind::Gif.code()),
                SaveRoute::QtRaster(_) => {
                    assert!(matches!(format, SaveFormat::Jpeg | SaveFormat::Webp))
                }
                SaveRoute::Native => {
                    assert!(matches!(format, SaveFormat::Psd | SaveFormat::Psb));
                    assert_eq!(facts.code, 0);
                }
                SaveRoute::Pdf => {
                    assert_eq!(format, SaveFormat::Pdf);
                    assert_eq!(facts.code, 0);
                }
            }
        }
        assert_eq!(SaveFormat::Dcs.id(), 10);
        assert_eq!(SaveFormat::Dcs.code(), 7);
        assert_ne!(SaveFormat::Dcs.id(), SaveFormat::Dcs.code());
        assert_eq!(SaveFormat::Pict.id(), 14);
        assert_eq!(SaveFormat::Pict.code(), 11);
        assert_ne!(SaveFormat::Pict.id(), SaveFormat::Pict.code());
        assert_eq!(SaveFormat::Psd.id(), 0);
        assert_eq!(SaveFormat::Psd.code(), 0);
        for format in [SaveFormat::Psb, SaveFormat::Webp, SaveFormat::Pdf] {
            assert_eq!(format.code(), 0);
            assert_ne!(format.id(), format.code());
        }
    }

    #[test]
    fn save_format_suffix_aliases_are_case_insensitive() {
        for format in SaveFormat::ALL {
            for alias in format.aliases() {
                assert_eq!(SaveFormat::from_suffix(alias), Some(format), "{alias}");
                assert_eq!(
                    SaveFormat::from_suffix(&alias.to_ascii_uppercase()),
                    Some(format),
                    "{alias}"
                );
            }
            if format != SaveFormat::Dcs {
                assert_eq!(
                    SaveFormat::from_suffix(format.canonical().trim_start_matches('.')),
                    Some(format)
                );
            }
        }
        assert_eq!(SaveFormat::from_suffix("EPS"), Some(SaveFormat::Eps));
    }

    #[test]
    fn dcs_has_no_suffix_alias() {
        assert_eq!(SaveFormat::from_suffix("dcs"), None);
        assert_eq!(SaveFormat::from_suffix("eps"), Some(SaveFormat::Eps));
        assert_eq!(SaveFormat::from_source_name("dcs"), Some(SaveFormat::Dcs));
    }

    #[test]
    fn save_format_source_names_resolve() {
        for format in SaveFormat::ALL {
            assert_eq!(SaveFormat::from_source_name(format.name()), Some(format));
            for alias in format.aliases() {
                assert_eq!(SaveFormat::from_source_name(alias), Some(format));
            }
        }
        assert_eq!(SaveFormat::from_source_name("xyz"), None);
    }

    #[test]
    fn save_format_route_matrix() {
        let expected = [
            (SaveFormat::Psd, SaveRoute::Native),
            (SaveFormat::Psb, SaveRoute::Native),
            (SaveFormat::Png, SaveRoute::Engine(SaveKind::Png)),
            (SaveFormat::Jpeg, SaveRoute::QtRaster("JPG")),
            (SaveFormat::Tiff, SaveRoute::Engine(SaveKind::Tiff)),
            (SaveFormat::Webp, SaveRoute::QtRaster("WEBP")),
            (SaveFormat::Bmp, SaveRoute::Engine(SaveKind::Bmp)),
            (SaveFormat::Pdf, SaveRoute::Pdf),
            (SaveFormat::Gif, SaveRoute::AppGif),
            (SaveFormat::Eps, SaveRoute::Engine(SaveKind::Eps)),
            (SaveFormat::Dcs, SaveRoute::Engine(SaveKind::Dcs)),
            (SaveFormat::Raw, SaveRoute::Engine(SaveKind::Raw)),
            (SaveFormat::Tga, SaveRoute::Engine(SaveKind::Tga)),
            (SaveFormat::Cineon, SaveRoute::Engine(SaveKind::Cineon)),
            (SaveFormat::Pict, SaveRoute::Engine(SaveKind::Pict)),
        ];
        for (format, route) in expected {
            assert_eq!(format.route(), route, "{format:?}");
        }
    }
}
