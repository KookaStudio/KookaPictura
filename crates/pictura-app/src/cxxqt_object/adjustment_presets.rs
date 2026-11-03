//! Adjustment presets: the engine's kind facts and the user store, exposed to
//! the shell's Preset menus. Free functions (their own bridge, so no
//! `PictureView` declaration list grows).

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[namespace = "pictura"]
    extern "Rust" {
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

        /// Whether `kind` is in the CS6 save whitelist.
        fn adjustment_preset_can_save(kind: &QString) -> bool;

        /// Whether `kind`'s Properties page carries the Preset menu.
        fn adjustment_preset_supports(kind: &QString) -> bool;

        /// Whether two blocks decode to the same adjustment.
        fn adjustment_preset_matches(a: &[u8], b: &[u8]) -> bool;
    }
}
