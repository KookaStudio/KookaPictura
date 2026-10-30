//! A reusable 8-bit transform between profiles of different colour spaces.

use lcms2::{ColorSpaceSignature, DisallowCache, Flags, GlobalContext, PixelFormat, Transform};

use crate::{ColorError, Intent, Profile};

/// The 8-bit interleaved pixel format and channel count of `profile`'s colour
/// space. Lab and other spaces are not accepted.
pub(crate) fn space_format(profile: &Profile) -> Result<(PixelFormat, usize), ColorError> {
    match profile.0.color_space() {
        ColorSpaceSignature::GrayData => Ok((PixelFormat::GRAY_8, 1)),
        ColorSpaceSignature::RgbData => Ok((PixelFormat::RGB_8, 3)),
        ColorSpaceSignature::CmykData => Ok((PixelFormat::CMYK_8, 4)),
        other => Err(ColorError::Unsupported(format!("colour space {other:?}"))),
    }
}

/// [`crate::convert_space`] held open: the transform is built once and applied
/// to any number of pixels. A caller converting one colour per mouse move (the
/// colour picker) keeps one of these instead of rebuilding the transform for
/// every pixel. Like [`crate::ByteTransform`] it disables lcms2's one-pixel
/// cache, so it is `Sync`.
pub struct SpaceTransform {
    transform: Transform<u8, u8, GlobalContext, DisallowCache>,
}

impl SpaceTransform {
    pub fn new(
        src: &Profile,
        dst: &Profile,
        intent: Intent,
        black_point_compensation: bool,
    ) -> Result<Self, ColorError> {
        let (in_format, _) = space_format(src)?;
        let (out_format, _) = space_format(dst)?;
        let mut flags = Flags::NO_CACHE;
        if black_point_compensation {
            flags = flags | Flags::BLACKPOINT_COMPENSATION;
        }
        Transform::new_flags_context(
            GlobalContext::new(),
            &src.0,
            in_format,
            &dst.0,
            out_format,
            intent.to_lcms(),
            flags,
        )
        .map(|transform| SpaceTransform { transform })
        .map_err(|e| ColorError::Unsupported(e.to_string()))
    }

    /// Convert the interleaved pixels in `data` into `out`, which holds the
    /// same number of pixels in the destination profile's channel count.
    pub fn apply(&self, data: &[u8], out: &mut [u8]) {
        self.transform.transform_pixels(data, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The reference is a fresh, cache-enabled transform per pixel: what
    // `convert_space` built before it held a `SpaceTransform`.
    fn one_shot(src: &Profile, dst: &Profile, data: &[u8], out_channels: usize) -> Vec<u8> {
        let (in_format, _) = space_format(src).expect("src format");
        let (out_format, _) = space_format(dst).expect("dst format");
        let transform: Transform<u8, u8> = Transform::new_flags(
            &src.0,
            in_format,
            &dst.0,
            out_format,
            Intent::RelativeColorimetric.to_lcms(),
            Flags::BLACKPOINT_COMPENSATION,
        )
        .expect("one-shot transform");
        let mut out = vec![0u8; out_channels];
        transform.transform_pixels(data, &mut out);
        out
    }

    #[test]
    fn space_transform_matches_convert_space() {
        fn assert_shareable<T: Sync + Send>() {}
        assert_shareable::<SpaceTransform>();

        let srgb = Profile::srgb();
        let cmyk = Profile::coated_cmyk();
        let intent = Intent::RelativeColorimetric;
        let to_cmyk = SpaceTransform::new(&srgb, &cmyk, intent, true).expect("rgb -> cmyk");
        let to_rgb = SpaceTransform::new(&cmyk, &srgb, intent, true).expect("cmyk -> rgb");

        for r in (0..=255u8).step_by(85) {
            for g in (0..=255u8).step_by(85) {
                for b in (0..=255u8).step_by(85) {
                    let rgb = [r, g, b];
                    let mut inks = [0u8; 4];
                    to_cmyk.apply(&rgb, &mut inks);
                    assert_eq!(inks.to_vec(), one_shot(&srgb, &cmyk, &rgb, 4), "{rgb:?}");

                    let mut back = [0u8; 3];
                    to_rgb.apply(&inks, &mut back);
                    assert_eq!(back.to_vec(), one_shot(&cmyk, &srgb, &inks, 3), "{inks:?}");
                }
            }
        }
    }
}
