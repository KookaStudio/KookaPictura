//! HDR Conversion (32 -> 16/8) bridge: the four-method conversion, one
//! `HDR Conversion` history state, and the selection clear. Folded into
//! [`image_adjust`](super::image_adjust) so the seam has one fewer header.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;

pub(crate) use imp::*;
