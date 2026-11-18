//! Layer comps: read the document's comp list and mutate it through one
//! specialized bridge. The panel owns selection and dialog defaults; the
//! bridge owns document truth.

mod imp;

use super::qobject::PictureView;
use super::variables::{
    variables_available, variables_command, variables_preview, variables_state,
};
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
        fn variables_available(view: &PictureView, group: &QString) -> bool;
        fn variables_state(view: &PictureView, draft: bool) -> QString;
        fn variables_command(
            view: Pin<&mut PictureView>,
            action: &QString,
            payload: &QString,
        ) -> QString;
        fn variables_preview(view: &PictureView, payload: &QString) -> Vec<u8>;
        fn layer_comp_count(view: &PictureView) -> i32;
        fn layer_comp_id(view: &PictureView, index: i32) -> i32;
        fn layer_comp_name(view: &PictureView, index: i32) -> QString;
        fn layer_comp_comment(view: &PictureView, index: i32) -> QString;
        fn layer_comp_record(view: &PictureView, index: i32) -> i32;
        fn layer_comp_warning(view: &PictureView, index: i32) -> bool;
        fn layer_comp_last_available(view: &PictureView) -> bool;
        fn layer_comp_create(
            view: Pin<&mut PictureView>,
            name: &QString,
            comment: &QString,
            record_bits: i32,
        ) -> i32;
        fn layer_comp_apply(view: Pin<&mut PictureView>, id: i32) -> bool;
        fn layer_comp_update(view: Pin<&mut PictureView>, id: i32) -> bool;
        fn layer_comp_delete(view: Pin<&mut PictureView>, id: i32) -> bool;
        fn layer_comp_rename(view: Pin<&mut PictureView>, id: i32, name: &QString) -> bool;
        fn layer_comp_set_options(
            view: Pin<&mut PictureView>,
            id: i32,
            name: &QString,
            comment: &QString,
            record_bits: i32,
        ) -> bool;
        fn layer_comp_duplicate(view: Pin<&mut PictureView>, id: i32) -> i32;
        fn layer_comp_clear_warning(view: Pin<&mut PictureView>, id: i32) -> bool;
        fn layer_comp_clear_all_warnings(view: Pin<&mut PictureView>) -> bool;
        fn layer_comp_restore_last(view: Pin<&mut PictureView>) -> bool;
        fn layer_comp_export_targets(view: &PictureView, prefix: &QString) -> QString;
        fn layer_comp_export_file(
            view: &PictureView,
            id: i32,
            format: &QString,
            include_icc: bool,
            path: &QString,
        ) -> bool;
        fn layer_comp_export_pdf(view: &PictureView, path: &QString) -> bool;
    }
}
