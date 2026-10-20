//! Profile-free conversions from a PSD color mode's stored planes to planar
//! 8-bit RGB. Bitmap and Indexed are exact; CMYK and Lab are approximations of
//! the reference's color-managed transforms.

use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerStack, Samples, SourceChannels,
};

/// Expand a depth-1 Bitmap plane into planar RGB. Rows are `ceil(width / 8)`
/// bytes, MSB-first; a set bit is black (0) and a clear bit is white (255).
pub fn bitmap_rows_to_rgb(bits: &[u8], width: usize, height: usize) -> Vec<u8> {
    let row_bytes = width.div_ceil(8);
    let plane = width * height;
    let mut out = vec![0u8; plane * 3];
    for y in 0..height {
        for x in 0..width {
            let byte = bits.get(y * row_bytes + x / 8).copied().unwrap_or(0);
            let value = if byte & (0x80 >> (x % 8)) != 0 {
                0
            } else {
                255
            };
            let i = y * width + x;
            out[i] = value;
            out[plane + i] = value;
            out[2 * plane + i] = value;
        }
    }
    out
}

/// Replicate one 8-bit gray plane into planar RGB.
pub(crate) fn gray_to_rgb(gray: &[u8]) -> Vec<u8> {
    let plane = gray.len();
    let mut out = Vec::with_capacity(plane * 3);
    out.extend_from_slice(gray);
    out.extend_from_slice(gray);
    out.extend_from_slice(gray);
    out
}

/// ponytail: ungrounded CMY assumption for 3-channel Multichannel — the plates
/// are treated as subtractive CMY with no black (`r = 255 - c`, etc.), not an
/// ICC-managed transform. Only N=1 and N=3 Multichannel open; other counts are
/// Unsupported.
pub(crate) fn cmy_to_rgb(cmy: &[u8]) -> Vec<u8> {
    let plane = cmy.len() / 3;
    let (c, rest) = cmy.split_at(plane);
    let (m, y) = rest.split_at(plane);
    let mut out = vec![0u8; plane * 3];
    for i in 0..plane {
        out[i] = 255 - c[i];
        out[plane + i] = 255 - m[i];
        out[2 * plane + i] = 255 - y[i];
    }
    out
}

/// Expand a single index plane through a 768-byte palette laid out as 256 red,
/// then 256 green, then 256 blue bytes.
pub fn indexed_to_rgb(indices: &[u8], palette: &[u8; 768]) -> Vec<u8> {
    let plane = indices.len();
    let mut out = vec![0u8; plane * 3];
    for (i, &index) in indices.iter().enumerate() {
        let index = index as usize;
        out[i] = palette[index];
        out[plane + i] = palette[256 + index];
        out[2 * plane + i] = palette[512 + index];
    }
    out
}

/// ponytail: profile-free approximation of the reference's ICC CMYK transform;
/// saturated colors differ. Stored 0 is full ink, 255 is no ink, so each RGB
/// channel is `floor(color * black / 255)`.
pub fn cmyk_to_rgb(cmyk: &[u8]) -> Vec<u8> {
    let plane = cmyk.len() / 4;
    let (c, rest) = cmyk.split_at(plane);
    let (m, rest) = rest.split_at(plane);
    let (y, k) = rest.split_at(plane);
    let mut out = vec![0u8; plane * 3];
    for i in 0..plane {
        out[i] = (c[i] as u16 * k[i] as u16 / 255) as u8;
        out[plane + i] = (m[i] as u16 * k[i] as u16 / 255) as u8;
        out[2 * plane + i] = (y[i] as u16 * k[i] as u16 / 255) as u8;
    }
    out
}

/// The exact profile-free right-inverse of [`cmyk_to_rgb`]: the working RGB
/// planar planes become `[R, G, B, 255]` (a full black plate, i.e. no black), so
/// `cmyk_to_rgb(rgb_to_cmyk(rgb)) == rgb` byte for byte (`v * 255 / 255 == v`).
/// This is stronger than the Lab inverse, which quantizes the 8-bit Lab bytes.
pub(crate) fn rgb_to_cmyk(rgb: &[u8]) -> Vec<u8> {
    let plane = rgb.len() / 3;
    let (r, rest) = rgb.split_at(plane);
    let (g, b) = rest.split_at(plane);
    let mut out = vec![255u8; plane * 4];
    out[..plane].copy_from_slice(r);
    out[plane..2 * plane].copy_from_slice(g);
    out[2 * plane..3 * plane].copy_from_slice(b);
    out
}

/// D50 XYZ → linear sRGB (Bradford D50→D65), shared by [`lab_to_rgb`] and
/// [`xyz_d50_to_srgb_u8`].
const XYZ_D50_TO_LINEAR_SRGB: [[f64; 3]; 3] = [
    [3.1338561, -1.6168667, -0.4906146],
    [-0.9787684, 1.9161415, 0.0334540],
    [0.0719453, -0.2289914, 1.4052427],
];

/// Profile-free CIE XYZ (D50) → 8-bit sRGB with the same matrix class as
/// [`lab_to_rgb`]; out-of-gamut components clamp.
pub fn xyz_d50_to_srgb_u8(xyz: [f64; 3]) -> [u8; 3] {
    let mut out = [0u8; 3];
    for (c, row) in XYZ_D50_TO_LINEAR_SRGB.iter().enumerate() {
        out[c] = srgb_encode(row[0] * xyz[0] + row[1] * xyz[1] + row[2] * xyz[2]);
    }
    out
}

/// ponytail: approximation of a color-managed Lab->sRGB transform. This is the
/// standard CIELAB(D50) -> sRGB(D65) conversion (D50 white through the Bradford
/// D50->D65 matrix) and matches lcms2's exact, unoptimized transform within 1
/// LSB for in-gamut colors (verified over a sweep). A renderer that precomputes
/// an optimized color LUT (psd-tools / Pillow `.convert("RGB")`) can diverge by
/// up to ~20 LSB on some in-gamut colors, so that optimized path is not the
/// reference; saturated out-of-gamut colors also clip instead of gamut-mapping.
pub fn lab_to_rgb(lab: &[u8]) -> Vec<u8> {
    const XN: f64 = 0.96422;
    const ZN: f64 = 0.82521;
    let plane = lab.len() / 3;
    let (l, rest) = lab.split_at(plane);
    let (a, b) = rest.split_at(plane);
    let mut out = vec![0u8; plane * 3];
    for i in 0..plane {
        let fy = (l[i] as f64 * 100.0 / 255.0 + 16.0) / 116.0;
        let fx = fy + (a[i] as f64 - 128.0) / 500.0;
        let fz = fy - (b[i] as f64 - 128.0) / 200.0;
        let xyz = [XN * lab_f(fx), lab_f(fy), ZN * lab_f(fz)];
        for (c, row) in XYZ_D50_TO_LINEAR_SRGB.iter().enumerate() {
            let linear = row[0] * xyz[0] + row[1] * xyz[1] + row[2] * xyz[2];
            out[c * plane + i] = srgb_encode(linear);
        }
    }
    out
}

/// The profile-free algebraic inverse of [`lab_to_rgb`]: decode sRGB, rotate the
/// linear RGB back to XYZ D50 with the inverse Bradford matrix, then express it
/// as CIELAB bytes. Same approximation class as the read side; no profile.
pub fn rgb_to_lab(rgb: &[u8]) -> Vec<u8> {
    const M_INV: [[f64; 3]; 3] = [
        [0.4360747, 0.3850649, 0.1430804],
        [0.2225045, 0.7168786, 0.0606169],
        [0.0139322, 0.0971045, 0.7141733],
    ];
    const XN: f64 = 0.96422;
    const ZN: f64 = 0.82521;
    let plane = rgb.len() / 3;
    let (r, rest) = rgb.split_at(plane);
    let (g, b) = rest.split_at(plane);
    let mut out = vec![0u8; plane * 3];
    for i in 0..plane {
        let linear = [srgb_decode(r[i]), srgb_decode(g[i]), srgb_decode(b[i])];
        let xyz = [
            M_INV[0][0] * linear[0] + M_INV[0][1] * linear[1] + M_INV[0][2] * linear[2],
            M_INV[1][0] * linear[0] + M_INV[1][1] * linear[1] + M_INV[1][2] * linear[2],
            M_INV[2][0] * linear[0] + M_INV[2][1] * linear[1] + M_INV[2][2] * linear[2],
        ];
        let fx = lab_fwd(xyz[0] / XN);
        let fy = lab_fwd(xyz[1]);
        let fz = lab_fwd(xyz[2] / ZN);
        out[i] = lab_byte((116.0 * fy - 16.0) * 255.0 / 100.0);
        out[plane + i] = lab_byte(128.0 + 500.0 * (fx - fy));
        out[2 * plane + i] = lab_byte(128.0 + 200.0 * (fy - fz));
    }
    out
}

fn lab_f(t: f64) -> f64 {
    const EPSILON: f64 = 216.0 / 24389.0;
    const KAPPA: f64 = 24389.0 / 27.0;
    let t3 = t * t * t;
    if t3 > EPSILON {
        t3
    } else {
        (116.0 * t - 16.0) / KAPPA
    }
}

/// The forward CIELAB `f` function: the exact inverse of [`lab_f`].
fn lab_fwd(t: f64) -> f64 {
    const EPSILON: f64 = 216.0 / 24389.0;
    const KAPPA: f64 = 24389.0 / 27.0;
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

fn srgb_encode(linear: f64) -> u8 {
    let value = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8
}

fn srgb_decode(value: u8) -> f64 {
    let value = value as f64 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn lab_byte(value: f64) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

/// Retain a Lab document's decoded 8-bit Lab layer color channels before
/// `normalize` converts them to RGB, so `write_psd` re-emits an unedited layer
/// exactly. Only color channels (ids `0..3`) are stored; masks and alpha are not
/// Lab-encoded. No-op at any depth other than 8.
pub(crate) fn retain_lab_layer_planes(layers: &mut LayerStack, depth: u16) {
    if depth != 8 {
        return;
    }
    for layer in layers {
        if layer.source_channels.is_none() {
            let planes: Vec<(i16, Samples)> = layer
                .channels
                .iter()
                .filter(|c| c.id >= 0 && c.id < 3)
                .map(|c| (c.id, Samples::U8(c.data.to_vec())))
                .collect();
            if planes.len() == 3 {
                layer.source_channels =
                    Some(SourceChannels::new(BitDepth::Eight, layer.rect, planes));
            }
        }
        retain_lab_layer_planes(&mut layer.children, depth);
    }
}

/// Retain a CMYK document's decoded 8-bit CMYK layer color channels before
/// `normalize` converts them to RGB, so `write_psd` re-emits an unedited layer
/// exactly. Only color channels (ids `0..4`) are stored; masks and alpha are not
/// CMYK-encoded. No-op at any depth other than 8.
pub(crate) fn retain_cmyk_layer_planes(layers: &mut LayerStack, depth: u16) {
    if depth != 8 {
        return;
    }
    for layer in layers {
        if layer.source_channels.is_none() {
            let planes: Vec<(i16, Samples)> = layer
                .channels
                .iter()
                .filter(|c| c.id >= 0 && c.id < 4)
                .map(|c| (c.id, Samples::U8(c.data.to_vec())))
                .collect();
            if planes.len() == 4 {
                layer.source_channels =
                    Some(SourceChannels::new(BitDepth::Eight, layer.rect, planes));
            }
        }
        retain_cmyk_layer_planes(&mut layer.children, depth);
    }
}

/// Retain an Indexed document's decoded 8-bit index layer channel before
/// `normalize` expands it through the palette to RGB, so `write_psd` re-emits an
/// unedited layer exactly. Only the single index channel (id `0`) is stored;
/// masks and alpha are not palette-encoded. No-op at any depth other than 8.
pub(crate) fn retain_indexed_layer_planes(layers: &mut LayerStack, depth: u16) {
    if depth != 8 {
        return;
    }
    for layer in layers {
        if layer.source_channels.is_none() {
            let planes: Vec<(i16, Samples)> = layer
                .channels
                .iter()
                .filter(|c| c.id == 0)
                .map(|c| (c.id, Samples::U8(c.data.to_vec())))
                .collect();
            if planes.len() == 1 {
                layer.source_channels =
                    Some(SourceChannels::new(BitDepth::Eight, layer.rect, planes));
            }
        }
        retain_indexed_layer_planes(&mut layer.children, depth);
    }
}

/// The retained native composite plane at `index`, narrowed to 8-bit.
///
/// `native_plane` (`write.rs`) borrows a retained native plane only when
/// narrowing it reproduces the plane the builder passes as `current`. For
/// Lab/CMYK the planes are color-encoded and the 8-bit store is the *narrowed*
/// encoding, so `current` must be this narrowed-retained plane, not
/// `rgb_to_lab`/`rgb_to_cmyk` of the working RGB. At depth 8 the store is
/// already 8-bit, so this is the identity.
pub(crate) fn composite_retained_8(doc: &Document, depth: u16, index: usize) -> Option<Vec<u8>> {
    let retained = crate::write::composite_retained(doc, depth, index)?;
    if retained.len() != doc.width as usize * doc.height as usize {
        return None;
    }
    Some(retained.narrow_to_u8())
}

/// The retained native layer channel `id`, narrowed to 8-bit (identity at
/// depth 8). See [`composite_retained_8`].
///
/// A hand-built or corrupt store can carry a plane shorter than the layer rect
/// demands; rejecting it here keeps `narrow_channel` in bounds and lets the
/// caller fall back to re-encoding, matching the old typed-error behavior.
pub(crate) fn layer_retained_8(layer: &Layer, depth: u16, id: i16) -> Option<Vec<u8>> {
    let retained = crate::write::layer_retained(layer, depth, id)?;
    let width = layer.rect.width().max(0) as usize;
    let height = layer.rect.height().max(0) as usize;
    if retained.len() != width * height {
        return None;
    }
    Some(retained.narrow_to_u8())
}

/// Replace a layer's color channels (`0..color_channels`) with converted RGB
/// planes, then recurse into the layer's children so a pixel layer nested in a
/// group is converted too (not just a top-level one). Non-color channels are
/// untouched; a layer whose color channels do not match the mode's layout is
/// left unchanged.
pub(crate) fn convert_layer_color_channels(
    layer: &mut Layer,
    mode: ColorMode,
    palette: &[u8; 768],
) {
    convert_one_layer(layer, mode, palette);
    for child in &mut layer.children {
        convert_layer_color_channels(child, mode, palette);
    }
}

fn convert_one_layer(layer: &mut Layer, mode: ColorMode, palette: &[u8; 768]) {
    let color_channels = mode.color_channels() as usize;
    if color_channels == 0 {
        // ponytail: Multichannel layer color ids are not converted — the mode has
        // no fixed color-channel count, so only the document composite is mapped.
        return;
    }
    let positions: Vec<usize> = layer
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| c.id >= 0 && (c.id as usize) < color_channels)
        .map(|(i, _)| i)
        .collect();
    if positions.len() != color_channels {
        return;
    }
    let plane = layer.channels[positions[0]].data.len();
    if positions
        .iter()
        .any(|&i| layer.channels[i].data.len() != plane)
    {
        return;
    }
    let mut planar = Vec::with_capacity(plane * color_channels);
    for &i in &positions {
        planar.extend_from_slice(&layer.channels[i].data);
    }
    let rgb = match mode {
        ColorMode::Indexed => indexed_to_rgb(&planar, palette),
        ColorMode::Cmyk => cmyk_to_rgb(&planar),
        ColorMode::Lab => lab_to_rgb(&planar),
        // A Bitmap or Duotone layer's single gray plane replicates to RGB.
        ColorMode::Bitmap | ColorMode::Duotone => gray_to_rgb(&planar),
        ColorMode::Grayscale | ColorMode::Rgb | ColorMode::Multichannel => return,
    };
    let others: Vec<Channel> = layer
        .channels
        .iter()
        .enumerate()
        .filter(|(i, _)| !positions.contains(i))
        .map(|(_, c)| c.clone())
        .collect();
    let mut channels: Vec<Channel> = (0..3)
        .map(|c| Channel {
            id: c as i16,
            data: rgb[c * plane..(c + 1) * plane].to_vec().into(),
        })
        .collect();
    channels.extend(others);
    layer.channels = channels;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_row_aa_is_alternating_black_white() {
        let rgb = bitmap_rows_to_rgb(&[0xAA], 8, 1);
        assert_eq!(&rgb[0..8], &[0, 255, 0, 255, 0, 255, 0, 255]);
        assert_eq!(&rgb[8..16], &rgb[0..8]);
        assert_eq!(&rgb[16..24], &rgb[0..8]);
    }

    #[test]
    fn bitmap_non_multiple_of_eight_width_pads_rows() {
        // 10px wide: 2 row bytes, the unused low 6 bits are ignored.
        let rgb = bitmap_rows_to_rgb(&[0b1010_0000, 0b1100_0000], 10, 1);
        let got: Vec<u8> = rgb[0..10].to_vec();
        assert_eq!(got, vec![0, 255, 0, 255, 255, 255, 255, 255, 0, 0]);
    }

    #[test]
    fn indexed_lookup_uses_non_interleaved_palette() {
        let mut palette = [0u8; 768];
        palette[5] = 10;
        palette[256 + 5] = 20;
        palette[512 + 5] = 30;
        let rgb = indexed_to_rgb(&[5, 0], &palette);
        assert_eq!(&rgb[0..2], &[10, 0]);
        assert_eq!(&rgb[2..4], &[20, 0]);
        assert_eq!(&rgb[4..6], &[30, 0]);
    }

    #[test]
    fn cmyk_known_and_black_white_values() {
        let rgb = cmyk_to_rgb(&[128, 64, 32, 200]);
        assert_eq!(&rgb[0..1], &[100]);
        assert_eq!(&rgb[1..2], &[50]);
        assert_eq!(&rgb[2..3], &[25]);
        assert_eq!(cmyk_to_rgb(&[0, 0, 0, 0]), vec![0, 0, 0]);
        assert_eq!(cmyk_to_rgb(&[255, 255, 255, 255]), vec![255, 255, 255]);
    }

    #[test]
    fn rgb_to_cmyk_is_the_exact_right_inverse_of_cmyk_to_rgb() {
        // K = 255 (no black) makes the product an identity, so this is exact for
        // every byte value, not a 1-LSB approximation.
        for r in 0..=255u8 {
            for g in [0u8, 1, 127, 128, 254, 255] {
                for b in [0u8, 1, 127, 128, 254, 255] {
                    let rgb = [r, g, b];
                    let cmyk = rgb_to_cmyk(&rgb);
                    assert_eq!(cmyk, [r, g, b, 255], "rgb {r},{g},{b}");
                    assert_eq!(cmyk_to_rgb(&cmyk), rgb, "round trip {r},{g},{b}");
                }
            }
        }
        // A multi-pixel planar layout: three pixels, plane-major.
        let rgb = [10, 20, 30, 40, 50, 60, 70, 80, 90];
        let cmyk = rgb_to_cmyk(&rgb);
        assert_eq!(cmyk[0..3], [10, 20, 30]);
        assert_eq!(cmyk[3..6], [40, 50, 60]);
        assert_eq!(cmyk[6..9], [70, 80, 90]);
        assert_eq!(cmyk[9..12], [255, 255, 255]);
        assert_eq!(cmyk_to_rgb(&cmyk), rgb);
    }

    #[test]
    fn lab_neutral_gray_and_white_within_two() {
        let gray = lab_to_rgb(&[200, 128, 128]);
        for value in gray {
            assert!((value as i32 - 194).abs() <= 2, "gray {value}");
        }
        let white = lab_to_rgb(&[255, 128, 128]);
        for value in white {
            assert!((value as i32 - 255).abs() <= 2, "white {value}");
        }
    }

    #[test]
    fn lab_non_neutral_matches_lcms2_exact() {
        // Reference values from lcms2's exact (unoptimized) LAB->sRGB transform,
        // which this formula implements. psd-tools/Pillow `.convert("RGB")`
        // applies the optimized LUT and is deliberately not the reference.
        for (lab, expected) in [
            ([225u8, 82, 114], [60u8, 246, 246]),
            ([200, 110, 100], [123, 205, 245]),
            ([150, 110, 110], [84, 152, 172]),
            ([120, 160, 140], [164, 89, 93]),
            ([90, 120, 160], [82, 86, 28]),
            ([160, 90, 120], [17, 170, 165]),
            ([110, 100, 160], [64, 114, 45]),
            ([200, 140, 160], [230, 185, 135]),
        ] {
            let rgb = lab_to_rgb(&lab);
            for c in 0..3 {
                assert_eq!(rgb[c], expected[c], "lab {lab:?} channel {c}");
            }
        }
    }

    #[test]
    fn rgb_to_lab_inverts_lab_to_rgb_on_the_read_grid() {
        // rgb_to_lab is the algebraic inverse of lab_to_rgb: reversing the exact
        // read-oracle values recovers them within the 1-LSB read tolerance.
        for (lab, rgb) in [
            ([225u8, 82, 114], [60u8, 246, 246]),
            ([200, 110, 100], [123, 205, 245]),
            ([150, 110, 110], [84, 152, 172]),
            ([120, 160, 140], [164, 89, 93]),
            ([90, 120, 160], [82, 86, 28]),
            ([160, 90, 120], [17, 170, 165]),
            ([110, 100, 160], [64, 114, 45]),
            ([200, 140, 160], [230, 185, 135]),
        ] {
            let back = rgb_to_lab(&rgb);
            for c in 0..3 {
                assert!(
                    (back[c] as i32 - lab[c] as i32).abs() <= 1,
                    "lab {lab:?} rgb {rgb:?} channel {c} -> {back:?}"
                );
            }
        }
        // Every 8-bit gray round-trips within 1 LSB. ponytail: 8-bit Lab
        // quantizes saturated colors coarsely, so rgb_to_lab is not a 1-LSB
        // right-inverse over the whole RGB cube (only in-gamut, well-conditioned
        // colors); the read side is approximate in the same way.
        for v in 0..=255u16 {
            let gray = [v as u8; 3];
            let back = lab_to_rgb(&rgb_to_lab(&gray));
            for c in 0..3 {
                assert!(
                    (back[c] as i32 - v as i32).abs() <= 1,
                    "gray {v} channel {c} -> {back:?}"
                );
            }
        }
    }
}
