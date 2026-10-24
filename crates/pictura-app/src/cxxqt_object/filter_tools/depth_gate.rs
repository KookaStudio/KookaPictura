//! The 8-bit gate Adaptive Wide Angle and Vanishing Point share: CS6's 16-
//! and 32-bpc filter lists leave both out, so every commit path refuses
//! other documents.

use cxx_qt::CxxQtType;
use pictura_core::{BitDepth, ColorMode, Document};

use super::super::qobject::PictureView;

/// 8-bit RGB or Grayscale.
pub(in crate::cxxqt_object) fn document_supported(doc: &Document) -> bool {
    doc.source_depth.unwrap_or(doc.depth) == BitDepth::Eight
        && matches!(doc.mode, ColorMode::Rgb | ColorMode::Grayscale)
}

pub(super) fn filter_eight_bit_document(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(document_supported)
}

/// Why `kind` may not commit on `doc`, whichever path (Last Filter
/// included) asks for it.
pub(in crate::cxxqt_object) fn eight_bit_refusal(
    kind: &str,
    doc: Option<&Document>,
) -> Option<String> {
    let name = match kind {
        "adaptive-wide-angle" => "Adaptive Wide Angle",
        "vanishing-point" => "Vanishing Point",
        _ => return None,
    };
    (!doc.is_some_and(document_supported))
        .then(|| format!("{name} needs an 8-bit RGB or Grayscale document"))
}
