//! Image > Adjustments > HDR Toning: the Local Adaptation neighborhood operator.
//!
//! Folded into [`image_adjust`](super::image_adjust) so the seam has one fewer
//! header. The nine Local Adaptation controls preview on the canvas through the
//! Filter menu's preview machinery and apply as one "HDR Toning" state. The
//! kernel lives in `pictura_filters::hdr_toning`.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;

pub(crate) use imp::*;
