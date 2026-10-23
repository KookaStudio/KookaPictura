//! ICC color management for Kooka Pictura.
//!
//! M3 scope: working-space profiles (sRGB / Adobe RGB / ProPhoto), profile
//! assignment vs conversion, rendering intents, and black point compensation.
//! Spec: `docs/01-architecture/color-management.md`.
//!
//! The engine is Little CMS 2 through the `lcms2` crate. Built-in wide-gamut
//! profiles are synthesised from their primaries at runtime rather than bundled,
//! so no Adobe profile files are distributed.

use lcms2::{
    CIExyY, CIExyYTRIPLE, ColorSpaceSignature, Flags, InfoType, Intent as LcmsIntent, Locale,
    PixelFormat, Profile as LcmsProfile, ToneCurve, Transform,
};

pub mod ntsc;

/// Rendering intent (ICC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

impl Intent {
    fn to_lcms(self) -> LcmsIntent {
        match self {
            Intent::Perceptual => LcmsIntent::Perceptual,
            Intent::RelativeColorimetric => LcmsIntent::RelativeColorimetric,
            Intent::Saturation => LcmsIntent::Saturation,
            Intent::AbsoluteColorimetric => LcmsIntent::AbsoluteColorimetric,
        }
    }
}

const COATED_CMYK_ICC: &[u8] = include_bytes!("../../../assets/icc/photocraft-coated-cmyk.icc");

#[derive(Debug, thiserror::Error)]
pub enum ColorError {
    #[error("invalid ICC profile")]
    InvalidProfile,
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// How an opened RGB document's embedded ICC profile is treated.
///
/// This is an application preference, not document or history state. Persisted
/// as a stable integer by [`Policy::to_code`] / [`Policy::from_code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Policy {
    /// Keep the embedded profile: pixels stay in it, resource 1039 stays, and
    /// `Document.document_icc` drives the display conversion.
    #[default]
    Preserve,
    /// Convert the pixels to the sRGB working space and drop the profile.
    Convert,
    /// Ignore the embedded profile: pixels unchanged, untagged.
    Off,
}

impl Policy {
    /// The stable persistence code: `0` Preserve, `1` Convert, `2` Off.
    pub fn to_code(self) -> i32 {
        match self {
            Policy::Preserve => 0,
            Policy::Convert => 1,
            Policy::Off => 2,
        }
    }

    /// Parse a persistence code written by [`Policy::to_code`]; unknown codes
    /// yield `None` so a caller can fall back to the default.
    pub fn from_code(code: i32) -> Option<Policy> {
        match code {
            0 => Some(Policy::Preserve),
            1 => Some(Policy::Convert),
            2 => Some(Policy::Off),
            _ => None,
        }
    }
}

/// An ICC profile handle.
#[derive(Debug)]
pub struct Profile(LcmsProfile);

impl Profile {
    /// sRGB IEC61966-2.1 (the default working space).
    pub fn srgb() -> Profile {
        Profile(LcmsProfile::new_srgb())
    }

    /// Adobe RGB (1998): D65, Adobe primaries, gamma 563/256.
    pub fn adobe_rgb() -> Profile {
        rgb_profile(
            CIExyY {
                x: 0.3127,
                y: 0.3290,
                Y: 1.0,
            },
            CIExyYTRIPLE {
                Red: CIExyY {
                    x: 0.6400,
                    y: 0.3300,
                    Y: 1.0,
                },
                Green: CIExyY {
                    x: 0.2100,
                    y: 0.7100,
                    Y: 1.0,
                },
                Blue: CIExyY {
                    x: 0.1500,
                    y: 0.0600,
                    Y: 1.0,
                },
            },
            ToneCurve::new(563.0 / 256.0),
        )
    }

    /// ProPhoto RGB (ROMM RGB): D50, ROMM primaries, ROMM tone curve.
    pub fn pro_photo() -> Profile {
        rgb_profile(
            CIExyY {
                x: 0.34567,
                y: 0.35850,
                Y: 1.0,
            },
            CIExyYTRIPLE {
                Red: CIExyY {
                    x: 0.7347,
                    y: 0.2653,
                    Y: 1.0,
                },
                Green: CIExyY {
                    x: 0.1596,
                    y: 0.8404,
                    Y: 1.0,
                },
                Blue: CIExyY {
                    x: 0.0366,
                    y: 0.0001,
                    Y: 1.0,
                },
            },
            romm_tone_curve(),
        )
    }

    /// The bundled synthetic coated-offset CMYK output profile
    /// (`assets/icc/photocraft-coated-cmyk.icc`, CC0). ponytail: a modelled
    /// press, not a measured characterisation.
    pub fn coated_cmyk() -> Profile {
        Profile::from_icc(COATED_CMYK_ICC).expect("bundled CMYK profile parses")
    }

    /// The bundled CMYK profile's bytes, embedded verbatim where a file needs
    /// the exact profile (a PDF output intent).
    pub fn coated_cmyk_icc() -> &'static [u8] {
        COATED_CMYK_ICC
    }

    /// Gray with a 2.2 gamma tone curve and a D50 white.
    pub fn gray_gamma22() -> Profile {
        let white = CIExyY {
            x: 0.34567,
            y: 0.35850,
            Y: 1.0,
        };
        Profile(
            LcmsProfile::new_gray(&white, &ToneCurve::new(2.2))
                .expect("built-in gray profile is valid"),
        )
    }

    /// Colour channels of the profile's data space: 1 gray, 3 RGB/Lab, 4 CMYK;
    /// `None` for any other space.
    pub fn channels(&self) -> Option<u8> {
        match self.0.color_space() {
            ColorSpaceSignature::GrayData => Some(1),
            ColorSpaceSignature::RgbData | ColorSpaceSignature::LabData => Some(3),
            ColorSpaceSignature::CmykData => Some(4),
            _ => None,
        }
    }

    /// Parse an ICC profile from raw bytes. Malformed input yields an error.
    pub fn from_icc(bytes: &[u8]) -> Result<Profile, ColorError> {
        LcmsProfile::new_icc(bytes)
            .map(Profile)
            .map_err(|_| ColorError::InvalidProfile)
    }

    /// Serialize the profile back to ICC bytes.
    pub fn to_icc(&self) -> Vec<u8> {
        // A Profile only exists after lcms2 accepted it, so serialization
        // succeeds in practice; the empty fallback keeps this panic-free.
        let mut bytes = self.0.icc().unwrap_or_default();
        // lcms2 stamps the wall-clock creation time at profile build and offers
        // no setter (Little-CMS #71), so two serializations could straddle a
        // second boundary; zero the ICC header date field (bytes 24..36) to make
        // serialization byte-deterministic.
        if bytes.len() >= 36 {
            bytes[24..36].fill(0);
        }
        bytes
    }

    /// The profile re-serialized as ICC version 2.1, for files whose format
    /// predates ICC v4 (PDF 1.4 and older). lcms2 picks v2 tag types on save.
    pub fn to_icc_v2(&self) -> Vec<u8> {
        let Ok(mut copy) = LcmsProfile::new_icc(&self.to_icc()) else {
            return self.to_icc();
        };
        copy.set_version(2.1);
        let mut bytes = copy.icc().unwrap_or_default();
        if bytes.len() >= 36 {
            bytes[24..36].fill(0);
        }
        bytes
    }

    /// The profile's `Description` tag (`cmsInfoDescription`), when present.
    pub fn description(&self) -> Option<String> {
        self.0.info(InfoType::Description, Locale::default())
    }

    /// Whether this profile is the sRGB working space, by its description.
    ///
    /// ponytail: lcms2 has no profile-equality check, so this matches "srgb"
    /// case-insensitively in the `Description` tag; a differently-named sRGB
    /// profile is treated as non-sRGB (converted, within a rounding LSB).
    pub fn is_srgb(&self) -> bool {
        self.description()
            .is_some_and(|d| d.to_ascii_lowercase().contains("srgb"))
    }
}

fn rgb_profile(white: CIExyY, primaries: CIExyYTRIPLE, curve: ToneCurve) -> Profile {
    let curves = [&curve, &curve, &curve];
    Profile(
        LcmsProfile::new_rgb(&white, &primaries, &curves)
            .expect("built-in RGB primaries are valid"),
    )
}

/// ROMM RGB's tone curve: linear toe below 1/32, then a gamma-1.8 power.
/// lcms2 has no parametric type for it, so tabulate the encoded→linear mapping.
fn romm_tone_curve() -> ToneCurve {
    const N: usize = 4096;
    let mut table = [0u16; N];
    for (i, slot) in table.iter_mut().enumerate() {
        let encoded = i as f64 / (N - 1) as f64;
        let linear = if encoded < 0.03125 {
            encoded / 16.0
        } else {
            encoded.powf(1.8)
        };
        *slot = (linear * 65535.0).round() as u16;
    }
    ToneCurve::new_tabulated(&table)
}

/// Convert interleaved 8-, 16-, or 32-bit float samples between two profiles.
///
/// `channels` is 1 (gray), 3 (RGB) or 4 (RGBA); RGBA alpha is copied through
/// unchanged. Output keeps the input's channel count and bit depth. 16-bit
/// samples are native-endian, as are 32-bit float samples.
#[allow(clippy::too_many_arguments)]
pub fn convert(
    src: &Profile,
    dst: &Profile,
    data: &[u8],
    width: u32,
    height: u32,
    channels: u8,
    bits: u8,
    intent: Intent,
    black_point_compensation: bool,
) -> Result<Vec<u8>, ColorError> {
    let (in_format, out_format) = formats(channels, bits)?;
    let bytes_per_pixel = channels as usize * (bits / 8) as usize;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
        .ok_or_else(|| ColorError::Unsupported("image dimensions overflow".into()))?;
    if data.len() != expected {
        return Err(ColorError::Unsupported(format!(
            "expected {expected} bytes for {width}x{height}x{channels} @ {bits}-bit, got {}",
            data.len()
        )));
    }
    if expected == 0 {
        return Ok(Vec::new());
    }

    let mut flags = Flags::default();
    if black_point_compensation {
        flags = flags | Flags::BLACKPOINT_COMPENSATION;
    }
    if channels == 4 {
        flags = flags | Flags::COPY_ALPHA;
    }

    // `Transform<u8, u8>` lets lcms2 read/write the byte buffer directly using
    // the declared pixel format; no endianness or alignment juggling needed.
    let transform: Transform<u8, u8> = Transform::new_flags(
        &src.0,
        in_format,
        &dst.0,
        out_format,
        intent.to_lcms(),
        flags,
    )
    .map_err(|e| ColorError::Unsupported(e.to_string()))?;

    let mut out = vec![0u8; expected];
    transform.transform_pixels(data, &mut out);
    Ok(out)
}

/// Convert interleaved 8-bit samples between two profiles whose colour spaces
/// may differ (RGB → CMYK, RGB → gray, gray → RGB, …). Each side's channel
/// count is its profile's ([`Profile::channels`]); there is no alpha. Lab
/// profiles are not accepted.
pub fn convert_space(
    src: &Profile,
    dst: &Profile,
    data: &[u8],
    intent: Intent,
    black_point_compensation: bool,
) -> Result<Vec<u8>, ColorError> {
    let format = |profile: &Profile| match profile.0.color_space() {
        ColorSpaceSignature::GrayData => Ok((PixelFormat::GRAY_8, 1usize)),
        ColorSpaceSignature::RgbData => Ok((PixelFormat::RGB_8, 3)),
        ColorSpaceSignature::CmykData => Ok((PixelFormat::CMYK_8, 4)),
        other => Err(ColorError::Unsupported(format!("colour space {other:?}"))),
    };
    let (in_format, in_channels) = format(src)?;
    let (out_format, out_channels) = format(dst)?;
    if !data.len().is_multiple_of(in_channels) {
        return Err(ColorError::Unsupported(format!(
            "{} bytes is not a whole number of {in_channels}-channel pixels",
            data.len()
        )));
    }
    let pixels = data.len() / in_channels;
    let mut out = vec![0u8; pixels * out_channels];
    if pixels == 0 {
        return Ok(out);
    }
    let mut flags = Flags::default();
    if black_point_compensation {
        flags = flags | Flags::BLACKPOINT_COMPENSATION;
    }
    let transform: Transform<u8, u8> = Transform::new_flags(
        &src.0,
        in_format,
        &dst.0,
        out_format,
        intent.to_lcms(),
        flags,
    )
    .map_err(|e| ColorError::Unsupported(e.to_string()))?;
    transform.transform_pixels(data, &mut out);
    Ok(out)
}

/// Retag: returns the same pixels with a new profile attached (no transform).
pub fn assign(data: &[u8], profile: Profile) -> (Vec<u8>, Profile) {
    (data.to_vec(), profile)
}

/// A reusable 8-bit interleaved transform between two profiles.
///
/// Building a transform parses both profiles and their tone curves, so a caller
/// that converts the same document on every display refresh holds one of these
/// and reapplies it instead of rebuilding it. `apply` is per-pixel and writes
/// `out` from `data` without touching either profile again.
pub struct ByteTransform(Transform<u8, u8>);

impl ByteTransform {
    /// Build a transform for 1-, 3-, or 4-channel 8-bit samples. A 4-channel
    /// transform copies alpha through unchanged.
    pub fn new(
        src: &Profile,
        dst: &Profile,
        channels: u8,
        intent: Intent,
        black_point_compensation: bool,
    ) -> Result<Self, ColorError> {
        let (in_format, out_format) = formats(channels, 8)?;
        let mut flags = Flags::default();
        if black_point_compensation {
            flags = flags | Flags::BLACKPOINT_COMPENSATION;
        }
        if channels == 4 {
            flags = flags | Flags::COPY_ALPHA;
        }
        Transform::new_flags(
            &src.0,
            in_format,
            &dst.0,
            out_format,
            intent.to_lcms(),
            flags,
        )
        .map(ByteTransform)
        .map_err(|e| ColorError::Unsupported(e.to_string()))
    }

    /// Apply the transform to `data`, writing exactly `data.len()` bytes to
    /// `out`.
    pub fn apply(&self, data: &[u8], out: &mut [u8]) {
        self.0.transform_pixels(data, out);
    }
}

fn formats(channels: u8, bits: u8) -> Result<(PixelFormat, PixelFormat), ColorError> {
    let format = match (channels, bits) {
        (1, 8) => PixelFormat::GRAY_8,
        (1, 16) => PixelFormat::GRAY_16,
        (1, 32) => PixelFormat::GRAY_FLT,
        (3, 8) => PixelFormat::RGB_8,
        (3, 16) => PixelFormat::RGB_16,
        (3, 32) => PixelFormat::RGB_FLT,
        (4, 8) => PixelFormat::RGBA_8,
        (4, 16) => PixelFormat::RGBA_16,
        (4, 32) => PixelFormat::RGBA_FLT,
        (c, b) => return Err(ColorError::Unsupported(format!("{c} channels at {b} bits"))),
    };
    Ok((format, format))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn description_and_srgb_detection() {
        assert!(Profile::srgb().is_srgb(), "the working space is sRGB");
        let adobe = Profile::adobe_rgb();
        assert!(!adobe.is_srgb(), "Adobe RGB is not sRGB");
        assert!(
            adobe.description().is_some(),
            "a synthesized profile has a description"
        );
        let reloaded = Profile::from_icc(&adobe.to_icc()).expect("round-trips");
        assert!(!reloaded.is_srgb(), "detection survives an ICC round trip");
    }

    fn rgb8(pixels: &[[u8; 3]]) -> Vec<u8> {
        pixels.iter().flatten().copied().collect()
    }

    fn rgb16(pixels: &[[u16; 3]]) -> Vec<u8> {
        let mut out = Vec::new();
        for pixel in pixels {
            for channel in pixel {
                out.extend_from_slice(&channel.to_ne_bytes());
            }
        }
        out
    }

    fn rgb16_from_bytes(bytes: &[u8]) -> Vec<u16> {
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_ne_bytes(*c))
            .collect()
    }

    #[test]
    fn identity_rgb8() {
        let srgb = Profile::srgb();
        let img = rgb8(&[[0, 0, 0], [255, 255, 255], [10, 128, 240], [200, 50, 100]]);
        let out = convert(
            &srgb,
            &srgb,
            &img,
            4,
            1,
            3,
            8,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        for (a, b) in img.iter().zip(&out) {
            assert!((*a as i16 - *b as i16).abs() <= 1, "{a} vs {b}");
        }
    }

    #[test]
    fn identity_rgb16() {
        let srgb = Profile::srgb();
        let pixels = [
            [0u16, 0, 0],
            [65535, 65535, 65535],
            [1000, 30000, 60000],
            [40000, 20000, 500],
        ];
        let img = rgb16(&pixels);
        let out = convert(
            &srgb,
            &srgb,
            &img,
            4,
            1,
            3,
            16,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        let got = rgb16_from_bytes(&out);
        let want: Vec<u16> = pixels.iter().flatten().copied().collect();
        for (a, b) in want.iter().zip(&got) {
            assert!((*a as i32 - *b as i32).abs() <= 1, "{a} vs {b}");
        }
    }

    #[test]
    fn identity_rgb32() {
        let srgb = Profile::srgb();
        let pixels = [
            [0.0f32, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.25, 0.5, 0.75],
            [0.1, 0.9, 0.5],
        ];
        let img: Vec<u8> = pixels
            .iter()
            .flatten()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        let out = convert(
            &srgb,
            &srgb,
            &img,
            4,
            1,
            3,
            32,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        assert_eq!(out.len(), img.len());
        let got: Vec<f32> = out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_ne_bytes(*c))
            .collect();
        let want: Vec<f32> = pixels.iter().flatten().copied().collect();
        for (a, b) in want.iter().zip(&got) {
            assert!((a - b).abs() <= 0.001, "32-bit identity moved {a} to {b}");
        }

        // Gray and RGBA float buffers use the same pixel formats.
        let gray_profile = Profile::gray_gamma22();
        let gray = [0.0f32, 1.0, 0.25, 0.75];
        let img: Vec<u8> = gray.iter().flat_map(|v| v.to_ne_bytes()).collect();
        let out = convert(
            &gray_profile,
            &gray_profile,
            &img,
            4,
            1,
            1,
            32,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        assert_eq!(out.len(), img.len());
        for (a, b) in gray.iter().zip(
            out.as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_ne_bytes(*c)),
        ) {
            assert!((a - b).abs() <= 0.001, "gray identity moved {a} to {b}");
        }

        let alpha = [0.0f32, 1.0, 0.5, 0.25];
        let mut img = Vec::new();
        for (pixel, a) in pixels.iter().zip(alpha) {
            for value in pixel {
                img.extend_from_slice(&value.to_ne_bytes());
            }
            img.extend_from_slice(&a.to_ne_bytes());
        }
        let out = convert(
            &srgb,
            &srgb,
            &img,
            4,
            1,
            4,
            32,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        assert_eq!(out.len(), img.len());
        for (i, rgba) in out.as_chunks::<16>().0.iter().enumerate() {
            for (ch, (a, b)) in pixels[i]
                .iter()
                .zip(
                    rgba[..12]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| f32::from_ne_bytes(*c)),
                )
                .enumerate()
            {
                assert!(
                    (a - b).abs() <= 0.001,
                    "RGBA identity moved {ch}: {a} to {b}"
                );
            }
            let got_alpha = f32::from_ne_bytes(rgba[12..].try_into().unwrap());
            assert!((alpha[i] - got_alpha).abs() <= f32::EPSILON, "alpha copied");
        }
    }

    #[test]
    fn unsupported_format_is_rejected() {
        let srgb = Profile::srgb();
        let two = convert(
            &srgb,
            &srgb,
            &[0u8; 8],
            2,
            1,
            2,
            8,
            Intent::RelativeColorimetric,
            false,
        );
        assert!(matches!(two, Err(ColorError::Unsupported(_))));
        let twelve = convert(
            &srgb,
            &srgb,
            &[0u8; 4],
            1,
            1,
            1,
            12,
            Intent::RelativeColorimetric,
            false,
        );
        assert!(matches!(twelve, Err(ColorError::Unsupported(_))));
    }

    #[test]
    fn round_trip_srgb_adobe_rgb() {
        let srgb = Profile::srgb();
        let adobe = Profile::adobe_rgb();
        let img = rgb8(&[
            [0, 0, 0],
            [255, 255, 255],
            [128, 128, 128],
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [64, 160, 220],
        ]);
        let mid = convert(
            &srgb,
            &adobe,
            &img,
            7,
            1,
            3,
            8,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        let back = convert(
            &adobe,
            &srgb,
            &mid,
            7,
            1,
            3,
            8,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        for (a, b) in img.iter().zip(&back) {
            assert!((*a as i16 - *b as i16).abs() <= 3, "{a} vs {b}");
        }
    }

    #[test]
    fn every_intent_produces_output() {
        let srgb = Profile::srgb();
        let adobe = Profile::adobe_rgb();
        let mut img = Vec::new();
        for i in 0..64u8 {
            img.extend_from_slice(&[i * 4, 255 - i * 4, i * 3]);
        }
        for intent in [
            Intent::Perceptual,
            Intent::RelativeColorimetric,
            Intent::Saturation,
            Intent::AbsoluteColorimetric,
        ] {
            let out = convert(&srgb, &adobe, &img, 64, 1, 3, 8, intent, true).unwrap();
            assert_eq!(out.len(), img.len());
            let (triples, _) = out.as_chunks::<3>();
            let distinct: HashSet<&[u8; 3]> = triples.iter().collect();
            assert!(distinct.len() > 8, "{intent:?} produced degenerate output");
        }
    }

    #[test]
    fn gray_identity() {
        // No gray built-in, so build one with lcms2 and load it through from_icc.
        let gray = {
            let white = lcms2::white_point_from_temp(6504.0).unwrap();
            let curve = ToneCurve::new(2.2);
            let profile = LcmsProfile::new_gray(&white, &curve).unwrap();
            Profile::from_icc(&profile.icc().unwrap()).unwrap()
        };
        let img = [0u8, 1, 64, 128, 200, 254, 255];
        let out = convert(
            &gray,
            &gray,
            &img,
            7,
            1,
            1,
            8,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        for (a, b) in img.iter().zip(&out) {
            assert!((*a as i16 - *b as i16).abs() <= 1, "{a} vs {b}");
        }
    }

    #[test]
    fn rgba_alpha_is_preserved() {
        let srgb = Profile::srgb();
        let adobe = Profile::adobe_rgb();
        let pixels: [[u8; 4]; 4] = [
            [0, 0, 0, 0],
            [255, 255, 255, 1],
            [10, 128, 240, 127],
            [200, 50, 100, 255],
        ];
        let img: Vec<u8> = pixels.iter().flatten().copied().collect();
        let out = convert(
            &srgb,
            &adobe,
            &img,
            4,
            1,
            4,
            8,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        for (i, pixel) in pixels.iter().enumerate() {
            assert_eq!(out[i * 4 + 3], pixel[3], "alpha at pixel {i}");
        }
    }

    #[test]
    fn malformed_icc_is_error_not_panic() {
        assert!(Profile::from_icc(&[]).is_err());
        assert!(Profile::from_icc(&[0u8; 128]).is_err());
        assert!(Profile::from_icc(b"definitely not an ICC profile").is_err());
    }

    #[test]
    fn icc_round_trips() {
        for profile in [Profile::srgb(), Profile::adobe_rgb(), Profile::pro_photo()] {
            let bytes = profile.to_icc();
            assert!(!bytes.is_empty());
            let reloaded = Profile::from_icc(&bytes).unwrap();
            assert_eq!(reloaded.0.color_space(), profile.0.color_space());
        }
    }

    #[test]
    fn serialized_profiles_have_a_zeroed_creation_date() {
        let icc = Profile::adobe_rgb().to_icc();
        assert!(
            icc[24..36].iter().all(|&b| b == 0),
            "the serialized ICC creation date must be zeroed"
        );
    }

    #[test]
    fn serialized_profiles_are_byte_identical() {
        assert_eq!(Profile::adobe_rgb().to_icc(), Profile::adobe_rgb().to_icc());
    }

    #[test]
    fn assign_retags_without_changing_pixels() {
        let img = rgb8(&[[1, 2, 3], [4, 5, 6]]);
        let (out, profile) = assign(&img, Profile::adobe_rgb());
        assert_eq!(out, img);
        assert_eq!(profile.0.color_space(), lcms2::ColorSpaceSignature::RgbData);
    }

    #[test]
    fn policy_defaults_to_preserve_and_codes_round_trip() {
        assert_eq!(Policy::default(), Policy::Preserve);
        for policy in [Policy::Preserve, Policy::Convert, Policy::Off] {
            assert_eq!(Policy::from_code(policy.to_code()), Some(policy));
        }
        assert_eq!(Policy::from_code(-1), None);
        assert_eq!(Policy::from_code(3), None);
    }

    #[test]
    fn convert_space_maps_rgb_to_cmyk_and_gray() {
        let cmyk = Profile::coated_cmyk();
        assert_eq!(cmyk.channels(), Some(4));
        assert_eq!(Profile::gray_gamma22().channels(), Some(1));
        let rgb = [255, 255, 255, 0, 0, 0, 0, 180, 255];
        let inks = convert_space(
            &Profile::srgb(),
            &cmyk,
            &rgb,
            Intent::RelativeColorimetric,
            true,
        )
        .unwrap();
        assert_eq!(inks.len(), 12);
        assert!(inks[..4].iter().all(|&v| v <= 2), "white carries no ink");
        assert!(inks[7] > 200, "black carries heavy K");
        assert!(inks[8] > inks[10], "a cyan-blue is more cyan than yellow");
        let gray = convert_space(
            &Profile::srgb(),
            &Profile::gray_gamma22(),
            &rgb,
            Intent::RelativeColorimetric,
            false,
        )
        .unwrap();
        assert_eq!(gray.len(), 3);
        assert!(gray[0] > 250 && gray[1] < 5);
        assert!(convert_space(
            &Profile::srgb(),
            &cmyk,
            &rgb[..4],
            Intent::Perceptual,
            false
        )
        .is_err());
    }

    #[test]
    fn v2_serialization_keeps_the_colour_space() {
        for profile in [
            Profile::srgb(),
            Profile::coated_cmyk(),
            Profile::gray_gamma22(),
        ] {
            let v2 = profile.to_icc_v2();
            assert_eq!(v2[8], 2, "major version byte");
            let reparsed = Profile::from_icc(&v2).expect("v2 bytes parse");
            assert_eq!(reparsed.channels(), profile.channels());
        }
    }
}
