//! The built-in Adobe PDF presets. ponytail: the names are CS6's; their
//! settings are inferred from the presets' published intent (resolution,
//! JPEG quality, colour handling), not copied from Adobe's files.

use super::options::{
    ColorConversion, Compatibility, Compression, Destination, Downsample, PdfOptions,
    ProfileInclusion, Quality, Standard,
};

/// Built-in preset names in list order; each is shown in brackets.
pub const BUILTIN_PRESETS: [&str; 12] = [
    "[Photoshop Default]",
    "[High Quality Print]",
    "[Press Quality]",
    "[Smallest File Size]",
    "[PDF/X-1a:2001]",
    "[PDF/X-3:2002]",
    "[PDF/X-4:2008]",
    "[PDF/A-1b:2005]",
    "[Standard]",
    "[Rich Content PDF]",
    "[Illustrator Default]",
    "[Oversized Pages]",
];

struct Spec {
    description: &'static str,
    standard: Standard,
    compatibility: Compatibility,
    preserve_editing: bool,
    fast_web_view: bool,
    /// `(ppi, above ppi)`, or `None` for Do Not Downsample.
    bicubic: Option<(f64, f64)>,
    /// `None` for ZIP.
    jpeg: Option<Quality>,
    convert_to: Option<Destination>,
}

fn spec(name: &str) -> Option<Spec> {
    let base = Spec {
        description: "",
        standard: Standard::None,
        compatibility: Compatibility::Acrobat5,
        preserve_editing: false,
        fast_web_view: true,
        bicubic: Some((300.0, 450.0)),
        jpeg: Some(Quality::Maximum),
        convert_to: None,
    };
    Some(match name {
        "[Photoshop Default]" => Spec {
            description: "Use these settings to create Photoshop PDF documents that keep \
                          Photoshop editing data.",
            compatibility: Compatibility::Acrobat6,
            preserve_editing: true,
            fast_web_view: false,
            bicubic: None,
            jpeg: None,
            ..base
        },
        "[High Quality Print]" => Spec {
            description: "Use these settings for quality printing on desktop printers and \
                          proofers.",
            ..base
        },
        "[Press Quality]" => Spec {
            description: "Use these settings for high-quality prepress printing.",
            ..base
        },
        "[Smallest File Size]" => Spec {
            description: "Use these settings for on-screen display, e-mail, and the Internet.",
            compatibility: Compatibility::Acrobat6,
            bicubic: Some((100.0, 150.0)),
            jpeg: Some(Quality::Low),
            convert_to: Some(Destination::Srgb),
            ..base
        },
        "[PDF/X-1a:2001]" => Spec {
            description: "Use these settings for PDF/X-1a:2001 (CMYK print exchange).",
            standard: Standard::X1a2001,
            compatibility: Compatibility::Acrobat4,
            fast_web_view: false,
            convert_to: Some(Destination::CoatedCmyk),
            ..base
        },
        "[PDF/X-3:2002]" => Spec {
            description: "Use these settings for PDF/X-3:2002 (colour-managed print exchange).",
            standard: Standard::X32002,
            compatibility: Compatibility::Acrobat4,
            fast_web_view: false,
            ..base
        },
        "[PDF/X-4:2008]" => Spec {
            description: "Use these settings for PDF/X-4:2008 (print exchange with live \
                          transparency).",
            standard: Standard::X42008,
            compatibility: Compatibility::Acrobat7,
            fast_web_view: false,
            ..base
        },
        "[PDF/A-1b:2005]" => Spec {
            description: "Use these settings for PDF/A-1b:2005 long-term archiving.",
            standard: Standard::A1b2005,
            fast_web_view: false,
            ..base
        },
        "[Standard]" => Spec {
            description: "Use these settings for business documents to view and print.",
            bicubic: Some((150.0, 225.0)),
            jpeg: Some(Quality::Medium),
            convert_to: Some(Destination::Srgb),
            ..base
        },
        "[Rich Content PDF]" => Spec {
            description: "Use these settings for accessible on-screen documents.",
            compatibility: Compatibility::Acrobat6,
            bicubic: Some((150.0, 225.0)),
            jpeg: Some(Quality::Medium),
            convert_to: Some(Destination::Srgb),
            ..base
        },
        "[Illustrator Default]" => Spec {
            description: "Use these settings for documents shared with Illustrator.",
            compatibility: Compatibility::Acrobat6,
            fast_web_view: false,
            bicubic: None,
            jpeg: None,
            ..base
        },
        "[Oversized Pages]" => Spec {
            description: "Use these settings for engineering drawings larger than 200 × 200 \
                          inches.",
            compatibility: Compatibility::Acrobat6,
            jpeg: Some(Quality::High),
            ..base
        },
        _ => return None,
    })
}

/// The built-in preset `name` (with its brackets), or `None`.
pub fn builtin_preset(name: &str) -> Option<PdfOptions> {
    let spec = spec(name)?;
    let mut o = PdfOptions {
        description: spec.description.to_string(),
        standard: spec.standard,
        compatibility: spec.compatibility,
        preserve_editing: spec.preserve_editing,
        fast_web_view: spec.fast_web_view,
        embed_thumbnails: spec.preserve_editing,
        ..PdfOptions::default()
    };
    match spec.bicubic {
        Some((ppi, above)) => {
            o.downsample = Downsample::Bicubic;
            o.downsample_ppi = ppi;
            o.downsample_above_ppi = above;
        }
        None => o.downsample = Downsample::None,
    }
    match spec.jpeg {
        Some(quality) => {
            o.compression = Compression::Jpeg;
            o.quality = quality;
        }
        None => o.compression = Compression::Zip,
    }
    if let Some(destination) = spec.convert_to {
        o.color_conversion = ColorConversion::ToDestination;
        o.destination = destination;
        o.profile_inclusion = if destination == Destination::Srgb {
            ProfileInclusion::None
        } else {
            ProfileInclusion::IncludeDestination
        };
    }
    o.normalize();
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_preset_resolves_and_normalizes_to_itself() {
        for name in BUILTIN_PRESETS {
            let preset = builtin_preset(name).expect(name);
            let mut again = preset.clone();
            again.normalize();
            assert_eq!(again, preset, "{name}");
        }
        assert_eq!(builtin_preset("[Nope]"), None);
        let smallest = builtin_preset("[Smallest File Size]").unwrap();
        assert_eq!(smallest.quality, Quality::Low);
        assert_eq!(smallest.downsample_ppi, 100.0);
        assert_eq!(
            builtin_preset("[Photoshop Default]"),
            Some(PdfOptions::default())
        );
    }
}
