//! Blend Text Colors Using Gamma (Color Settings) and `Type > Replace All
//! Missing Fonts`: free functions over a [`PictureView`]. Qt decides which
//! fonts are missing; the engine re-sets the layers that use them.
//!
//! [`PictureView`]: crate::cxxqt_object::qobject::PictureView

use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

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

        /// Re-set every type layer whose font is one of `missing` (one name per line) in the default font, recording one "Replace All Missing Fonts" state. Returns how many layers changed; 0 records nothing.
        fn type_replace_fonts(view: Pin<&mut PictureView>, missing: &QString) -> i32;
    }
}

fn type_set_text_gamma(enabled: bool, gamma: f64) {
    pictura_render::set_text_gamma(enabled.then_some(gamma as f32));
}

fn type_text_gamma_refresh(view: Pin<&mut PictureView>) {
    if view.as_ref().rust().session.doc.is_some() {
        view.recomposite();
    }
}

fn type_replace_fonts(view: Pin<&mut PictureView>, missing: &QString) -> i32 {
    let missing = missing.to_string();
    let names: Vec<&str> = missing.lines().filter(|name| !name.is_empty()).collect();
    if names.is_empty() {
        return 0;
    }
    let mut replaced = 0;
    view.edit("Replace All Missing Fonts", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        replaced = pictura_render::replace_type_fonts(doc, &names);
        if replaced > 0 {
            Edit::Structural
        } else {
            Edit::None
        }
    });
    i32::try_from(replaced).unwrap_or(i32::MAX)
}
