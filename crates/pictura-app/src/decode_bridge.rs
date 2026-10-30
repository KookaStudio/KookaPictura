//! A tiny bridge for the Qt decode edge's bulk copy.
//!
//! Kept out of `cxxqt_object.rs` so the main bridge file stays within its
//! size budget. `decode_image.cpp` includes the generated header and calls
//! `copy_bytes` to hand a whole decoded frame to Rust in one operation.

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    extern "Rust" {
        /// Copy `bytes` into a new Rust `Vec` in one operation, so the C++
        /// decoder copies whole rows or frames instead of one byte per call.
        fn copy_bytes(bytes: &[u8]) -> Vec<u8>;
    }
}
