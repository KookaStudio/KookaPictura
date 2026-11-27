//! Adjustment presets: the engine's kind facts and the user store, exposed to
//! the shell's Preset menus. Free functions (their own bridge, so no
//! `PictureView` declaration list grows).

mod imp;
mod library;

use imp::*;
use library::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        type ReadPresetSet;

        fn preset_read_set(kind: i32, extension: &QString, bytes: &[u8]) -> Box<ReadPresetSet>;
        fn preset_set_error(set: &ReadPresetSet) -> QString;
        fn preset_set_warnings(set: &ReadPresetSet) -> QString;
        fn preset_set_size(set: &ReadPresetSet) -> i32;
        fn preset_row_name(set: &ReadPresetSet, index: i32) -> QString;
        fn preset_row_rgba(set: &ReadPresetSet, index: i32) -> u32;
        fn preset_row_values(set: &ReadPresetSet, index: i32) -> QString;
        fn preset_row_preview(set: &ReadPresetSet, index: i32) -> QImage;
        fn preset_row_bytes(set: &ReadPresetSet, index: i32) -> Vec<u8>;
        fn preset_encode_set(
            kind: i32,
            extension: &QString,
            rows: &[u8],
            output: &mut Vec<u8>,
        ) -> QString;
        fn preset_make_swatch(name: &QString, argb: u32) -> Vec<u8>;
        fn preset_make_builtin(kind: i32, name: &QString, index: i32) -> Vec<u8>;

        /// The catalogue kind of `block` (`"levels"`, `"curves"`, …), or empty.
        fn adjustment_preset_kind(block: &[u8]) -> QString;

        /// The saved preset names for `kind`, in store order.
        fn adjustment_preset_saved_names(kind: &QString) -> QStringList;

        /// The block saved as `name`, or empty.
        fn adjustment_preset_saved_block(kind: &QString, name: &QString) -> Vec<u8>;

        /// Save `block` as `name`; false when the name, kind, or block is refused.
        fn adjustment_preset_save(kind: &QString, name: &QString, block: &[u8]) -> bool;

        /// Delete the saved `name`; false when it was absent.
        fn adjustment_preset_delete(kind: &QString, name: &QString) -> bool;

        /// Whether the Properties panel offers to save presets for `kind`.
        fn adjustment_preset_can_save(kind: &QString) -> bool;

        /// Whether `kind`'s Properties page carries the Preset menu.
        fn adjustment_preset_supports(kind: &QString) -> bool;

        /// Whether two blocks decode to the same adjustment.
        fn adjustment_preset_matches(a: &[u8], b: &[u8]) -> bool;
    }
}
