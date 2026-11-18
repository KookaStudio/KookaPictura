//! The composite as the streaming Save As writers read it: one band of rows
//! at a time from the document's (tiled) planar composite, with the same
//! samples [`Raster::from_document`](super::Raster::from_document) computes for
//! the whole image (#1320).

use pictura_core::{ColorMode, Document, Plane, Samples};

use super::Raster;
use crate::error::PsdError;

/// Rows per band the streaming writers read.
pub(super) const BAND_ROWS: usize = 256;

pub(crate) struct RasterRows<'a> {
    pub width: u32,
    pub height: u32,
    pub mode: ColorMode,
    data: &'a Plane<u8>,
    channels: usize,
    /// Whether any composite pixel is not fully opaque.
    pub alpha: bool,
    pub palette: Option<[u8; 768]>,
    /// The retained Indexed source's indices, borrowed from its flat store.
    pub indices: Option<&'a [u8]>,
}

impl<'a> RasterRows<'a> {
    pub(crate) fn from_document(doc: &'a Document) -> Result<Self, PsdError> {
        let (width, height) = (doc.width, doc.height);
        if width == 0 || height == 0 {
            return Err(PsdError::Invalid("zero image size".into()));
        }
        let n = width as usize * height as usize;
        let channels = doc.composite.channels as usize;
        if doc.composite.width != width || doc.composite.height != height {
            return Err(PsdError::Invalid("composite size mismatch".into()));
        }
        if channels == 0 || doc.composite.data.len() < n * channels {
            return Err(PsdError::Invalid("short composite buffer".into()));
        }
        if channels > 4 {
            return Err(PsdError::Invalid("unsupported composite channels".into()));
        }
        let mode = doc.source.mode().unwrap_or(doc.mode);
        let (palette, indices) = if mode == ColorMode::Indexed {
            let indices = doc.source.planes().as_ref().and_then(|s| match &s.samples {
                Samples::U8(v) if v.len() == n => Some(v.as_slice()),
                _ => None,
            });
            match (doc.source.palette(), indices) {
                (Some(palette), Some(indices)) => (Some(palette), Some(indices)),
                _ => (None, None),
            }
        } else {
            (None, None)
        };
        let mut rows = Self {
            width,
            height,
            mode,
            data: &doc.composite.data,
            channels,
            alpha: false,
            palette,
            indices,
        };
        rows.alpha = channels >= 4 && rows.any_translucent();
        Ok(rows)
    }

    fn any_translucent(&self) -> bool {
        let (w, h) = (self.width as usize, self.height as usize);
        if self.data.uniform_over(0..w, 3 * h..4 * h) == Some(255) {
            return false;
        }
        let mut alpha = Vec::new();
        for y0 in (0..h).step_by(BAND_ROWS) {
            let rows = BAND_ROWS.min(h - y0);
            alpha.resize(rows * w, 0);
            self.data
                .read_rect_into(0..w, 3 * h + y0..3 * h + y0 + rows, w, &mut alpha);
            if alpha.iter().any(|&a| a != 255) {
                return true;
            }
        }
        false
    }

    /// Rows `y0 .. y0 + rows` as interleaved RGBA, as the whole-image raster
    /// expands each composite layout: gray to RGB, a missing alpha to 255.
    pub(super) fn rgba(&self, y0: usize, rows: usize, out: &mut Vec<u8>) {
        let (w, h) = (self.width as usize, self.height as usize);
        let n = w * rows;
        out.clear();
        out.resize(n * 4, 255);
        let mut plane = vec![0u8; n];
        let read = |c: usize, plane: &mut Vec<u8>| {
            self.data
                .read_rect_into(0..w, c * h + y0..c * h + y0 + rows, w, plane);
        };
        let (color, alpha) = match self.channels {
            1 => (&[0usize, 0, 0][..], None),
            2 => (&[0, 0, 0][..], Some(1)),
            3 => (&[0, 1, 2][..], None),
            _ => (&[0, 1, 2][..], Some(3)),
        };
        for (slot, &c) in color.iter().enumerate() {
            if slot == 0 || c != color[slot - 1] {
                read(c, &mut plane);
            }
            for (px, &v) in out.as_chunks_mut::<4>().0.iter_mut().zip(&plane) {
                px[slot] = v;
            }
        }
        if let Some(c) = alpha {
            read(c, &mut plane);
            for (px, &v) in out.as_chunks_mut::<4>().0.iter_mut().zip(&plane) {
                px[3] = v;
            }
        }
    }

    /// The Rec. 601 luma of each RGBA pixel, as the whole-image raster's gray
    /// plane.
    pub(super) fn gray_of(rgba: &[u8], out: &mut Vec<u8>) {
        out.clear();
        out.extend(
            rgba.as_chunks::<4>()
                .0
                .iter()
                .map(|p| pictura_core::rec601_luma(p[0], p[1], p[2])),
        );
    }
}

/// The samples a Save As writer reads, a band of rows at a time: the
/// whole-image [`Raster`] or the streamed [`RasterRows`] (#1320).
pub(super) trait Pixels {
    fn size(&self) -> (u32, u32);
    fn mode(&self) -> ColorMode;
    fn alpha(&self) -> bool;
    fn palette(&self) -> Option<[u8; 768]>;
    fn has_indices(&self) -> bool;
    /// Rows `y0 .. y0 + rows` as interleaved RGBA.
    fn rgba(&self, y0: usize, rows: usize, out: &mut Vec<u8>);
    /// Rows `y0 .. y0 + rows` of the `kind` planes, one buffer per plane;
    /// `rgba` holds the same rows.
    fn planes(&self, kind: Planes, y0: usize, rows: usize, rgba: &[u8], out: &mut [Vec<u8>]);
}

/// Which planes a writer stores for the document's mode.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Planes {
    Gray,
    Rgb,
    Cmyk,
    Lab,
    Indices,
}

impl Pixels for Raster {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn mode(&self) -> ColorMode {
        self.mode
    }
    fn alpha(&self) -> bool {
        self.alpha
    }
    fn palette(&self) -> Option<[u8; 768]> {
        self.palette
    }
    fn has_indices(&self) -> bool {
        self.indices.is_some()
    }
    fn rgba(&self, y0: usize, rows: usize, out: &mut Vec<u8>) {
        let w = self.width as usize;
        out.clear();
        out.extend_from_slice(&self.rgba[y0 * w * 4..(y0 + rows) * w * 4]);
    }
    fn planes(&self, kind: Planes, y0: usize, rows: usize, _: &[u8], out: &mut [Vec<u8>]) {
        let n = self.width as usize * self.height as usize;
        let span = y0 * self.width as usize..(y0 + rows) * self.width as usize;
        let (source, count): (&[u8], usize) = match kind {
            Planes::Gray => (&self.gray, 1),
            Planes::Rgb => (&self.rgb, 3),
            Planes::Cmyk => (&self.cmyk, 4),
            Planes::Lab => (&self.lab, 3),
            Planes::Indices => (self.indices.as_deref().unwrap_or_default(), 1),
        };
        for (c, plane) in out.iter_mut().take(count).enumerate() {
            plane.clear();
            plane.extend_from_slice(&source[c * n + span.start..c * n + span.end]);
        }
    }
}

impl Pixels for RasterRows<'_> {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn mode(&self) -> ColorMode {
        self.mode
    }
    fn alpha(&self) -> bool {
        self.alpha
    }
    fn palette(&self) -> Option<[u8; 768]> {
        self.palette
    }
    fn has_indices(&self) -> bool {
        self.indices.is_some()
    }
    fn rgba(&self, y0: usize, rows: usize, out: &mut Vec<u8>) {
        RasterRows::rgba(self, y0, rows, out);
    }
    fn planes(&self, kind: Planes, y0: usize, rows: usize, rgba: &[u8], out: &mut [Vec<u8>]) {
        match kind {
            Planes::Gray => RasterRows::gray_of(rgba, &mut out[0]),
            Planes::Indices => {
                let w = self.width as usize;
                out[0].clear();
                out[0]
                    .extend_from_slice(&self.indices.unwrap_or_default()[y0 * w..(y0 + rows) * w]);
            }
            // CMYK and Lab documents take the whole-image path.
            Planes::Rgb | Planes::Cmyk | Planes::Lab => {
                for (c, plane) in out.iter_mut().take(3).enumerate() {
                    plane.clear();
                    plane.extend(rgba.as_chunks::<4>().0.iter().map(|p| p[c]));
                }
            }
        }
    }
}
