//! The filter dialog's live preview.

use pictura_core::{Layer, PsdRect};

/// The pre-filter pixels backing a live filter preview.
pub(in crate::cxxqt_object) struct FilterPreview {
    /// Panel path of the layer being previewed.
    pub(in crate::cxxqt_object) layer_path: String,
    /// What the preview overwrote.
    pub(in crate::cxxqt_object) baseline: PreviewBaseline,
}

/// What a filter preview overwrote, so the next one starts from the original.
pub(in crate::cxxqt_object) enum PreviewBaseline {
    /// The whole layer, before a whole-layer preview (sharing its planes).
    Layer(Box<Layer>),
    /// The pixels of `rect` (inside the layer) in each layer-sized channel,
    /// before a section preview, which writes nothing outside it. A copy of
    /// the section alone, so the live planes are written in place and never
    /// copied whole.
    Section {
        layer_rect: PsdRect,
        rect: PsdRect,
        planes: Vec<(i16, Vec<u8>)>,
    },
}

/// A filter dialog's live preview: either on a layer's pixels or on a layer
/// mask's coverage.
///
/// The two are the same kind of session, so the slot holds one of them, never
/// both.
pub(in crate::cxxqt_object) enum FilterPreviewSession {
    /// A preview on a layer's pixels, with the baseline it overwrote.
    Layer(FilterPreview),
    /// A preview on a mask's coverage, with the coverage it overwrote.
    Mask {
        /// The panel path of the layer whose mask is previewed.
        path: String,
        /// The mask's pre-preview coverage.
        original: Vec<u8>,
    },
}
