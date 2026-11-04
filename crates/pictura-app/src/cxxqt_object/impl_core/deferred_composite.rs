//! The bridge for a large Move commit's off-thread composite (#1157).

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::{
    move_composite_pending, move_composite_set_delay, CompositeJob, DeferredComposite,
};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether a Move commit's composite is still on its worker.
        fn move_composite_pending(view: &PictureView) -> bool;

        /// Hold a started Move composite for `ms` milliseconds (test seam).
        fn move_composite_set_delay(view: Pin<&mut PictureView>, ms: u64);
    }
}
