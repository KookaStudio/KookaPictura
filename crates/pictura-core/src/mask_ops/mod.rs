//! Shared exact mask kernels: separable square-footprint morphology,
//! clamped-window median, and a scanline four-connected flood.
//!
//! Every kernel here produces the same samples as the naive scan it replaces.
//! The cost is independent of the radius (morphology and median) and the flood
//! keeps a span frontier instead of one queue entry per visited pixel.

mod flood;
mod median;
mod morph;

pub use flood::flood;
#[cfg(test)]
pub(crate) use flood::flood_peak;
pub use median::median;
pub use morph::{dilate, erode, Border};

#[cfg(test)]
mod tests;
