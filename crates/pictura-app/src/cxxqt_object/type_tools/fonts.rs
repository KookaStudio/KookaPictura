//! Blend Text Colors Using Gamma (Color Settings) and `Type > Replace All
//! Missing Fonts`: free functions over a [`PictureView`]. Qt decides which
//! fonts are missing; the engine re-sets the layers that use them.
//!
//! [`PictureView`]: crate::cxxqt_object::qobject::PictureView

mod imp;

use crate::cxxqt_object::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Turn Blend Text Colors Using Gamma on at `gamma` (clamped to 1.00–2.20) or off, for every document. Shown composites keep their pixels until `type_text_gamma_refresh`.
        fn type_set_text_gamma(enabled: bool, gamma: f64);

        /// Recomposite the view after the text gamma changed. No history.
        fn type_text_gamma_refresh(view: Pin<&mut PictureView>);

        /// Whether the document's type layers can be re-set: only in an 8-bit RGB document.
        fn type_layers_editable(view: &PictureView) -> bool;

        /// Re-set every type layer whose font is one of `missing` (one name per line) in the default font, recording one "Replace All Missing Fonts" state. Returns how many layers changed; 0 records nothing.
        fn type_replace_fonts(view: Pin<&mut PictureView>, missing: &QString) -> i32;

        /// Turn Missing Glyph Protection on or off for every document.
        fn type_set_missing_glyph_protection(on: bool);

        /// Whether Missing Glyph Protection is on.
        fn type_missing_glyph_protection() -> bool;

        /// Recomposite the view after the glyph-protection setting changed. No history.
        fn type_missing_glyph_protection_refresh(view: Pin<&mut PictureView>);
    }
}
