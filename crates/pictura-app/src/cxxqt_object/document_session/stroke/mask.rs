//! The stroke on a layer mask's coverage.

use pictura_core::{Document, PsdRect};
use pictura_paint::Stroke;

/// A live brush stroke painting a layer mask's coverage: the mask exposed as a
/// grayscale document (`document`), its document-space rectangle (`rect`), and
/// the exact [`Stroke`] running on it.
pub(in crate::cxxqt_object) struct MaskStroke {
    pub(in crate::cxxqt_object) path: String,
    pub(in crate::cxxqt_object) rect: PsdRect,
    pub(in crate::cxxqt_object) document: Document,
    pub(in crate::cxxqt_object) stroke: Stroke,
}
