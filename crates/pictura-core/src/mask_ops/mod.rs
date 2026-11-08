//! Shared exact mask kernels: separable square-footprint morphology,
//! clamped-window median, and a scanline four-connected flood.
//!
//! Every kernel here produces the same samples as the naive scan it replaces.
//! The cost is independent of the radius (morphology and median) and the flood
//! keeps a span frontier instead of one queue entry per visited pixel.
//!
//! The flood and the neighbourhood kernels also run over a [`TiledMask`]
//! (#1214): [`flood_tiled`] pulls its source one tile at a time, and
//! [`map_bands`] runs a kernel one row of tiles at a time.

mod flood;
mod flood_tiled;
mod median;
mod morph;
mod windows;

#[cfg(test)]
pub(crate) use flood::flood_peak;
pub use flood::{flood, flood_from};
pub use flood_tiled::{
    flood_tiled, match_mask, FloodSource, Grid, MatchTiles, Seeds, TileBits, TileMatch,
};
pub use median::median;
pub use morph::{dilate, erode, Border};
pub use windows::{coverage_bounds, first_covered, last_covered, map_bands, tiled_view, zip_tiles};

/// A coverage mask (0–255 per pixel) stored as the tiled `Plane<u8>` of
/// #1213, laid on the document grid: `Empty` where nothing is covered,
/// `Uniform` where one value covers a whole tile, `Dense` elsewhere. A plane
/// that is still flat is accepted wherever a `TiledMask` is.
pub type TiledMask = crate::Plane<u8>;

/// An empty `w × h` tiled mask: every tile `Empty`, nothing allocated.
pub fn empty_mask(w: usize, h: usize) -> TiledMask {
    crate::Plane::filled(w, h, 1, 0u8)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tiled_tests;
