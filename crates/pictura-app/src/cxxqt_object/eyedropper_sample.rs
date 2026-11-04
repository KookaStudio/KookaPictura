//! The Eyedropper's size/scope-aware sampling bridge. A free function over a
//! [`PictureView`], folded into [`paint_tools`](super::paint_tools).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;

pub(crate) use imp::*;
