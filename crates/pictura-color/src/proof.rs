//! Soft-proof simulation (`color/proof-setup`).
//!
//! A proof preview simulates an output device on the sRGB display: the input
//! and output profile are the working sRGB space, and the device is supplied as
//! the lcms2 proofing profile. The returned pixels keep their alpha unchanged.

use lcms2::{DisallowCache, Flags, GlobalContext, Transform};

use super::{formats, ColorError, Intent, Profile};

/// The active proof setup: the device to simulate and how to simulate it.
pub struct ProofSetup {
    pub device: Profile,
    pub intent: Intent,
    pub bpc: bool,
    pub simulate_paper: bool,
    pub simulate_black: bool,
}

impl ProofSetup {
    /// Build a setup from device ICC bytes, rejecting an unreadable profile.
    pub fn from_device_icc(
        icc: &[u8],
        intent: Intent,
        bpc: bool,
        simulate_paper: bool,
        simulate_black: bool,
    ) -> Result<Self, ColorError> {
        let device = Profile::from_icc(icc)?;
        Ok(Self {
            device,
            intent,
            bpc,
            simulate_paper,
            simulate_black,
        })
    }
}

/// A reusable proofing transform over interleaved RGBA8 pixels.
pub struct ProofPreview {
    transform: Transform<u8, u8, GlobalContext, DisallowCache>,
}

impl ProofPreview {
    /// Build the proofing transform for `setup`.
    pub fn new(setup: &ProofSetup) -> Result<Self, ColorError> {
        let srgb = Profile::srgb();
        // ponytail: lcms2 has no separate paper/black-ink switches. Paper
        // simulation uses absolute proofing; black-ink simulation turns BPC
        // off. Tune here if a measured press reference needs different flags.
        let proofing_intent = if setup.simulate_paper {
            Intent::AbsoluteColorimetric
        } else {
            setup.intent
        };
        let mut flags = Flags::NO_CACHE | Flags::SOFT_PROOFING | Flags::COPY_ALPHA;
        if setup.bpc && !setup.simulate_black {
            flags = flags | Flags::BLACKPOINT_COMPENSATION;
        }
        let format = formats(4, 8)?.0;
        Transform::new_proofing_context(
            GlobalContext::new(),
            &srgb.0,
            format,
            &srgb.0,
            format,
            &setup.device.0,
            setup.intent.to_lcms(),
            proofing_intent.to_lcms(),
            flags,
        )
        .map(|transform| Self { transform })
        .map_err(|e| ColorError::Unsupported(e.to_string()))
    }

    /// Transform the RGB bytes in place; alpha is copied through.
    pub fn apply(&self, rgba: &mut [u8]) {
        if rgba.len().is_multiple_of(4) {
            self.transform.transform_in_place(rgba);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lcms2::{Flags, PixelFormat, Transform};

    fn setup(simulate_paper: bool, simulate_black: bool) -> ProofSetup {
        ProofSetup {
            device: Profile::coated_cmyk(),
            intent: Intent::RelativeColorimetric,
            bpc: true,
            simulate_paper,
            simulate_black,
        }
    }

    fn preview(setup: &ProofSetup, pixel: [u8; 4]) -> [u8; 4] {
        let preview = ProofPreview::new(setup).expect("proof transform");
        let mut out = pixel;
        preview.apply(&mut out);
        out
    }

    #[test]
    fn proof_preview_changes_colors() {
        let pixel = [255, 0, 0, 255];
        let proofed = preview(&setup(false, false), pixel);
        assert_ne!(proofed, pixel);
        assert_eq!(proofed[3], 255);

        let srgb = Profile::srgb();
        let proofing = Profile::coated_cmyk();
        let transform: Transform<u8, u8> = Transform::new_proofing(
            &srgb.0,
            PixelFormat::RGBA_8,
            &srgb.0,
            PixelFormat::RGBA_8,
            &proofing.0,
            Intent::RelativeColorimetric.to_lcms(),
            Intent::RelativeColorimetric.to_lcms(),
            Flags::SOFT_PROOFING | Flags::COPY_ALPHA | Flags::BLACKPOINT_COMPENSATION,
        )
        .expect("direct proof transform");
        let mut expected = pixel;
        transform.transform_in_place(&mut expected);
        assert_eq!(proofed, expected);
    }

    #[test]
    fn proof_simulations_differ() {
        let pixel = [180, 60, 30, 255];
        let plain = preview(&setup(false, false), pixel);
        let paper = preview(&setup(true, false), pixel);
        let black = preview(&setup(false, true), pixel);
        assert_ne!(plain, paper);
        assert_ne!(plain, black);
        assert_ne!(paper, black);
    }

    #[test]
    fn proof_alpha_untouched() {
        let proofed = preview(&setup(false, false), [200, 100, 50, 128]);
        assert_eq!(proofed[3], 128);
        assert_ne!(&proofed[..3], &[200u8, 100, 50][..]);
    }

    #[test]
    fn proof_invalid_profile_errors() {
        assert!(matches!(
            ProofSetup::from_device_icc(&[], Intent::RelativeColorimetric, true, false, false),
            Err(ColorError::InvalidProfile)
        ));
    }
}
