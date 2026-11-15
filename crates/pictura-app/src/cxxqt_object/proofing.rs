//! Proof setup and Proof Colors view state (`color/proof-setup`).
//!
//! The setup is application view state: the shell keeps it in the session and
//! pushes it onto every open view, and the canvas reads it while presenting.
//! Free functions over a [`PictureView`] (their own bridge).

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

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
    }
}
