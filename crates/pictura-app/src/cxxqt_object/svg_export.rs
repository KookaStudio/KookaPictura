//! `File ▸ Export As…` with the SVG format: build the document's SVG text. Free
//! functions over a [`PictureView`], folded into
//! [`decode_bridge`](crate::decode_bridge). SVG is text, so it does not go
//! through the Qt `QImageWriter` edge; the dialog writes the returned string.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;

pub(crate) use imp::*;
