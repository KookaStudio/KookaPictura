//! The blend-mode bridge: the shell builds every layer blend combo from the
//! engine's `BlendMode::LAYER_MODES` instead of a second hand-kept table.

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "pictura"]
    struct BlendModeRow {
        /// The 4-byte PSD key, e.g. `"mul "`.
        key: QString,
        name: QString,
        /// Whether the mode has a neutral colour (New Layer's "Fill with
        /// neutral color" option).
        has_neutral: bool,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The 27 layer modes in engine order; Pass Through is excluded.
        fn blend_mode_rows() -> Vec<BlendModeRow>;
    }
}
