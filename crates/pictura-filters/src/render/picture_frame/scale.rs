//! How big a frame's parts are, worked out from the canvas and the sliders.
//!
//! Ported from perfecto25/photorust (`core/src/filters/frame.rs`), GPL-3.0.

use super::{FrameOptions, FLOWER_SIZE, LEAF_SIZE, MARGIN, SIZE, THICKNESS};

pub(super) struct Scale {
    /// How far in from the canvas edge the frame runs.
    pub(super) inset: f32,
    /// A motif's reach: a pinwheel's radius, a spray's spread.
    pub(super) unit: f32,
    /// A flower's radius.
    pub(super) flower: f32,
    /// A leaf's length.
    pub(super) leaf: f32,
    /// The width of a vine.
    pub(super) vine: f32,
    /// Thickness as a multiple of CS6's default.
    pub(super) thickness: f32,
}

/// Margin, per step, as a share of the canvas's shorter side; and the base it
/// starts from. Fitted to CS6's own preview, where Margin 14 puts the
/// pinwheels' centres 7% of the way in.
pub(super) const MARGIN_BASE: f32 = 0.015;
pub(super) const MARGIN_PER_STEP: f32 = 0.004;

/// A motif's reach, a flower's radius and a leaf's length, each at 100 on its
/// slider, as a share of the canvas's shorter side. At CS6's defaults of 20 a
/// pinwheel spans about a seventh of the side, and its flowers are dots a
/// fortieth across.
pub(super) const UNIT_AT_FULL: f32 = 0.36;
pub(super) const FLOWER_AT_FULL: f32 = 0.065;
pub(super) const LEAF_AT_FULL: f32 = 0.1;

/// A vine's width as a share of a motif's reach, at CS6's default Thickness.
pub(super) const VINE_SHARE: f32 = 0.045;
pub(super) const THICKNESS_DEFAULT: f32 = 29.0;

pub(super) fn scale_for(options: &FrameOptions, width: u32, height: u32) -> Scale {
    let side = width.min(height) as f32;
    let size = options.size.clamp(*SIZE.start(), *SIZE.end()) as f32 / 100.0;
    let unit = side * size * UNIT_AT_FULL;
    let thickness = options
        .thickness
        .clamp(*THICKNESS.start(), *THICKNESS.end()) as f32
        / THICKNESS_DEFAULT;
    Scale {
        inset: side * (MARGIN_BASE + MARGIN_PER_STEP * options.margin.min(*MARGIN.end()) as f32),
        unit,
        flower: side
            * options
                .flower_size
                .clamp(*FLOWER_SIZE.start(), *FLOWER_SIZE.end()) as f32
            / 100.0
            * FLOWER_AT_FULL,
        leaf: side
            * options
                .leaf_size
                .clamp(*LEAF_SIZE.start(), *LEAF_SIZE.end()) as f32
            / 100.0
            * LEAF_AT_FULL,
        vine: (unit * VINE_SHARE * thickness).max(0.8),
        thickness,
    }
}
