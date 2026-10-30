//! Image > Adjustments: destructive adjustments of the active pixel layer,
//! within the selection. A dialog edits a scratch adjustment block (the 4-byte
//! key then the payload, the encoding an adjustment layer stores) through the
//! same descriptor pages the Properties panel uses, previews it on the canvas
//! through the Filter menu's preview machinery (cancelled by
//! `filter_preview_cancel`), and applies it as one state named for the
//! adjustment. Invert, Desaturate, Equalize, and the Auto commands apply
//! directly. Free functions over a [`PictureView`] (their own bridge).
//!
//! [`PictureView`]: super::qobject::PictureView

mod gradient_map;
mod image_mode;
mod image_ops;
mod image_size;
mod imp;
pub(in crate::cxxqt_object) mod jobs;
mod new_document;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The block the `kind` dialog opens on (`"levels"`, `"curves"`, …; colours `0xRRGGBB` for Gradient Map), or empty for an unknown kind.
        fn image_adjustment_default(kind: &QString, foreground: u32, background: u32) -> Vec<u8>;

        /// `block`'s descriptor page (the Properties panel's row format), or empty.
        fn image_adjustment_page(block: &[u8]) -> QStringList;

        /// `block` with parameter `key` set to `value`, or empty when refused.
        fn image_adjustment_set(block: &[u8], key: &QString, value: f64) -> Vec<u8>;

        /// A Curves `block`'s `channel` points as `"x,y x,y …"`, or empty.
        fn image_adjustment_curve(block: &[u8], channel: i32) -> QString;

        /// A Curves `block` with `channel`'s points replaced, or empty when refused.
        fn image_adjustment_set_curve(block: &[u8], channel: i32, points: &QString) -> Vec<u8>;

        /// Preview `block` on the active layer over the document rect `(x, y, w, h)` (the whole layer when `w` is not positive), re-applied from the pre-preview pixels; no history. False when refused.
        fn image_adjust_preview(
            view: Pin<&mut PictureView>,
            block: &[u8],
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply `block` to the whole active layer as one `label` state (an open preview is replaced). False when refused.
        fn image_adjust_apply(view: Pin<&mut PictureView>, block: &[u8], label: &QString) -> bool;

        /// Apply a dialog-less command — `"invert"`, `"desaturate"`, `"equalize"`, `"auto-tone"`, `"auto-contrast"`, `"auto-color"` — as one state named as CS6 names it. False when refused.
        fn image_adjust_direct(view: Pin<&mut PictureView>, kind: &QString) -> bool;
    }
}
