//! A tiny bridge for the Qt decode edge's bulk copy, plus the two root-level
//! free functions that have no Qt object: the stdio MCP entry point and the SVG
//! serializer. Kept out of `cxxqt_object.rs` so the main bridge file stays
//! within its size budget. `decode_image.cpp` includes the generated header and
//! calls `copy_bytes` to hand a whole decoded frame to Rust in one operation.

mod imp;

use crate::cxxqt_object::qobject::PictureView;
use crate::cxxqt_object::svg_export::svg_from_view;
use crate::mcp_entry::mcp_run;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    extern "Rust" {
        /// Copy `bytes` into a new Rust `Vec` in one operation, so the C++
        /// decoder copies whole rows or frames instead of one byte per call.
        fn copy_bytes(bytes: &[u8]) -> Vec<u8>;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Serve MCP on stdio until stdin closes, and return the exit code.
        fn mcp_run(socket: &QString, token_path: &QString, args: &QStringList) -> i32;

        /// The active document serialized to SVG text, or empty without one.
        /// Never changes the document, its path, dirty flag, or history.
        fn svg_from_view(view: &PictureView) -> QString;
    }
}
