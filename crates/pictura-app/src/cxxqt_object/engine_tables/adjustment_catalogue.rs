//! The adjustment catalogue bridge: the shell wires Image > Adjustments, New
//! Fill/Adjustment Layer, the Layers panel menu and the Adjustments panel from
//! `pictura_render::ADJUSTMENT_CATALOGUE` instead of its own kind tables.

mod imp;

pub(crate) use imp::*;
