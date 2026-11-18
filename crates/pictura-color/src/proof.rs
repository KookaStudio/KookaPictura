//! Soft-proof simulation (`color/proof-setup`).
//!
//! A proof preview simulates an output device on the sRGB display: the input
//! and output profile are the working sRGB space, and the device is supplied as
//! the lcms2 proofing profile. The returned pixels keep their alpha unchanged.

use lcms2::{DisallowCache, Flags, GlobalContext, PixelFormat, ThreadContext, Transform};

use super::{formats, space_format, ColorError, Intent, Profile};

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

/// A proofing transform whose output is the printer's device colour space.
pub struct HardProof {
    transform: Transform<u8, u8, GlobalContext, DisallowCache>,
    source_components: usize,
    output_components: usize,
}

impl HardProof {
    pub fn new(
        source: &Profile,
        output: &Profile,
        intent: Intent,
        bpc: bool,
        setup: &ProofSetup,
    ) -> Result<Self, ColorError> {
        let (source_format, source_components) = space_format(source)?;
        let (output_format, output_components) = space_format(output)?;
        let proof_intent = if setup.simulate_paper {
            Intent::AbsoluteColorimetric
        } else {
            setup.intent
        };
        let mut flags = Flags::NO_CACHE | Flags::SOFT_PROOFING;
        if bpc && setup.bpc && !setup.simulate_black {
            flags = flags | Flags::BLACKPOINT_COMPENSATION;
        }
        let transform = Transform::new_proofing_context(
            GlobalContext::new(),
            &source.0,
            source_format,
            &output.0,
            output_format,
            &setup.device.0,
            intent.to_lcms(),
            proof_intent.to_lcms(),
            flags,
        )
        .map_err(|error| ColorError::Unsupported(error.to_string()))?;
        Ok(Self {
            transform,
            source_components,
            output_components,
        })
    }

    pub fn convert(&self, samples: &[u8]) -> Result<Vec<u8>, ColorError> {
        if !samples.len().is_multiple_of(self.source_components) {
            return Err(ColorError::Unsupported("incomplete print pixel".into()));
        }
        let mut output = vec![0; samples.len() / self.source_components * self.output_components];
        self.transform.transform_pixels(samples, &mut output);
        Ok(output)
    }
}

/// Soft proof in sRGB, with a separate per-pixel gamut mask. Two alarm values
/// distinguish actual alarms from valid pixels with the same colour.
pub fn print_gamut_mask(srgb: &[u8], setup: &ProofSetup) -> Result<Vec<bool>, ColorError> {
    print_gamut_mask_source(srgb, &Profile::srgb(), setup)
}

pub fn print_gamut_mask_source(
    samples: &[u8],
    source: &Profile,
    setup: &ProofSetup,
) -> Result<Vec<bool>, ColorError> {
    let (format, components) = space_format(source)?;
    if !samples.len().is_multiple_of(components) {
        return Err(ColorError::Unsupported("incomplete source pixel".into()));
    }
    let icc = setup.device.to_icc();
    let source_icc = source.to_icc();
    let transform = |alarm| -> Result<Vec<u16>, ColorError> {
        let mut context = ThreadContext::new();
        context.set_alarm_codes([alarm; 16]);
        let display = lcms2::Profile::new_srgb_context(&context);
        let source = lcms2::Profile::new_icc_context(&context, &source_icc)
            .map_err(|_| ColorError::InvalidProfile)?;
        let device = lcms2::Profile::new_icc_context(&context, &icc)
            .map_err(|_| ColorError::InvalidProfile)?;
        let mut flags = Flags::NO_CACHE | Flags::SOFT_PROOFING | Flags::GAMUT_CHECK;
        if setup.bpc {
            flags = flags | Flags::BLACKPOINT_COMPENSATION;
        }
        let transform: Transform<u8, u16, ThreadContext, DisallowCache> =
            Transform::new_proofing_context(
                &context,
                &source,
                format,
                &display,
                PixelFormat::RGB_16,
                &device,
                setup.intent.to_lcms(),
                setup.intent.to_lcms(),
                flags,
            )
            .map_err(|error| ColorError::Unsupported(error.to_string()))?;
        let mut output = vec![0; samples.len() / components * 3];
        transform.transform_pixels(samples, &mut output);
        Ok(output)
    };
    let a = transform(0x1357)?;
    let b = transform(0xeac0)?;
    Ok(a.chunks_exact(3)
        .zip(b.chunks_exact(3))
        .map(|(a, b)| a != b)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(simulate_paper: bool, simulate_black: bool) -> ProofSetup {
        ProofSetup {
            device: Profile::coated_cmyk(),
            intent: Intent::RelativeColorimetric,
            bpc: true,
            simulate_paper,
            simulate_black,
        }
    }

    #[test]
    fn hard_proof_outputs_printer_device_samples() {
        let source = Profile::srgb();
        let output = Profile::coated_cmyk();
        let setup = ProofSetup {
            device: Profile::gray_gamma22(),
            intent: Intent::RelativeColorimetric,
            bpc: true,
            simulate_paper: true,
            simulate_black: false,
        };
        let samples = [240, 30, 100, 22, 192, 66, 2, 3, 20];
        let reference: Transform<u8, u8> = Transform::new_proofing(
            &source.0,
            PixelFormat::RGB_8,
            &output.0,
            PixelFormat::CMYK_8,
            &setup.device.0,
            lcms2::Intent::Perceptual,
            lcms2::Intent::AbsoluteColorimetric,
            Flags::SOFT_PROOFING | Flags::BLACKPOINT_COMPENSATION,
        )
        .unwrap();
        let mut expected = [0; 12];
        reference.transform_pixels(&samples, &mut expected);
        let job = HardProof::new(&source, &output, Intent::Perceptual, true, &setup).unwrap();
        assert_eq!(job.convert(&samples).unwrap(), expected);
        assert!(job.convert(&samples[..2]).is_err());
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
