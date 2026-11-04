//! Path-addressed layer kind/lock queries. Free functions over a
//! [`PictureView`], folded into [`paths`](super::paths).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;

pub(crate) use imp::*;
