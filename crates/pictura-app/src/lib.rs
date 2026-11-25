//! M0 application shell: the Rust↔Qt bridge spike.
//!
//! The [`cxxqt_object`] module defines a `QObject` in Rust via cxx-qt. The C++
//! `main` (see `cpp/main.cpp`) links the static library, reads the composite
//! image from the Rust object, and shows it in a zoom/pan widget.

#[cfg(test)]
mod build_config;
/// Counts the unit-test binary's heap for the memory guards (#1210); the
/// shipped library keeps the system allocator.
#[cfg(test)]
#[global_allocator]
static HEAP: pictura_testkit::PeakAlloc = pictura_testkit::PeakAlloc::system();
mod app_runtime_bridge;
pub mod cxxqt_object;
mod decode_bridge;
mod diagnostic_logging;
pub mod gpu;
mod history;
mod history_log;
pub(crate) mod layer_target;
mod mcp_entry;
mod preferences_bridge;
mod prefs;
mod worker_threads;
