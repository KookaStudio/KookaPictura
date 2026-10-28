//! The depth and mode gates of the filters CS6 restricts: Adaptive Wide
//! Angle and Vanishing Point are missing from its 16- and 32-bpc filter
//! lists, and Lens Correction takes 8- and 16-bit RGB or Grayscale. Every
//! commit path refuses other documents.

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{BitDepth, ColorMode, Document};

use super::super::qobject::PictureView;

/// Whether the mode the user sees is RGB or Grayscale: a CMYK or Lab
/// conversion keeps RGB working planes and records its mode separately.
fn rgb_or_gray(doc: &Document) -> bool {
    matches!(
        pictura_render::document_color_mode(doc),
        ColorMode::Rgb | ColorMode::Grayscale
    )
}

/// 8-bit RGB or Grayscale.
pub(in crate::cxxqt_object) fn document_supported(doc: &Document) -> bool {
    doc.source_depth.unwrap_or(doc.depth) == BitDepth::Eight && rgb_or_gray(doc)
}

/// 8- or 16-bit RGB or Grayscale.
pub(in crate::cxxqt_object) fn lens_supported(doc: &Document) -> bool {
    matches!(
        doc.source_depth.unwrap_or(doc.depth),
        BitDepth::Eight | BitDepth::Sixteen
    ) && rgb_or_gray(doc)
}

pub(super) fn filter_document_takes(view: &PictureView, kind: &QString) -> bool {
    let kind = kind.to_string();
    depth_refusal(&kind, view.rust().session.doc.as_ref()).is_none()
        && super::smart_apply::smart_refusal(&kind, view.rust()).is_none()
}

/// Why `kind` may not commit on `doc`, whichever path (Last Filter
/// included) asks for it.
pub(in crate::cxxqt_object) fn depth_refusal(kind: &str, doc: Option<&Document>) -> Option<String> {
    let (name, takes, what): (_, fn(&Document) -> bool, _) = match kind {
        "adaptive-wide-angle" => ("Adaptive Wide Angle", document_supported, "an 8-bit"),
        "vanishing-point" => ("Vanishing Point", document_supported, "an 8-bit"),
        "lens-correction" => ("Lens Correction", lens_supported, "an 8- or 16-bit"),
        _ => return None,
    };
    (!doc.is_some_and(takes)).then(|| format!("{name} needs {what} RGB or Grayscale document"))
}
