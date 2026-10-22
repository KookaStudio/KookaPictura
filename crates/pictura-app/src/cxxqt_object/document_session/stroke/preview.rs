//! The reduced-resolution stroke presented while a large brush is down.

use pictura_paint::{StrokeConfig, StrokeSample};

/// A large-brush stroke shown at a view-pyramid level instead of rasterized at
/// full resolution.
///
/// `snapshot` is the level's bytes as they were at stroke start — the pre-stroke
/// pixels the blend always starts from — so `coverage` can be applied to the
/// whole extent on every present without compounding. `samples` is the exact
/// input the commit replays through a real [`pictura_paint::Stroke`] on the
/// full-resolution document, which is what keeps the committed pixels
/// byte-identical to a stroke that never previewed.
pub(in crate::cxxqt_object) struct PreviewStroke {
    pub(in crate::cxxqt_object) level: u32,
    pub(in crate::cxxqt_object) scale: u32,
    pub(in crate::cxxqt_object) width: u32,
    pub(in crate::cxxqt_object) height: u32,
    pub(in crate::cxxqt_object) snapshot: Vec<u8>,
    pub(in crate::cxxqt_object) coverage: Vec<u8>,
    pub(in crate::cxxqt_object) extent: Option<pictura_core::PsdRect>,
    pub(in crate::cxxqt_object) samples: Vec<StrokeSample>,
    /// The stroke's configuration, so the preview rasterizes the same tip,
    /// flow and opacity the exact stroke will.
    pub(in crate::cxxqt_object) cfg: StrokeConfig,
    /// Set when the stroke paints a layer mask: the coverage then moves the
    /// composite between the mask's extremes instead of laying down colour.
    pub(in crate::cxxqt_object) mask: Option<MaskPreview>,
}

/// What a previewed mask stroke blends between, on the reduced-document grid
/// (which can be a pixel short of the preview level's): the mask's coverage at
/// stroke start, and the composite with the mask hiding and revealing
/// everywhere, as premultiplied sRGB RGBA like the pyramid levels.
pub(in crate::cxxqt_object) struct MaskPreview {
    pub(in crate::cxxqt_object) width: u32,
    pub(in crate::cxxqt_object) height: u32,
    pub(in crate::cxxqt_object) coverage: Vec<u8>,
    pub(in crate::cxxqt_object) hidden: Vec<u8>,
    pub(in crate::cxxqt_object) revealed: Vec<u8>,
}
