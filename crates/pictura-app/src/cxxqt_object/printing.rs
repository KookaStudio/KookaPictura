mod imp;
mod settings;

use super::qobject::PictureView;
use imp::*;
use settings::*;

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
    struct PrintRaster {
        width: u32,
        height: u32,
        components: u8,
        samples: Vec<u8>,
        profile: Vec<u8>,
        error: QString,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        fn print_document_settings(view: &PictureView) -> QString;
        fn print_metadata_pending(view: &PictureView) -> bool;
        fn print_document_settings_update(
            view: Pin<&mut PictureView>,
            old: &QString,
            updated: &QString,
            commit: bool,
        ) -> QString;
        fn print_document_description(view: &PictureView) -> QString;
        fn print_source_preview(view: &PictureView, max_side: u32) -> PrintRaster;
        fn print_output_profile_info(icc: &[u8]) -> QString;
        fn print_builtin_profile(device: i32) -> Vec<u8>;
        fn print_preview_pixels(
            rgba: &[u8],
            output: &[u8],
            proof: &[u8],
            settings: &QString,
        ) -> Vec<u8>;
        fn print_paper_color(icc: &[u8]) -> Vec<u8>;
        fn print_device_color(rgb: &[u8], output: &[u8], settings: &QString) -> Vec<u8>;
        fn print_document_raster(
            view: &PictureView,
            output: &[u8],
            proof: &[u8],
            settings: &QString,
        ) -> PrintRaster;
        fn print_document_preview(
            view: &PictureView,
            max_side: u32,
            output: &[u8],
            proof: &[u8],
            settings: &QString,
        ) -> PrintRaster;
        fn print_image_raster(
            rgba: &[u8],
            width: u32,
            height: u32,
            output: &[u8],
            proof: &[u8],
            settings: &QString,
        ) -> PrintRaster;
        fn print_pdf_bytes(
            raster: &PrintRaster,
            geometry: &[f64],
            background: &QString,
            marks: &QString,
        ) -> Vec<u8>;
    }
}
