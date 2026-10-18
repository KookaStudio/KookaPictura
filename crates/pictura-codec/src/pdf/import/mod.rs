//! Generic PDF import (`interop/pdf-eps-import`): page boxes, Bounding Box,
//! and page renders through Ghostscript (design D2, D3), and Images mode over
//! a minimal object reader (D4).

mod images;
pub(crate) mod objects;
#[cfg(test)]
mod tests;

use std::path::Path;

pub use images::PdfImage;

use crate::ghostscript::{png_args, read_png, Ghostscript, Raster, RasterError, ScratchDir};

/// A rectangle in PDF points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl PdfRect {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> PdfRect {
        PdfRect {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }

    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    fn intersect(&self, other: &PdfRect) -> PdfRect {
        let r = PdfRect {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        if r.x1 <= r.x0 || r.y1 <= r.y0 {
            *other
        } else {
            r
        }
    }
}

/// The Import PDF dialog's Crop To choices, in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropTo {
    BoundingBox,
    MediaBox,
    CropBox,
    BleedBox,
    TrimBox,
    ArtBox,
}

impl CropTo {
    pub fn from_index(index: i32) -> CropTo {
        match index {
            1 => CropTo::MediaBox,
            2 => CropTo::CropBox,
            3 => CropTo::BleedBox,
            4 => CropTo::TrimBox,
            5 => CropTo::ArtBox,
            _ => CropTo::BoundingBox,
        }
    }
}

/// One page's boxes in PDF user space, with the PDF defaults applied, and its
/// rotation. `bounding` is the painted-marks box in view space, once queried.
#[derive(Debug, Clone, PartialEq)]
pub struct PageInfo {
    pub media: PdfRect,
    pub crop: PdfRect,
    pub bleed: PdfRect,
    pub trim: PdfRect,
    pub art: PdfRect,
    pub rotate: u16,
    pub bounding: Option<PdfRect>,
}

impl PageInfo {
    /// The rendered page's size in points: the MediaBox, rotated.
    pub fn view_size(&self) -> (f64, f64) {
        let (w, h) = (self.media.width(), self.media.height());
        if self.rotate % 180 == 90 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// The `kind` box in view space: points from the rendered page's
    /// lower-left corner, after `/Rotate`. Bounding Box without a query (or
    /// on a blank page) is the Crop Box.
    pub fn view_box(&self, kind: CropTo) -> PdfRect {
        let user = match kind {
            CropTo::BoundingBox => {
                return self.bounding.unwrap_or_else(|| self.to_view(&self.crop))
            }
            CropTo::MediaBox => self.media,
            CropTo::CropBox => self.crop,
            CropTo::BleedBox => self.bleed,
            CropTo::TrimBox => self.trim,
            CropTo::ArtBox => self.art,
        };
        self.to_view(&user)
    }

    fn to_view(&self, r: &PdfRect) -> PdfRect {
        let (w, h) = (self.media.width(), self.media.height());
        let point = |x: f64, y: f64| {
            let (x, y) = (x - self.media.x0, y - self.media.y0);
            match self.rotate {
                90 => (y, w - x),
                180 => (w - x, h - y),
                270 => (h - y, x),
                _ => (x, y),
            }
        };
        let (ax, ay) = point(r.x0, r.y0);
        let (bx, by) = point(r.x1, r.y1);
        PdfRect::new(ax, ay, bx, by)
    }
}

/// A render and the recoverable problems Ghostscript reported making it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub raster: Raster,
    pub warnings: Vec<String>,
}

const PAGE_INFO: &str = "/F InputFile (r) file def F runpdfbegin \
(PAGES ) print pdfpagecount == flush \
1 1 pdfpagecount { dup pdfgetpage (PAGE ) print exch ==only \
[/MediaBox /CropBox /BleedBox /TrimBox /ArtBox /Rotate] \
{ 2 copy known { ( ) print dup ==only ( ) print 2 copy get ==only } if pop } forall \
pop () = flush } for runpdfend";

/// Every page's boxes and rotation. Fails with `PasswordRequired` /
/// `WrongPassword` for an encrypted PDF without the right `password`.
pub fn page_info(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
) -> Result<Vec<PageInfo>, RasterError> {
    let args = vec![
        "-dNODISPLAY".to_string(),
        format!("-sInputFile={}", path.display()),
        "-c".to_string(),
        PAGE_INFO.to_string(),
    ];
    let run = gs.run(path, password, &args)?;
    let pages = parse_page_info(&run.output);
    if pages.is_empty() {
        return Err(RasterError::Failed("no pages".into()));
    }
    Ok(pages)
}

/// Parse the `PAGE <n> /MediaBox [..] …` lines the page-info program prints.
pub(crate) fn parse_page_info(text: &str) -> Vec<PageInfo> {
    let mut pages = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("PAGE ") else {
            continue;
        };
        let mut boxes: [Option<PdfRect>; 5] = [None; 5];
        let mut rotate = 0i64;
        let mut tokens = rest.split_whitespace().skip(1).peekable();
        while let Some(key) = tokens.next() {
            let slot = match key {
                "/MediaBox" => Some(0),
                "/CropBox" => Some(1),
                "/BleedBox" => Some(2),
                "/TrimBox" => Some(3),
                "/ArtBox" => Some(4),
                "/Rotate" => {
                    rotate = tokens
                        .next()
                        .and_then(|t| t.parse::<f64>().ok())
                        .map_or(0, |r| r as i64);
                    None
                }
                _ => None,
            };
            let Some(slot) = slot else { continue };
            let mut values = Vec::new();
            for token in tokens.by_ref() {
                let done = token.ends_with(']');
                let number = token.trim_matches(|c| c == '[' || c == ']');
                if let Ok(v) = number.parse::<f64>() {
                    values.push(v);
                }
                if done {
                    break;
                }
            }
            if let [x0, y0, x1, y1] = values[..] {
                boxes[slot] = Some(PdfRect::new(x0, y0, x1, y1));
            }
        }
        let media = boxes[0].unwrap_or(PdfRect::new(0.0, 0.0, 612.0, 792.0));
        let crop = boxes[1].map_or(media, |c| c.intersect(&media));
        let inner = |slot: usize| boxes[slot].map_or(crop, |b| b.intersect(&media));
        pages.push(PageInfo {
            media,
            crop,
            bleed: inner(2),
            trim: inner(3),
            art: inner(4),
            rotate: (((rotate % 360) + 360) % 360 / 90 * 90) as u16,
            bounding: None,
        });
    }
    pages
}

const BBOX_DPI: u32 = 144;

/// Fill each page's `bounding` from Ghostscript's `bbox` device. A blank page
/// keeps `None`.
pub fn query_bounding_boxes(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
    pages: &mut [PageInfo],
) -> Result<(), RasterError> {
    // The bbox device defaults to ~4000 dpi, which rasterizes every embedded
    // photo at that size (seconds per page); 144 dpi keeps the box within a
    // fraction of a point at a small part of the cost.
    let args = vec![
        "-sDEVICE=bbox".to_string(),
        format!("-r{BBOX_DPI}"),
        "-f".to_string(),
        path.display().to_string(),
    ];
    let run = gs.run(path, password, &args)?;
    let boxes = run
        .output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("%%HiResBoundingBox:"))
        .map(|rest| {
            let v: Vec<f64> = rest
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            match v[..] {
                [x0, y0, x1, y1] if x1 > x0 && y1 > y0 => Some(PdfRect::new(x0, y0, x1, y1)),
                _ => None,
            }
        });
    for (page, bounding) in pages.iter_mut().zip(boxes) {
        page.bounding = bounding;
    }
    Ok(())
}

/// Render page `number` (1-based) so its `crop` box fills `width` × `height`
/// pixels, onto transparency.
#[allow(clippy::too_many_arguments)]
pub fn render_page(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
    number: u32,
    page: &PageInfo,
    crop: CropTo,
    size: (u32, u32),
    anti_alias: bool,
) -> Result<Rendered, RasterError> {
    let (_, view_height) = page.view_size();
    let area = page.view_box(crop);
    render_area(
        gs,
        path,
        password,
        number,
        area,
        view_height,
        size,
        anti_alias,
    )
}

/// Render the view-space `area` of page `number` (whose rendered height is
/// `view_height` points) into `width` × `height` pixels, onto transparency.
#[allow(clippy::too_many_arguments)]
pub fn render_area(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
    number: u32,
    area: PdfRect,
    view_height: f64,
    (width, height): (u32, u32),
    anti_alias: bool,
) -> Result<Rendered, RasterError> {
    if width == 0 || height == 0 || area.width() <= 0.0 || area.height() <= 0.0 {
        return Err(RasterError::Failed("empty page area".into()));
    }
    let rx = 72.0 * f64::from(width) / area.width();
    let ry = 72.0 * f64::from(height) / area.height();
    let scratch = ScratchDir::new()?;
    let out = scratch.path("page.png");
    let mut args = vec![
        format!("-dFirstPage={number}"),
        format!("-dLastPage={number}"),
    ];
    args.extend(png_args(rx, ry, anti_alias, &out, path));
    let run = gs.run(path, password, &args)?;
    let full = read_png(&out)?;
    let left = (area.x0 * rx / 72.0).round() as i64;
    let top = ((view_height - area.y1) * ry / 72.0).round() as i64;
    Ok(Rendered {
        raster: crop_raster(&full, left, top, width, height),
        warnings: run.warnings,
    })
}

/// Render every page so the longest page side is `longest` pixels; smaller
/// pages keep their relative size.
pub fn render_thumbnails(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
    pages: &[PageInfo],
    longest: u32,
) -> Result<Vec<Raster>, RasterError> {
    let side = pages
        .iter()
        .map(|p| {
            let (w, h) = p.view_size();
            w.max(h)
        })
        .fold(1.0f64, f64::max);
    let res = 72.0 * f64::from(longest) / side;
    let scratch = ScratchDir::new()?;
    let pattern = scratch.path("p%d.png");
    gs.run(path, password, &png_args(res, res, true, &pattern, path))?;
    (1..=pages.len())
        .map(|i| read_png(&scratch.path(&format!("p{i}.png"))))
        .collect()
}

/// `w` × `h` pixels of `src` starting at (`left`, `top`); outside is
/// transparent.
pub(crate) fn crop_raster(src: &Raster, left: i64, top: i64, w: u32, h: u32) -> Raster {
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for y in 0..h as i64 {
        let sy = top + y;
        if sy < 0 || sy >= i64::from(src.height) {
            continue;
        }
        for x in 0..w as i64 {
            let sx = left + x;
            if sx < 0 || sx >= i64::from(src.width) {
                continue;
            }
            let from = (sy as usize * src.width as usize + sx as usize) * 4;
            let to = (y as usize * w as usize + x as usize) * 4;
            rgba[to..to + 4].copy_from_slice(&src.rgba[from..from + 4]);
        }
    }
    Raster {
        width: w,
        height: h,
        rgba,
    }
}

/// The image XObjects of one PDF (Images mode).
pub struct PdfImages {
    objects: objects::PdfObjects,
    pub images: Vec<PdfImage>,
}

impl PdfImages {
    /// `None` when `pdf` is unreadable or encrypted (Images mode is then off).
    pub fn read(pdf: &[u8]) -> Option<PdfImages> {
        let objects = objects::PdfObjects::parse(pdf)?;
        let images = images::list_images(&objects)?;
        Some(PdfImages { objects, images })
    }

    /// Render image `index` at its own pixel size; `longest` scales it down
    /// for a thumbnail.
    pub fn render(
        &self,
        gs: &Ghostscript,
        index: usize,
        longest: Option<u32>,
    ) -> Result<Rendered, RasterError> {
        let image = self
            .images
            .get(index)
            .ok_or_else(|| RasterError::Failed("no such image".into()))?;
        let wrapped = images::wrap_image(&self.objects, image.object)
            .ok_or_else(|| RasterError::Failed("unreadable image".into()))?;
        let scratch = ScratchDir::new()?;
        let input = scratch.path("image.pdf");
        std::fs::write(&input, wrapped).map_err(|e| RasterError::Failed(e.to_string()))?;
        let out = scratch.path("image.png");
        let side = image.width.max(image.height);
        let res = longest.map_or(72.0, |l| 72.0 * f64::from(l.min(side)) / f64::from(side));
        let run = gs.run(&input, None, &png_args(res, res, false, &out, &input))?;
        let raster = read_png(&out)?;
        Ok(Rendered {
            raster,
            warnings: run.warnings,
        })
    }
}
