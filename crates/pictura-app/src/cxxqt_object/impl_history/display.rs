//! What the shell needs to know about the level-0 frame it holds a copy of.
//! A free function over a [`PictureView`], folded into
//! [`purge`](super::purge).
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;

pub(crate) use imp::*;
