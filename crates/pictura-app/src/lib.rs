//! M0 application shell: the Rust↔Qt bridge spike.
//!
//! The [`cxxqt_object`] module defines a `QObject` in Rust via cxx-qt. The C++
//! `main` (see `cpp/main.cpp`) links the static library, reads the composite
//! image from the Rust object, and shows it in a zoom/pan widget.

pub mod cxxqt_object;
mod decode_bridge;
pub mod gpu;
mod history;
pub(crate) mod layer_target;
