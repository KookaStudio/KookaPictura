//! Colour Settings and picker services. Free functions over a `PictureView`
//! (their own bridge) so the central qobject declaration list stays under its
//! size cap.

mod imp;

use super::qobject::PictureView;
pub use imp::*;

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
        /// Store the Color Settings values on the view. Codes follow the
        /// session schema: policy 0/1/2, intent 0/1/2/3, working CMYK 0.
        fn set_color_settings(
            view: Pin<&mut PictureView>,
            cmyk_policy: i32,
            working_cmyk: i32,
            intent: i32,
            bpc: bool,
            dither: bool,
            ask_mismatch: bool,
            ask_missing: bool,
        );
        fn cmyk_policy(view: &PictureView) -> i32;
        fn render_intent(view: &PictureView) -> i32;
        fn black_point_compensation(view: &PictureView) -> bool;
        fn use_dither(view: &PictureView) -> bool;
        fn ask_mismatch_open(view: &PictureView) -> bool;
        fn ask_missing_open(view: &PictureView) -> bool;
        fn set_picker_choice(view: Pin<&mut PictureView>, choice: i32);
        fn picker_choice(view: &PictureView) -> i32;

        /// Convert the active RGB document to a built-in destination with
        /// explicit options and optional flattening; one history state.
        fn convert_profile_with_options(
            view: Pin<&mut PictureView>,
            profile_index: i32,
            intent: i32,
            bpc: bool,
            dither: bool,
            flatten: bool,
        ) -> bool;

        /// Preview the same conversion without history.
        fn preview_convert_profile(
            view: Pin<&mut PictureView>,
            profile_index: i32,
            intent: i32,
            bpc: bool,
            dither: bool,
            flatten: bool,
        ) -> bool;

        /// Restore the pre-preview document.
        fn cancel_convert_preview(view: Pin<&mut PictureView>) -> bool;

        /// The working-CMYK projection of an sRGB colour as C,M,Y,K percentages.
        fn picker_cmyk(r: i32, g: i32, b: i32) -> Vec<i32>;

        /// The sRGB colour for C,M,Y,K percentages, as `0x00RRGGBB`.
        fn picker_rgb(c: i32, m: i32, y: i32, k: i32) -> u32;

        /// Whether the sRGB colour is outside the working-CMYK gamut.
        fn picker_in_gamut(r: i32, g: i32, b: i32) -> bool;

        /// The closest printable sRGB colour, as `0x00RRGGBB`.
        fn picker_closest_rgb(r: i32, g: i32, b: i32) -> u32;

        /// Probe a PSD/PSB or raster file's embedded profile. Returns
        /// `[kind, description]`, kind `missing` / `malformed` / `srgb` /
        /// `mismatch`.
        fn probe_file_profile(path: &QString) -> QStringList;

        /// The document's working-profile description, or `sRGB IEC61966-2.1`.
        fn document_profile_name(view: &PictureView) -> QString;

        /// The saved Embed Color Profile choice: -1 unset, 0 off, 1 on.
        fn save_embed_profile(view: &PictureView) -> i32;

        /// Remember the Save As Embed Color Profile choice for this document.
        fn set_save_embed_profile(view: Pin<&mut PictureView>, on: bool);

        /// The default Embed Color Profile choice for `native` (PSD/PSB).
        fn default_save_embed_profile(view: &PictureView, native: bool) -> bool;

        /// A built-in profile's ICC bytes: 0 sRGB, 1 Adobe RGB, 2 ProPhoto RGB.
        fn builtin_profile_icc(index: i32) -> Vec<u8>;

        /// Override the incoming policy for the next open only (0/1/2).
        fn set_open_policy_override(view: Pin<&mut PictureView>, code: i32);
    }
}
