//! Video family (`FILT-001`): NTSC Colors.
//!
//! The gamut math lives in [`pictura_color::ntsc`]; this module is the planar
//! buffer pass and the filter error contract.

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

/// Clamp every pixel's colour into the NTSC video-safe gamut, leaving alpha
/// untouched. Deterministic and pointwise, so no region apron is needed.
pub fn ntsc_colors(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    validate(buf)?;
    let n = buf.pixel_count();
    for i in 0..n {
        let rgb = [
            f64::from(buf.data[i]) / 255.0,
            f64::from(buf.data[n + i]) / 255.0,
            f64::from(buf.data[2 * n + i]) / 255.0,
        ];
        let out = pictura_color::ntsc::clamp_to_ntsc_gamut(rgb);
        for (c, &value) in out.iter().enumerate() {
            buf.data[c * n + i] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgb: [u8; 3], alpha: u8) -> PixelBuffer {
        let mut data = vec![0u8; 4];
        data[..3].copy_from_slice(&rgb);
        data[3] = alpha;
        PixelBuffer {
            width: 1,
            height: 1,
            channels: 4,
            data: data.into(),
        }
    }

    #[test]
    fn ntsc_colors_desaturates_a_saturated_pixel() {
        let mut buf = pixel([255, 0, 0], 255);
        ntsc_colors(&mut buf).expect("apply");
        assert!(buf.data[0] < 255, "red was not reduced: {:?}", buf.data);
        assert_eq!(buf.data[1], buf.data[2], "green and blue diverged");
        assert!(buf.data[1] > 0, "the result lost all colour");
    }

    #[test]
    fn ntsc_colors_leaves_in_gamut_pixels_alone() {
        for rgb in [[255, 255, 255], [128, 128, 128], [150, 100, 120]] {
            let mut buf = pixel(rgb, 255);
            let before = buf.data.to_vec();
            ntsc_colors(&mut buf).expect("apply");
            assert_eq!(buf.data.as_ref(), before.as_slice(), "{rgb:?}");
        }
    }

    #[test]
    fn ntsc_colors_preserves_alpha() {
        let mut buf = pixel([255, 0, 0], 73);
        ntsc_colors(&mut buf).expect("apply");
        assert_eq!(buf.data[3], 73);
    }

    #[test]
    fn ntsc_colors_refuses_malformed_buffers() {
        let mut two_channel = PixelBuffer {
            width: 1,
            height: 1,
            channels: 2,
            data: vec![255, 0].into(),
        };
        let before = two_channel.data.to_vec();
        assert!(matches!(
            ntsc_colors(&mut two_channel),
            Err(FilterError::Unsupported(_))
        ));
        assert_eq!(two_channel.data.as_ref(), before.as_slice());

        let mut empty = PixelBuffer {
            width: 0,
            height: 0,
            channels: 3,
            data: Vec::new().into(),
        };
        assert!(matches!(
            ntsc_colors(&mut empty),
            Err(FilterError::InvalidParams(_))
        ));

        let mut short = PixelBuffer {
            width: 2,
            height: 2,
            channels: 3,
            data: vec![0; 3].into(),
        };
        assert!(matches!(
            ntsc_colors(&mut short),
            Err(FilterError::InvalidParams(_))
        ));
    }
}
