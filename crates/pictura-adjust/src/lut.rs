//! Color Lookup (`clrL`): a `.CUBE` parser and a trilinear 3-D LUT sampler.
//!
//! The docs contract (`docs/05-layers/adjustment-layers.md`) names
//! `pictura_adjust::lut` for the `.cube` loader and sampler. Only the `.CUBE`
//! text format is parsed; `.3DL`/`.LOOK` and the ICC-based lookup kinds are left
//! to the caller as a no-op.
//!
//! ponytail: `.CUBE` point order is red-fastest, which the file format fixes;
//! `dataOrder`/`tableOrder` are ignored because a `.CUBE` is self-describing, and
//! a non-default `DOMAIN_MIN`/`DOMAIN_MAX` is treated as `0..1`. Upgrade to
//! honoring them only if a real file disagrees.

use pictura_core::PixelBuffer;

use crate::common::{map_rgb, validate};
use crate::types::{AdjustError, ColorLookupParams, Lut3d};

/// Parse the `.CUBE` text format. Returns `None` for a missing or out-of-range
/// `LUT_3D_SIZE`, a wrong point count, or a non-finite component; never panics.
pub fn parse_cube(bytes: &[u8]) -> Option<Lut3d> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut size: Option<usize> = None;
    let mut points: Vec<[f32; 3]> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("TITLE") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
            let n: usize = rest.trim().parse().ok()?;
            if !(2..=64).contains(&n) {
                return None;
            }
            size = Some(n);
            continue;
        }
        if line.starts_with("DOMAIN_MIN") || line.starts_with("DOMAIN_MAX") {
            continue;
        }
        size?;
        let mut parts = line.split_whitespace();
        let mut rgb = [0.0f32; 3];
        for component in &mut rgb {
            *component = parts.next()?.parse().ok()?;
            if !component.is_finite() {
                return None;
            }
        }
        if parts.next().is_some() {
            return None;
        }
        points.push(rgb);
    }
    let size = size?;
    if points.len() != size * size * size {
        return None;
    }
    Some(Lut3d { size, points })
}

/// Linear index of node `(r, g, b)`; red varies fastest.
fn point_index(lut: &Lut3d, r: usize, g: usize, b: usize) -> usize {
    r + g * lut.size + b * lut.size * lut.size
}

fn node(lut: &Lut3d, r: usize, g: usize, b: usize) -> [f32; 3] {
    lut.points[point_index(lut, r, g, b)]
}

/// Trilinear sample of `lut` at an RGB position in `0.0..=1.0`.
pub(crate) fn sample(lut: &Lut3d, rgb: [f32; 3]) -> [f32; 3] {
    let max = (lut.size - 1) as f32;
    let mut lo = [0usize; 3];
    let mut hi = [0usize; 3];
    let mut fr = [0.0f32; 3];
    for c in 0..3 {
        let x = rgb[c].clamp(0.0, 1.0) * max;
        lo[c] = x.floor() as usize;
        hi[c] = (lo[c] + 1).min(lut.size - 1);
        fr[c] = x - lo[c] as f32;
    }
    let mut out = [0.0f32; 3];
    for (c, value) in out.iter_mut().enumerate() {
        let mut acc = 0.0;
        for (bi, &b) in [lo[2], hi[2]].iter().enumerate() {
            for (gi, &g) in [lo[1], hi[1]].iter().enumerate() {
                for (ri, &r) in [lo[0], hi[0]].iter().enumerate() {
                    let w = (if ri == 0 { 1.0 - fr[0] } else { fr[0] })
                        * (if gi == 0 { 1.0 - fr[1] } else { fr[1] })
                        * (if bi == 0 { 1.0 - fr[2] } else { fr[2] });
                    acc += w * node(lut, r, g, b)[c];
                }
            }
        }
        *value = acc;
    }
    out
}

/// `clrL`: sample the parsed lookup per pixel, or leave the buffer unchanged
/// when there is no lookup. An invalid `Lut3d` is `InvalidParams`.
pub(crate) fn color_lookup(
    params: &ColorLookupParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    let Some(lut) = &params.lookup else {
        return Ok(());
    };
    if !(2..=64).contains(&lut.size) || lut.points.len() != lut.size * lut.size * lut.size {
        return Err(AdjustError::InvalidParams("invalid 3-D LUT size".into()));
    }
    if lut.points.iter().flatten().any(|v| !v.is_finite()) {
        return Err(AdjustError::InvalidParams(
            "non-finite LUT component".into(),
        ));
    }
    map_rgb(buf, n, |r, g, b| {
        let sampled = sample(lut, [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
        [to_u8(sampled[0]), to_u8(sampled[1]), to_u8(sampled[2])]
    });
    Ok(())
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Run [`color_lookup`] on a validated buffer; used by `apply`.
pub(crate) fn apply_color_lookup(
    params: &ColorLookupParams,
    buf: &mut PixelBuffer,
) -> Result<(), AdjustError> {
    let n = validate(buf)?;
    color_lookup(params, buf, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(size: usize, rows: &[[f32; 3]]) -> Vec<u8> {
        let mut s = format!("LUT_3D_SIZE {size}\n");
        for row in rows {
            s.push_str(&format!("{} {} {}\n", row[0], row[1], row[2]));
        }
        s.into_bytes()
    }

    fn identity_rows(size: usize) -> Vec<[f32; 3]> {
        let max = (size - 1) as f32;
        let mut rows = Vec::new();
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    rows.push([r as f32 / max, g as f32 / max, b as f32 / max]);
                }
            }
        }
        rows
    }

    #[test]
    fn parses_identity_cube_and_ignores_noise() {
        let text = b"# comment\n\nTITLE \"x\"\nLUT_3D_SIZE 2\n".to_vec();
        let mut bytes = text;
        for row in identity_rows(2) {
            bytes.extend_from_slice(format!("{} {} {}\n", row[0], row[1], row[2]).as_bytes());
        }
        let lut = parse_cube(&bytes).expect("parses");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.points.len(), 8);
    }

    #[test]
    fn rejects_bad_size_and_bad_count() {
        assert_eq!(parse_cube(&cube(1, &[[0.0, 0.0, 0.0]])), None);
        assert_eq!(parse_cube(&cube(65, &[])), None);
        let short = b"LUT_3D_SIZE 2\n0 0 0\n".to_vec();
        assert_eq!(parse_cube(&short), None);
        let nan =
            b"LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 nan 1\n".to_vec();
        assert_eq!(parse_cube(&nan), None);
        assert_eq!(parse_cube(b"0 0 0\n"), None);
    }

    #[test]
    fn identity_cube_is_the_identity() {
        let lut = parse_cube(&cube(2, &identity_rows(2))).unwrap();
        assert_eq!(sample(&lut, [0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(sample(&lut, [1.0, 1.0, 1.0]), [1.0, 1.0, 1.0]);
        let mid = sample(&lut, [0.5, 0.5, 0.5]);
        for c in mid {
            assert!((c - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn point_order_is_red_fastest() {
        let mut rows = vec![[0.0f32; 3]; 27];
        rows[point_index(
            &Lut3d {
                size: 3,
                points: vec![[0.0; 3]; 27],
            },
            1,
            0,
            0,
        )] = [1.0, 0.0, 0.0];
        let lut = parse_cube(&cube(3, &rows)).unwrap();
        assert_eq!(sample(&lut, [0.5, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        assert_eq!(sample(&lut, [0.0, 0.5, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(sample(&lut, [0.0, 0.0, 0.5]), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn inverting_cube_maps_extremes() {
        let mut rows = Vec::new();
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    rows.push([(1 - r) as f32, (1 - g) as f32, (1 - b) as f32]);
                }
            }
        }
        let lut = parse_cube(&cube(2, &rows)).unwrap();
        let params = ColorLookupParams {
            kind: crate::types::ColorLookupKind::ThreeDLut,
            lookup: Some(lut),
        };
        let mut buf = PixelBuffer::new(1, 2, 4);
        buf.data = vec![0, 255, 0, 0, 0, 0, 77, 88].into();
        super::apply_color_lookup(&params, &mut buf).unwrap();
        assert_eq!(&buf.data[..2], &[255, 0]);
        assert_eq!(&buf.data[6..], &[77, 88], "alpha is preserved");
    }

    #[test]
    fn missing_lookup_is_a_noop_and_alpha_kept() {
        let params = ColorLookupParams {
            kind: crate::types::ColorLookupKind::AbstractProfile,
            lookup: None,
        };
        let mut buf = PixelBuffer::new(1, 1, 4);
        buf.data = vec![10, 20, 30, 40].into();
        super::apply_color_lookup(&params, &mut buf).unwrap();
        assert_eq!(buf.data, vec![10, 20, 30, 40]);
    }

    #[test]
    fn invalid_lut_is_rejected() {
        let mut buf = PixelBuffer::new(1, 1, 3);
        buf.data = vec![0, 0, 0].into();
        for lookup in [
            Lut3d {
                size: 3,
                points: vec![[0.0; 3]; 8],
            },
            Lut3d {
                size: 65,
                points: Vec::new(),
            },
            Lut3d {
                size: 2,
                points: vec![[f32::NAN; 3]; 8],
            },
        ] {
            let params = ColorLookupParams {
                kind: crate::types::ColorLookupKind::ThreeDLut,
                lookup: Some(lookup),
            };
            assert!(matches!(
                super::apply_color_lookup(&params, &mut buf),
                Err(AdjustError::InvalidParams(_))
            ));
        }
    }
}
