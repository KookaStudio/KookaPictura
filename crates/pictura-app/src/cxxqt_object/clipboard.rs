//! The Edit clipboard bridge: Cut / Copy / Copy Merged / Paste / Paste in
//! Place / Paste Into / Paste Outside / Clear and `Purge > Clipboard`. Free
//! functions over a [`PictureView`] (its own bridge, so the `PictureView`
//! declaration list does not grow). One clipboard serves every open document,
//! so a copy in one tab pastes into another; the shell mirrors it to the system
//! clipboard through `clipboard_export` / `clipboard_import`. Copy and Purge
//! record no history; Cut, Clear, and each paste record exactly one state, and
//! every refusal records nothing.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    /// Which Edit paste command runs.
    #[namespace = "pictura"]
    enum PasteKind {
        Plain,
        InPlace,
        Into,
        Outside,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// `Copy` (`merged == false`) / `Copy Merged`: replace the clipboard with the selected pixels of the layer at `path` or of the visible composite; the whole layer/canvas without a selection. No history; false (clipboard kept) when nothing is copied.
        fn clipboard_copy(view: Pin<&mut PictureView>, path: &QString, merged: bool) -> bool;

        /// `Cut`: `Copy` then `Clear` as one "Cut" state; false without mutating on either refusal.
        fn clipboard_cut(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// `Clear`: erase the selection (the whole layer without one) from the layer at `path`; one "Clear" state; false on a lock or when nothing changes.
        fn clipboard_clear(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Paste the clipboard as a new layer above `path`: Plain and InPlace at the clip's own document rect, Into centred on the selection bounds, Outside centred on document point `(centre_x, centre_y)`. Every paste deselects into the reselect store; Into/Outside also mask by the selection. One state; returns the new path or empty.
        fn clipboard_paste(
            view: Pin<&mut PictureView>,
            path: &QString,
            kind: PasteKind,
            centre_x: f64,
            centre_y: f64,
        ) -> QString;

        /// Whether the clipboard holds a copy.
        fn clipboard_has_contents() -> bool;

        /// The copy as packed straight RGBA (selection folded into alpha) for the system clipboard; empty with zero dimensions when there is none.
        fn clipboard_export(width: &mut i32, height: &mut i32) -> Vec<u8>;

        /// Replace the copy with another application's packed straight RGBA image, placed at the canvas origin; false (copy kept) for a zero dimension or a short buffer.
        fn clipboard_import(width: i32, height: i32, rgba: &[u8]) -> bool;

        /// `Purge > Clipboard`: drop the copy. Not undoable.
        fn clipboard_purge();

        /// Deep-copy the layer node at `path` in `source` into `view` by the New Layer insertion rule relative to `selection` — the Layers panel drag onto another document. `source` and `view` must be different documents. One "Duplicate Layer" state; returns the new path, or empty for an unknown path or a different color mode or bit depth.
        fn copy_layer_from_document(
            view: Pin<&mut PictureView>,
            source: &PictureView,
            path: &QString,
            selection: &QString,
        ) -> QString;
    }
}
