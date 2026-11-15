//! The picture a selection reads (#1274): a whole planar buffer, or a
//! composite fetched one [`TILE`]² tile at a time, so a large document never
//! needs a whole composite for the wand, Grow, Similar, Refine Edge or the
//! Magnetic Lasso. A tile holds the same bytes as that crop of the whole
//! composite, so both forms select identically.

use pictura_core::{PixelBuffer, TILE};
use std::sync::Arc;

use crate::tiled::{channel_tile, channel_uniform};

/// `fetch(tx, ty)` returns the straight interleaved RGBA of level-0 tile
/// `(tx, ty)`, clipped at the canvas edge.
pub type TileFetch<'a> = &'a (dyn Fn(usize, usize) -> Arc<[u8]> + Sync);

type Rect = (usize, usize, usize, usize);

/// The picture a selection reads.
#[derive(Clone, Copy)]
pub enum ImageSource<'a> {
    /// A planar buffer the size of the selection (1–2 planes read as gray).
    Buffer(&'a PixelBuffer),
    /// A `width`×`height` RGBA composite read through [`TileFetch`].
    Tiles {
        fetch: TileFetch<'a>,
        width: u32,
        height: u32,
    },
}

impl<'a> ImageSource<'a> {
    pub fn width(&self) -> u32 {
        match *self {
            Self::Buffer(img) => img.width,
            Self::Tiles { width, .. } => width,
        }
    }

    pub fn height(&self) -> u32 {
        match *self {
            Self::Buffer(img) => img.height,
            Self::Tiles { height, .. } => height,
        }
    }

    /// The colour planes compared: three for RGB, the one gray plane otherwise.
    pub(crate) fn colour_planes(&self) -> usize {
        match *self {
            Self::Buffer(img) if img.channels < 3 => 1,
            _ => 3,
        }
    }

    /// The width of tile column `tx`, clipped at the canvas edge.
    fn tile_width(&self, tx: usize) -> usize {
        (self.width() as usize - tx * TILE).min(TILE)
    }

    /// The colour planes over `rect`, which lies inside one grid tile, each
    /// row-major.
    pub(crate) fn tile_planes(&self, rect: Rect) -> Vec<Vec<u8>> {
        let (x0, y0, tw, th) = rect;
        match *self {
            Self::Buffer(img) => {
                let (w, h) = (img.width as usize, img.height as usize);
                (0..self.colour_planes())
                    .map(|k| channel_tile(&img.data, w, h, k, rect))
                    .collect()
            }
            Self::Tiles { fetch, .. } => {
                let (tx, ty) = (x0 / TILE, y0 / TILE);
                let tile = fetch(tx, ty);
                let stride = self.tile_width(tx);
                let (lx, ly) = (x0 - tx * TILE, y0 - ty * TILE);
                (0..3)
                    .map(|k| {
                        let mut plane = Vec::with_capacity(tw * th);
                        for dy in 0..th {
                            let row = (ly + dy) * stride + lx;
                            plane.extend((row..row + tw).map(|i| tile[i * 4 + k]));
                        }
                        plane
                    })
                    .collect()
            }
        }
    }

    /// The colour over `rect` (inside one grid tile) when every pixel there
    /// holds it. A buffer answers from its tile storage, so a dense tile of one
    /// colour reads as varied there; either answer selects the same pixels.
    pub(crate) fn uniform_colour(&self, rect: Rect) -> Option<[u8; 3]> {
        match *self {
            Self::Buffer(img) => {
                let h = img.height as usize;
                let values: Option<Vec<u8>> = (0..self.colour_planes())
                    .map(|k| channel_uniform(&img.data, h, k, rect))
                    .collect();
                values.map(|v| colour(&v))
            }
            Self::Tiles { .. } => {
                let planes = self.tile_planes(rect);
                let first: Vec<u8> = planes
                    .iter()
                    .map(|p| p.first().copied())
                    .collect::<Option<_>>()?;
                planes
                    .iter()
                    .zip(&first)
                    .all(|(plane, &v)| plane.iter().all(|&s| s == v))
                    .then(|| colour(&first))
            }
        }
    }
}

/// Three planes' colour, or one plane's gray as a colour.
pub(crate) fn colour(planes: &[u8]) -> [u8; 3] {
    match planes.len() {
        3 => [planes[0], planes[1], planes[2]],
        _ => [planes[0]; 3],
    }
}

/// A fetched tile and its `(tx, ty)`.
type FetchedTile = ((usize, usize), Arc<[u8]>);

/// Reads single pixels of an [`ImageSource`], keeping the last tile it fetched
/// so a scan through one tile fetches it once.
pub(crate) struct PixelReader<'a> {
    source: ImageSource<'a>,
    last: Option<FetchedTile>,
}

impl<'a> PixelReader<'a> {
    pub(crate) fn new(source: ImageSource<'a>) -> Self {
        Self { source, last: None }
    }

    /// The colour of pixel `(x, y)`, which must be on the canvas.
    pub(crate) fn rgb(&mut self, x: usize, y: usize) -> [u8; 3] {
        match self.source {
            ImageSource::Buffer(img) => crate::rgb_at(img, y * img.width as usize + x),
            ImageSource::Tiles { fetch, .. } => {
                let key = (x / TILE, y / TILE);
                if self.last.as_ref().is_none_or(|(at, _)| *at != key) {
                    self.last = Some((key, fetch(key.0, key.1)));
                }
                let tile = &self.last.as_ref().expect("a fetched tile").1;
                let i = ((y % TILE) * self.source.tile_width(key.0) + x % TILE) * 4;
                [tile[i], tile[i + 1], tile[i + 2]]
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A tile fetch over `img` (planar RGB or RGBA), for the identity tests.
    pub(crate) fn fetch_of(img: &PixelBuffer) -> impl Fn(usize, usize) -> Arc<[u8]> + Sync + '_ {
        move |tx, ty| {
            let (w, h) = (img.width as usize, img.height as usize);
            let (x0, y0) = (tx * TILE, ty * TILE);
            let (x1, y1) = ((x0 + TILE).min(w), (y0 + TILE).min(h));
            let n = w * h;
            let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = y * w + x;
                    let at = |c: usize| {
                        if c < img.channels as usize {
                            img.data.sample(c * n + i).unwrap_or(0)
                        } else {
                            255
                        }
                    };
                    out.extend([at(0), at(1), at(2), at(3)]);
                }
            }
            out.into()
        }
    }
}
