//! The Photoshop PDF bridge (`interop/pdf-export`): save the document through
//! `pictura_codec::pdf` with JPEG encoded at the Qt boundary, reopen a
//! Pictura Photoshop PDF from its embedded PSD, and the option, preset, and
//! `.joboptions` helpers the Save Adobe PDF dialog edits through. Options
//! travel as canonical `.joboptions` text (design D3). Free functions over a
//! [`PictureView`] (their own bridge, so the `PictureView` list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;
pub(super) mod import;

use super::qobject::PictureView;
pub use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("encode_image.h");
        /// Encode interleaved 8-bit samples (1 gray, 3 RGB, 4 CMYK) as a JPEG at `quality` 0–100; empty on failure.
        fn encode_jpeg(
            samples: &[u8],
            width: i32,
            height: i32,
            components: i32,
            quality: i32,
        ) -> Vec<u8>;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Write the document to `path` as a Photoshop PDF with the canonical `options` text; on success record the path, clear the modified state, and remember the options. Returns an empty string on success, else the reason.
        fn pdf_save(view: Pin<&mut PictureView>, path: &QString, options: &QString) -> QString;

        /// The font family of every type layer, one per line, for the shell to register before a save sets them as live text.
        fn pdf_type_fonts(view: &PictureView) -> QString;

        /// The options this document was last saved to PDF with; empty when never.
        fn pdf_view_options(view: &PictureView) -> QString;

        /// Open a Photoshop PDF with its layers (Pictura's embedded PSD, or Photoshop's own editing data rebuilt into a PSD) into the view, decrypting it with `password` (empty: the password this view last opened it with, else none). `ok`; else, the view unchanged, `none` (no editing data: import it), `password` (needs one), or `wrongpassword`.
        fn pdf_open(view: Pin<&mut PictureView>, path: &QString, password: &QString) -> QString;

        /// `[Photoshop Default]` as canonical options text.
        fn pdf_default_options() -> QString;

        /// One field of `options` as text (enums as their list index); empty for an unknown field.
        fn pdf_option(options: &QString, field: &QString) -> QString;

        /// `options` with `field` set to `value` and the gating re-applied; unchanged on refusal.
        fn pdf_set_option(options: &QString, field: &QString, value: &QString) -> QString;

        /// Whether the standard or compatibility level forces `field`.
        fn pdf_option_locked(options: &QString, field: &QString) -> bool;

        /// The Summary page text: one setting or warning per line.
        fn pdf_summary(options: &QString) -> QString;

        /// The built-in preset names, one per line.
        fn pdf_builtin_presets() -> QString;

        /// A built-in preset as canonical options text; empty for an unknown name.
        fn pdf_builtin_preset(name: &QString) -> QString;

        /// Parse a `.joboptions` file into canonical options text; empty when it holds no parameters.
        fn pdf_parse_joboptions(text: &QString) -> QString;

        /// The `.joboptions` text a preset file stores (no Save options, no Security).
        fn pdf_preset_text(options: &QString) -> QString;

        /// Whether two option texts agree on every preset setting.
        fn pdf_same_preset(a: &QString, b: &QString) -> bool;
    }
}
