//! The Channels panel's document queries and edits: the colour mode its rows
//! follow (the opened file's mode, which the working RGB / Gray data stands
//! in for), and the document's alpha channels (saved selections, named
//! "Alpha 1", "Alpha 2", … by position) — their thumbnails, New Channel, and
//! Delete Channel. Free functions over a [`PictureView`] (their own bridge).
//!
//! [`PictureView`]: super::qobject::PictureView

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
        /// The mode the Channels panel lists: `"rgb"`, `"grayscale"`, `"cmyk"`, `"lab"`, `"multichannel"`, `"indexed"`, `"bitmap"`, or `"duotone"` — the opened file's mode when it was converted for editing. Empty without a document.
        fn channels_mode(view: &PictureView) -> QString;

        /// Alpha channel `index` scaled to fit `max_side` (one byte a pixel, row-major), its size in `width` / `height`; empty for a bad index.
        fn channels_alpha_thumbnail(
            view: &PictureView,
            index: i32,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// Add an alpha channel, black (all masked) as CS6's New Channel makes it; one "New Channel" state. Returns its index, or -1 without a document.
        fn channels_new_alpha(view: Pin<&mut PictureView>) -> i32;

        /// Delete alpha channel `index`; one "Delete Channel" state. False for a bad index.
        fn channels_delete_alpha(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Load colour channel row `row` (0 = the composite) as the selection; `mask` is the visible RGB channels (bit 0 red, 1 green, 2 blue). False for a row outside the mode's colour rows.
        fn channels_load_color_selection(view: Pin<&mut PictureView>, row: i32, mask: i32) -> bool;

        /// The active layer's raster-mask channel name (`"<layer name> Mask"`), or
        /// empty when the active layer carries no raster mask.
        fn channels_active_mask_name(view: &PictureView) -> QString;

        /// The active layer's raster-mask coverage scaled to fit `max_side` (one
        /// byte a pixel, row-major), its size in `width` / `height`; empty without
        /// a mask. A disabled mask still shows its stored coverage.
        fn channels_active_mask_thumbnail(
            view: &PictureView,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;
    }
}
