//! The per-format raster save options bridge (`interop/save-format-options`):
//! store the settings the per-format dialog accepted on the view, read them
//! back in `save`, and encode the composite to bytes for the JPEG dialog's size
//! estimate and preview. Folded into [`export`](super::export).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;

pub(crate) use imp::*;
