//! What the shell needs to know about the level-0 frame it holds a copy of.
//! A free function over a [`PictureView`] (its own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The revision of the whole level-0 frame: it changes only when the
        /// frame is rebuilt, never for a region the view blitted. A canvas that
        /// took `image()` at this revision is still current.
        fn frame_revision(view: &PictureView) -> u64;

        /// The view-pyramid level the canvas must present, or 0 when the zoom
        /// picks it: a reduced stroke preview's level, else the coarse level
        /// standing in for a pyramid a worker is still building. A GPU stroke
        /// presents the zoom-selected level.
        fn preview_present_level(view: &PictureView) -> i32;
    }
}
