//! The command catalogue bridge: the C++ shell builds its menus from these
//! rows instead of maintaining a second hand-authored table.

mod imp;

pub(crate) use imp::*;
