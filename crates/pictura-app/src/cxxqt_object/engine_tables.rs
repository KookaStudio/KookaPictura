//! Free-function bridges that serve the engine's tables and colour conversions
//! to the shell, so it keeps no copy of its own: blend modes, colour models,
//! the adjustment catalogue and the tool catalogue.

mod adjustment_catalogue;
mod blend_modes;
mod color_models;
mod tool_catalogue;
