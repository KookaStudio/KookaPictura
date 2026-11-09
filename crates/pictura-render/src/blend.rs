//! The W3C blend functions live in `pictura-core` so the filter crate can
//! share them without depending on the renderer.

pub(crate) use pictura_core::blend::blend;
