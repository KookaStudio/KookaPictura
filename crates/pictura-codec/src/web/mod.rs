//! Save for Web & Devices (`docs/10-workflow-io/web-export-and-slices.md`):
//! palette reduction and dithering for GIF, PNG-8, and WBMP, plus the GIF and
//! WBMP encoders Qt does not ship. JPEG and PNG-24 (and PNG-8's file, from the
//! indexed image here) are written through Qt by the app.
//!
//! Ported from photorust's `shell/src/dialogs/SaveForWebDialog.cpp` and
//! `GifWriter.cpp`, which left palette and dither to Qt and searched its LZW
//! table linearly; the reduction methods, dithering, interlacing, and the
//! hashed LZW dictionary are new here.

mod act;
mod gif;
pub mod html;
mod metadata;
mod palette;
mod quantize;

pub use act::{encode_act, parse_act, ColorTable};
pub use gif::encode_gif;
pub use html::{slice_html, SliceCell, SliceHtmlOptions, SliceLayout};
pub use metadata::{embed_web_metadata, sfw_metadata, MetadataLevel, WebMetadata};
pub use palette::{
    delete as palette_delete, edit as palette_edit, map_transparency as palette_map_transparency,
    pack as pack_indexed, sort as palette_sort, unpack as unpack_indexed, PaletteSort,
};
pub use quantize::{quantize, quantize_custom, quantize_locked, web_safe_palette};

/// The Color Reduction methods. Perceptual, Selective, and Adaptive are all
/// median cut, differing in how a box is chosen for splitting: Adaptive by
/// its widest channel range, Perceptual by its widest luma-weighted range,
/// Selective by that range times its pixel count (favouring broad areas of
/// colour). Adobe's algorithms are unpublished, so these are approximations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorReduction {
    Perceptual,
    Selective,
    Adaptive,
    /// The 216 web-safe colours.
    Restrictive,
    BlackAndWhite,
    Grayscale,
}

/// The dithering methods; each takes an amount (0–100 %).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dither {
    None,
    /// Floyd–Steinberg error diffusion.
    Diffusion,
    /// A 4 × 4 ordered (Bayer) pattern.
    Pattern,
    /// Deterministic per-pixel noise, not diffused.
    Noise,
}

/// What a palette is built from: the method, the table size (2–256), the
/// dither and its amount (0–100), and transparency. With `transparency` a
/// pixel under half opacity becomes the transparent entry and the rest are
/// laid over `matte`; without it every pixel is laid over `matte`.
/// `mapped_transparency` extends a custom table's clear entry to opaque pixels
/// of that colour, which is what the Color Table's Map to Transparency does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteOptions {
    pub reduction: ColorReduction,
    pub colors: u16,
    pub dither: Dither,
    pub amount: u8,
    pub transparency: bool,
    pub mapped_transparency: bool,
    pub matte: [u8; 3],
}

/// An indexed image: a palette (alpha 0 marks the transparent entry, always
/// the last) and one palette index per pixel, row-major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Indexed {
    pub width: u32,
    pub height: u32,
    pub palette: Vec<[u8; 4]>,
    pub indices: Vec<u8>,
}

impl Indexed {
    /// The palette entry used for transparency, if any.
    pub fn transparent_index(&self) -> Option<u8> {
        self.palette.iter().position(|c| c[3] == 0).map(|i| i as u8)
    }
}

/// Encode RGBA8888 as a type-0 WBMP: black and white, 1 bit per pixel (1 is
/// white), rows padded to a byte, after the transparent pixels are laid over
/// white and the image reduced with `dither`.
pub fn encode_wbmp(rgba: &[u8], width: u32, height: u32, dither: Dither, amount: u8) -> Vec<u8> {
    let options = PaletteOptions {
        reduction: ColorReduction::BlackAndWhite,
        colors: 2,
        dither,
        amount,
        transparency: false,
        mapped_transparency: false,
        matte: [255, 255, 255],
    };
    let indexed = quantize(rgba, width, height, options);
    let mut out = vec![0u8, 0u8];
    for value in [width, height] {
        push_uintvar(&mut out, value);
    }
    let row = width.div_ceil(8) as usize;
    for y in 0..height as usize {
        let mut bits = vec![0u8; row];
        for x in 0..width as usize {
            let entry = indexed.palette[indexed.indices[y * width as usize + x] as usize];
            if entry[0] > 127 {
                bits[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&bits);
    }
    out
}

/// WBMP's multi-byte integer: 7 bits per byte, high bit set on all but the last.
fn push_uintvar(out: &mut Vec<u8>, value: u32) {
    let mut groups = vec![(value & 0x7f) as u8];
    let mut rest = value >> 7;
    while rest > 0 {
        groups.push(0x80 | (rest & 0x7f) as u8);
        rest >>= 7;
    }
    out.extend(groups.iter().rev());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wbmp_packs_white_as_one_with_its_header() {
        // 10 x 2: the left half black, the right half white.
        let mut rgba = Vec::new();
        for _ in 0..2 {
            for x in 0..10 {
                let v = if x < 5 { 0 } else { 255 };
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let wbmp = encode_wbmp(&rgba, 10, 2, Dither::None, 0);
        assert_eq!(&wbmp[..4], &[0, 0, 10, 2]);
        assert_eq!(
            &wbmp[4..],
            &[0b0000_0111, 0b1100_0000, 0b0000_0111, 0b1100_0000]
        );
        let mut big = Vec::new();
        push_uintvar(&mut big, 300);
        assert_eq!(big, vec![0x82, 0x2c]);
    }
}
