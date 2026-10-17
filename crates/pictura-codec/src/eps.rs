//! Generic EPS import (`interop/pdf-eps-import`): the bounding box comments,
//! the DOS binary EPS wrapper, and a render of the bounding box through
//! Ghostscript's `-dEPSCrop`.

use std::path::Path;

use crate::ghostscript::{png_args, read_png, Ghostscript, RasterError, ScratchDir};
use crate::pdf::import::{crop_raster, PdfRect, Rendered};

/// The DOS binary EPS header's magic number.
pub const DOS_EPS_MAGIC: [u8; 4] = [0xC5, 0xD0, 0xD3, 0xC6];

/// The PostScript section of an EPS: the whole file, or the section a DOS
/// binary header points at.
pub fn postscript_section(bytes: &[u8]) -> Option<&[u8]> {
    if !bytes.starts_with(&DOS_EPS_MAGIC) {
        return Some(bytes);
    }
    let word = |at: usize| -> Option<usize> {
        Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    let (start, len) = (word(4)?, word(8)?);
    bytes.get(start..start.checked_add(len)?)
}

/// The EPS bounding box in points: `%%HiResBoundingBox` when present, else
/// `%%BoundingBox`; an `(atend)` value is looked up in the trailer. `None`
/// without a usable box.
pub fn bounding_box(bytes: &[u8]) -> Option<PdfRect> {
    let ps = postscript_section(bytes)?;
    let text = String::from_utf8_lossy(ps);
    let find = |key: &str| -> Option<PdfRect> {
        text.lines()
            .filter_map(|line| line.strip_prefix(key))
            .filter_map(|rest| {
                let v: Vec<f64> = rest
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                match v[..] {
                    [x0, y0, x1, y1] if x1 > x0 && y1 > y0 => Some(PdfRect::new(x0, y0, x1, y1)),
                    _ => None,
                }
            })
            .next()
    };
    find("%%HiResBoundingBox:").or_else(|| find("%%BoundingBox:"))
}

/// Render the EPS at `path` so its bounding box fills `width` × `height`
/// pixels, onto transparency.
pub fn render_eps(
    gs: &Ghostscript,
    path: &Path,
    bounds: PdfRect,
    (width, height): (u32, u32),
    anti_alias: bool,
) -> Result<Rendered, RasterError> {
    if width == 0 || height == 0 {
        return Err(RasterError::Failed("empty image size".into()));
    }
    let rx = 72.0 * f64::from(width) / bounds.width();
    let ry = 72.0 * f64::from(height) / bounds.height();
    let scratch = ScratchDir::new()?;
    let out = scratch.path("eps.png");
    let mut args = vec!["-dEPSCrop".to_string()];
    args.extend(png_args(rx, ry, anti_alias, &out, path));
    let run = gs.run(path, None, &args)?;
    let full = read_png(&out)?;
    // `-dEPSCrop` rounds the page out to whole points past the upper-right
    // corner; the box's lower-left is the page's, so keep the bottom-left
    // `width` × `height`.
    Ok(Rendered {
        raster: crop_raster(
            &full,
            0,
            i64::from(full.height) - i64::from(height),
            width,
            height,
        ),
        warnings: run.warnings,
    })
}

#[cfg(test)]
mod tests;
