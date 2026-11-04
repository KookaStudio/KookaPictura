//! File ▸ New: a document in any of the dialog's modes, depths, Background
//! Contents, and resolutions, opened with one "New" history state. Folded into
//! [`image_size`](super::image_size). The base is
//! [`PictureView::new_document`]'s 8-bit RGB / Grayscale white or transparent
//! layer, made the locked Background unless transparent; the rest is the
//! Image ▸ Mode conversions and the ResolutionInfo writer applied before the
//! history starts. Ported from photorust's NewDocumentDialog.
//!
//! [`PictureView::new_document`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;

pub(crate) use imp::*;
