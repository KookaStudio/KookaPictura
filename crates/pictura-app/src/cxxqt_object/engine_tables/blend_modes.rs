//! The blend-mode bridge: the shell builds every layer blend combo from the
//! engine's `BlendMode::LAYER_MODES` instead of a second hand-kept table.

mod imp;

pub(crate) use imp::*;
