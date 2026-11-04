//! The colour-model bridge: the picker and the Info panel read Lab from
//! `pictura_color::model`, and the Channels panel thumbnails read the
//! engine's plate split, so the shell keeps no colour formula of its own.

mod imp;

pub(crate) use imp::*;
