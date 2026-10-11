//! The Eyedropper's size/scope-aware sampling bridge. A free function over a
//! [`PictureView`] with its own bridge, so the `PictureView` declaration list
//! does not grow (matching [`align`](super::align)).

use super::helpers_composite::sample_argb_scoped as sample_scoped;
use super::qobject::PictureView;
use cxx_qt::CxxQtType;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The average `0xAARRGGBB` of a `size`×`size` box centred on `(x, y)`
        /// read from `scope` (0 Current Layer, 1 Current & Below, 2 All Layers,
        /// 3 All Layers No Adjustments, 4 Current & Below No Adjustments); 0
        /// without a document or when nothing is sampled.
        fn sample_argb_scoped(view: &PictureView, x: i32, y: i32, size: i32, scope: i32) -> u32;
    }
}

fn sample_argb_scoped(view: &PictureView, x: i32, y: i32, size: i32, scope: i32) -> u32 {
    let rust = view.rust();
    let Some(doc) = rust.doc.as_ref() else {
        return 0;
    };
    let path = rust.active_layer.as_deref().unwrap_or_default();
    sample_scoped(doc, path, x, y, size, scope)
}
