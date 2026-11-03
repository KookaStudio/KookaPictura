//! Video family (`FILT-001`): NTSC Colors and De-Interlace.
//!
//! The gamut math lives in [`pictura_color::ntsc`]; this module is the planar
//! buffer pass and the filter error contract. De-Interlace is a row-pair pass
//! that reconstructs one interlaced field from the retained one.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::{validate, FilterError};

/// Which field a De-Interlace run discards and reconstructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterlaceField {
    /// Rows 1, 3, 5, … (0-based).
    Odd,
    /// Rows 0, 2, 4, … (0-based), so the buffer's first row is in this field.
    Even,
}

/// How a De-Interlace run forms the discarded field's new rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeinterlaceMethod {
    /// The rounded mean of the retained rows above and below.
    Interpolation,
    /// A copy of the retained row above.
    Duplication,
}

impl InterlaceField {
    fn eliminates(self, y: usize) -> bool {
        match self {
            Self::Odd => !y.is_multiple_of(2),
            Self::Even => y.is_multiple_of(2),
        }
    }
}

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

/// Reconstruct the field this run discards from the retained one, in place.
///
/// Row parity is the buffer's own 0-based row index, so the pass depends on
/// where the buffer starts; the catalogue row previews the whole layer for that
/// reason. A retained row is never written, so it stays bit-identical. Every
/// channel plane is reconstructed, including a fourth (alpha) plane.
pub fn de_interlace(
    buf: &mut PixelBuffer,
    field: InterlaceField,
    method: DeinterlaceMethod,
) -> Result<(), FilterError> {
    let n = validate(buf)?;
    let w = buf.width as usize;
    let h = buf.height as usize;
    if h < 2 {
        return Ok(());
    }
    let planes = buf.channels as usize;
    for plane in 0..planes {
        let base = plane * n;
        let src = buf.data[base..base + n].to_vec();
        buf.data[base..base + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                if !field.eliminates(y) {
                    return;
                }
                let above = retained_above(y);
                let below = retained_below(y, h);
                for (x, out) in row.iter_mut().enumerate() {
                    *out = match method {
                        DeinterlaceMethod::Interpolation => {
                            let a = u16::from(src[above * w + x]);
                            let b = u16::from(src[below * w + x]);
                            (a + b).div_ceil(2) as u8
                        }
                        DeinterlaceMethod::Duplication => src[above * w + x],
                    };
                }
            });
    }
    Ok(())
}

/// The retained row above `y`; the first row has none, so it uses the row below.
fn retained_above(y: usize) -> usize {
    if y == 0 {
        1
    } else {
        y - 1
    }
}

/// The retained row below `y`; the last row has none, so it uses the row above.
fn retained_below(y: usize, h: usize) -> usize {
    if y + 1 >= h {
        h - 2
    } else {
        y + 1
    }
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

    /// A `width`×`height` buffer whose every row and plane is distinct.
    fn pattern(width: u32, height: u32, channels: u8) -> PixelBuffer {
        let n = (width * height) as usize;
        let mut data = vec![0u8; n * channels as usize];
        for c in 0..channels as usize {
            for i in 0..n {
                data[c * n + i] = ((i * i + 7 * i + 31 * c) % 251) as u8;
            }
        }
        PixelBuffer {
            width,
            height,
            channels,
            data: data.into(),
        }
    }

    /// A one-column buffer whose every plane repeats `rows`.
    fn rows(rows: &[u8], channels: u8) -> PixelBuffer {
        let n = rows.len();
        let mut data = vec![0u8; n * channels as usize];
        for c in 0..channels as usize {
            for (y, &value) in rows.iter().enumerate() {
                data[c * n + y] = value;
            }
        }
        PixelBuffer {
            width: 1,
            height: rows.len() as u32,
            channels,
            data: data.into(),
        }
    }

    fn mean(a: u8, b: u8) -> u8 {
        (u16::from(a) + u16::from(b)).div_ceil(2) as u8
    }

    /// The retained neighbours the kernel uses, mirrored here so a test can
    /// state the expectation independently of the implementation.
    fn neighbours(y: usize, h: usize) -> (usize, usize) {
        let above = if y == 0 { 1 } else { y - 1 };
        let below = if y + 1 >= h { h - 2 } else { y + 1 };
        (above, below)
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

    #[test]
    fn de_interlace_reconstructs_the_odd_field() {
        let mut buf = pattern(3, 6, 3);
        let before = buf.data.to_vec();
        de_interlace(
            &mut buf,
            InterlaceField::Odd,
            DeinterlaceMethod::Interpolation,
        )
        .expect("apply");
        let (w, n) = (3usize, 18usize);
        for c in 0..3usize {
            for y in 0..6usize {
                for x in 0..w {
                    let i = c * n + y * w + x;
                    if y % 2 == 0 {
                        assert_eq!(buf.data[i], before[i], "row {y} is retained");
                    } else {
                        let (above, below) = neighbours(y, 6);
                        let a = before[c * n + above * w + x];
                        let b = before[c * n + below * w + x];
                        assert_eq!(buf.data[i], mean(a, b), "row {y} plane {c}");
                    }
                }
            }
        }
    }

    #[test]
    fn de_interlace_reconstructs_the_even_field_and_its_first_row() {
        let mut buf = pattern(3, 6, 3);
        let before = buf.data.to_vec();
        de_interlace(
            &mut buf,
            InterlaceField::Even,
            DeinterlaceMethod::Interpolation,
        )
        .expect("apply");
        let (w, n) = (3usize, 18usize);
        for c in 0..3usize {
            for y in 0..6usize {
                for x in 0..w {
                    let i = c * n + y * w + x;
                    if y % 2 == 1 {
                        assert_eq!(buf.data[i], before[i], "row {y} is retained");
                    } else {
                        let (above, below) = neighbours(y, 6);
                        let a = before[c * n + above * w + x];
                        let b = before[c * n + below * w + x];
                        assert_eq!(buf.data[i], mean(a, b), "row {y} plane {c}");
                    }
                }
            }
        }
    }

    #[test]
    fn de_interlace_interpolation_averages_and_rounds_up() {
        let mut buf = rows(&[10, 0, 15], 3);
        de_interlace(
            &mut buf,
            InterlaceField::Odd,
            DeinterlaceMethod::Interpolation,
        )
        .expect("apply");
        assert_eq!(buf.data[1], 13, "10 and 15 average to 12.5, rounded up");

        // A top-edge row has one retained neighbour, so the mean is that row.
        let mut top = rows(&[0, 40, 0], 3);
        de_interlace(
            &mut top,
            InterlaceField::Even,
            DeinterlaceMethod::Interpolation,
        )
        .expect("apply");
        assert_eq!(top.data[0], 40, "the first row takes the row below");
        assert_eq!(top.data[2], 40, "the last row takes the row above");
    }

    #[test]
    fn de_interlace_duplication_copies_the_row_above() {
        let mut odd = rows(&[10, 0, 20, 0, 30], 3);
        de_interlace(
            &mut odd,
            InterlaceField::Odd,
            DeinterlaceMethod::Duplication,
        )
        .expect("apply");
        assert_eq!(odd.data[1], 10);
        assert_eq!(odd.data[3], 20);

        let mut even = rows(&[0, 40, 0, 50, 0], 3);
        de_interlace(
            &mut even,
            InterlaceField::Even,
            DeinterlaceMethod::Duplication,
        )
        .expect("apply");
        assert_eq!(even.data[0], 40, "the first row takes the row below");
        assert_eq!(even.data[2], 40);
        assert_eq!(even.data[4], 50);
    }

    #[test]
    fn de_interlace_reconstructs_alpha() {
        let mut buf = pattern(2, 4, 4);
        let before = buf.data.to_vec();
        de_interlace(
            &mut buf,
            InterlaceField::Odd,
            DeinterlaceMethod::Interpolation,
        )
        .expect("apply");
        let (w, n) = (2usize, 8usize);
        let alpha = 3 * n;
        for y in 0..4usize {
            for x in 0..w {
                let i = alpha + y * w + x;
                if y % 2 == 0 {
                    assert_eq!(buf.data[i], before[i], "row {y} keeps its alpha");
                } else {
                    let (above, below) = neighbours(y, 4);
                    let a = before[alpha + above * w + x];
                    let b = before[alpha + below * w + x];
                    assert_eq!(buf.data[i], mean(a, b), "row {y} alpha");
                }
            }
        }
        assert_ne!(
            buf.data[alpha + 2],
            before[alpha + 2],
            "the alpha plane moved on an eliminated row"
        );
    }

    #[test]
    fn de_interlace_refuses_malformed_buffers() {
        let mut two_channel = PixelBuffer {
            width: 1,
            height: 2,
            channels: 2,
            data: vec![255, 0, 255, 0].into(),
        };
        let before = two_channel.data.to_vec();
        assert!(matches!(
            de_interlace(
                &mut two_channel,
                InterlaceField::Odd,
                DeinterlaceMethod::Interpolation
            ),
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
            de_interlace(
                &mut empty,
                InterlaceField::Odd,
                DeinterlaceMethod::Interpolation
            ),
            Err(FilterError::InvalidParams(_))
        ));

        let mut short = PixelBuffer {
            width: 2,
            height: 2,
            channels: 3,
            data: vec![0; 3].into(),
        };
        assert!(matches!(
            de_interlace(
                &mut short,
                InterlaceField::Odd,
                DeinterlaceMethod::Interpolation
            ),
            Err(FilterError::InvalidParams(_))
        ));
    }

    #[test]
    fn de_interlace_leaves_a_single_row_unchanged() {
        for field in [InterlaceField::Odd, InterlaceField::Even] {
            for method in [
                DeinterlaceMethod::Interpolation,
                DeinterlaceMethod::Duplication,
            ] {
                let mut buf = pattern(2, 1, 4);
                let before = buf.data.to_vec();
                de_interlace(&mut buf, field, method).expect("apply");
                assert_eq!(buf.data.as_ref(), before.as_slice(), "{field:?} {method:?}");
            }
        }
    }
}
