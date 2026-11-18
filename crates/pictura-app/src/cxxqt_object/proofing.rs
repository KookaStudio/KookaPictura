//! Proof setup and Proof Colors view state (`color/proof-setup`).
//!
//! The setup is application view state: the shell keeps it in the session and
//! pushes it onto every open view, and the canvas reads it while presenting.
//! The bridge also exposes print settings and colour-managed output.

mod imp;
mod print;
mod print_settings;

use super::qobject::PictureView;
pub(super) use imp::*;
use print::*;
use print_settings::*;

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
        /// Set the active proof setup on the view. `device` is 0 Working CMYK,
        /// 1 Working Gray, 2 sRGB, 3 Adobe RGB, 4 ProPhoto RGB; `intent` follows
        /// the Color Settings codes (0 Perceptual, 1 Relative, 2 Saturation,
        /// 3 Absolute).
        fn set_proof_setup(
            view: Pin<&mut PictureView>,
            device: i32,
            intent: i32,
            bpc: bool,
            simulate_paper: bool,
            simulate_black: bool,
        );

        /// Turn the proof display on or off; returns whether it changed.
        fn set_proof_colors(view: Pin<&mut PictureView>, on: bool) -> bool;

        /// The active proof setup's display name, e.g. `Working CMYK`.
        fn proof_setup_name(view: &PictureView) -> QString;
        fn proof_colors_active(view: &PictureView) -> bool;
        fn proof_device(view: &PictureView) -> i32;
        fn proof_intent(view: &PictureView) -> i32;
        fn proof_bpc(view: &PictureView) -> bool;
        fn proof_simulate_paper(view: &PictureView) -> bool;
        fn proof_simulate_black(view: &PictureView) -> bool;

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
