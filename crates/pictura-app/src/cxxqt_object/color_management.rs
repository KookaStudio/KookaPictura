//! Colour Settings and picker services. Free functions over a `PictureView`
//! (their own bridge) so the central qobject declaration list stays under its
//! size cap.

use super::qobject::PictureView;
use super::state::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_codec::{ConversionOptions, Intent, Policy, Profile};
use pictura_core::ColorMode;

use std::sync::{OnceLock, RwLock};

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

static PICKER_CMYK: OnceLock<RwLock<Vec<u8>>> = OnceLock::new();

fn picker_store() -> &'static RwLock<Vec<u8>> {
    PICKER_CMYK.get_or_init(|| RwLock::new(Profile::coated_cmyk().to_icc()))
}

fn intent_from_code(code: i32) -> Intent {
    match code {
        0 => Intent::Perceptual,
        2 => Intent::Saturation,
        3 => Intent::AbsoluteColorimetric,
        _ => Intent::RelativeColorimetric,
    }
}

fn builtin_profile(index: i32) -> Option<Profile> {
    match index {
        0 => Some(Profile::srgb()),
        1 => Some(Profile::adobe_rgb()),
        2 => Some(Profile::pro_photo()),
        _ => None,
    }
}

fn working_profile() -> Profile {
    let bytes = picker_store().read().map(|v| v.clone()).unwrap_or_default();
    Profile::from_icc(&bytes).unwrap_or_else(|_| Profile::coated_cmyk())
}

fn picker_options() -> ConversionOptions {
    ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    }
}

fn rgb_bytes(r: i32, g: i32, b: i32) -> [u8; 3] {
    [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    ]
}

fn cmyk_bytes(c: i32, m: i32, y: i32, k: i32) -> [u8; 4] {
    [
        (c.clamp(0, 100) * 255 / 100) as u8,
        (m.clamp(0, 100) * 255 / 100) as u8,
        (y.clamp(0, 100) * 255 / 100) as u8,
        (k.clamp(0, 100) * 255 / 100) as u8,
    ]
}

fn packed(rgb: &[u8]) -> u32 {
    (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2])
}

pub fn set_color_settings(
    mut view: Pin<&mut PictureView>,
    cmyk_policy: i32,
    working_cmyk: i32,
    intent: i32,
    bpc: bool,
    dither: bool,
    ask_mismatch: bool,
    ask_missing: bool,
) {
    let profile = match working_cmyk {
        0 => Profile::coated_cmyk().to_icc(),
        _ => Profile::coated_cmyk().to_icc(),
    };
    {
        let mut rust = view.as_mut().rust_mut();
        if let Some(policy) = Policy::from_code(cmyk_policy) {
            rust.cmyk_policy = policy;
        }
        rust.working_cmyk = profile.clone();
        rust.conversion_options = ConversionOptions {
            intent: intent_from_code(intent),
            black_point_compensation: bpc,
            dither,
        };
        rust.ask_mismatch_open = ask_mismatch;
        rust.ask_missing_open = ask_missing;
    }
    if let Ok(mut slot) = picker_store().write() {
        *slot = profile;
    }
}

pub fn cmyk_policy(view: &PictureView) -> i32 {
    view.rust().cmyk_policy.to_code()
}

pub fn render_intent(view: &PictureView) -> i32 {
    match view.rust().conversion_options.intent {
        Intent::Perceptual => 0,
        Intent::Saturation => 2,
        Intent::AbsoluteColorimetric => 3,
        Intent::RelativeColorimetric => 1,
    }
}

pub fn black_point_compensation(view: &PictureView) -> bool {
    view.rust().conversion_options.black_point_compensation
}

pub fn use_dither(view: &PictureView) -> bool {
    view.rust().conversion_options.dither
}

pub fn ask_mismatch_open(view: &PictureView) -> bool {
    view.rust().ask_mismatch_open
}

pub fn ask_missing_open(view: &PictureView) -> bool {
    view.rust().ask_missing_open
}

pub fn set_picker_choice(mut view: Pin<&mut PictureView>, choice: i32) {
    view.as_mut().rust_mut().picker_choice = if choice == 1 { 1 } else { 0 };
}

pub fn picker_choice(view: &PictureView) -> i32 {
    view.rust().picker_choice
}

fn convert_preview(
    mut rust: Pin<&mut PictureViewRust>,
    profile_index: i32,
    intent: i32,
    bpc: bool,
    dither: bool,
    flatten: bool,
) -> bool {
    let Some(dst) = builtin_profile(profile_index) else {
        return false;
    };
    let options = ConversionOptions {
        intent: intent_from_code(intent),
        black_point_compensation: bpc,
        dither,
    };
    let Some(mut converted) = rust.mode_preview_base() else {
        return false;
    };
    if converted.mode != ColorMode::Rgb {
        return false;
    }
    if flatten && pictura_render::flatten(&mut converted).is_err() {
        return false;
    }
    if !pictura_codec::convert_document_with(&mut converted, &dst, options) {
        return false;
    }
    rust.session.doc = Some(converted);
    true
}

pub fn convert_profile_with_options(
    mut view: Pin<&mut PictureView>,
    profile_index: i32,
    intent: i32,
    bpc: bool,
    dither: bool,
    flatten: bool,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let Some(dst) = builtin_profile(profile_index) else {
        return false;
    };
    let options = ConversionOptions {
        intent: intent_from_code(intent),
        black_point_compensation: bpc,
        dither,
    };
    let converted = {
        let mut rust = view.as_mut().rust_mut();
        if let Some(base) = rust.session.live.take_mode_preview() {
            rust.session.doc = Some(base);
        }
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        if doc.mode != ColorMode::Rgb {
            return false;
        }
        if flatten && pictura_render::flatten(doc).is_err() {
            return false;
        }
        pictura_codec::convert_document_with(doc, &dst, options)
    };
    if !converted {
        view.as_mut().recomposite();
        return false;
    }
    view.as_mut().edit("Convert to Profile", |_| Edit::Full);
    refresh_target(view);
    true
}

/// Flatten can remove the layer the target named; re-point it at the surviving
/// layer (or the topmost layer) after the conversion.
fn refresh_target(mut view: Pin<&mut PictureView>) {
    let (target, count) = {
        let rust = view.as_mut().rust_mut();
        let mut target = rust.layer_target.clone();
        if let Some(doc) = rust.session.doc.as_ref() {
            target.retain_resolving(doc);
        }
        let count = rust.session.doc.as_ref().map_or(0, |doc| doc.layers.len());
        (target, count)
    };
    let mut rust = view.as_mut().rust_mut();
    rust.set_layer_target(target);
    if rust.layer_target.is_empty() && count > 0 {
        rust.select_only_layer(&(count - 1).to_string());
    }
}

pub fn preview_convert_profile(
    mut view: Pin<&mut PictureView>,
    profile_index: i32,
    intent: i32,
    bpc: bool,
    dither: bool,
    flatten: bool,
) -> bool {
    let converted = {
        let rust = view.as_mut().rust_mut();
        convert_preview(rust, profile_index, intent, bpc, dither, flatten)
    };
    if converted {
        view.as_mut().recomposite();
    }
    converted
}

pub fn cancel_convert_preview(mut view: Pin<&mut PictureView>) -> bool {
    if !view.as_mut().rust_mut().cancel_mode_preview() {
        return false;
    }
    view.as_mut().recomposite();
    true
}

pub fn picker_cmyk(r: i32, g: i32, b: i32) -> Vec<i32> {
    let profile = working_profile();
    let rgb = rgb_bytes(r, g, b);
    pictura_codec::rgb_to_cmyk_with_profile(&rgb, &profile, picker_options())
        .map(|cmyk| {
            cmyk.into_iter()
                .map(|v| (u32::from(v) * 100 + 127) as i32 / 255)
                .collect()
        })
        .unwrap_or_else(|| vec![0, 0, 0, 100])
}

pub fn picker_rgb(c: i32, m: i32, y: i32, k: i32) -> u32 {
    let profile = working_profile();
    let cmyk = cmyk_bytes(c, m, y, k);
    pictura_codec::cmyk_to_rgb_with_profile(&cmyk, &profile, picker_options())
        .map(|rgb| packed(&rgb))
        .unwrap_or(u32::MAX)
}

pub fn picker_in_gamut(r: i32, g: i32, b: i32) -> bool {
    let profile = working_profile();
    let options = picker_options();
    let rgb = rgb_bytes(r, g, b);
    let Some(cmyk) = pictura_codec::rgb_to_cmyk_with_profile(&rgb, &profile, options) else {
        return true;
    };
    let Some(back) = pictura_codec::cmyk_to_rgb_with_profile(&cmyk, &profile, options) else {
        return true;
    };
    rgb.iter()
        .zip(back.iter())
        .all(|(a, b)| (*a as i16 - *b as i16).abs() <= 1)
}

pub fn probe_file_profile(path: &QString) -> QStringList {
    let path = path.to_string();
    let Ok(bytes) = std::fs::read(&path) else {
        return QStringList::default();
    };
    let probe = if bytes.starts_with(b"8BPS") {
        pictura_codec::probe_psd_profile(&bytes)
    } else {
        pictura_codec::probe_raster_profile(&bytes)
    };
    let Some(icc) = probe.embedded.as_deref() else {
        let kind = if probe.present {
            "malformed"
        } else {
            "missing"
        };
        return [kind, ""]
            .into_iter()
            .map(QString::from)
            .collect::<QStringList>();
    };
    let description = pictura_codec::profile_description(icc).unwrap_or_default();
    let kind = if probe.is_srgb { "srgb" } else { "mismatch" };
    [kind, description.as_str()]
        .into_iter()
        .map(QString::from)
        .collect::<QStringList>()
}

pub fn document_profile_name(view: &PictureView) -> QString {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::from("sRGB IEC61966-2.1");
    };
    doc.document_icc
        .as_deref()
        .and_then(pictura_codec::profile_description)
        .map(|name| QString::from(name.as_str()))
        .unwrap_or_else(|| QString::from("sRGB IEC61966-2.1"))
}

pub fn builtin_profile_icc(index: i32) -> Vec<u8> {
    builtin_profile(index)
        .map(|profile| profile.to_icc())
        .unwrap_or_default()
}

pub fn save_embed_profile(view: &PictureView) -> i32 {
    match view.rust().save_embed_profile {
        Some(on) => i32::from(on),
        None => -1,
    }
}

pub fn set_save_embed_profile(mut view: Pin<&mut PictureView>, on: bool) {
    view.as_mut().rust_mut().save_embed_profile = Some(on);
}

pub fn default_save_embed_profile(view: &PictureView, native: bool) -> bool {
    save_embed_default(view.rust(), native)
}

pub(super) fn save_embed_default(rust: &PictureViewRust, native: bool) -> bool {
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    if native && doc.source_mode == Some(ColorMode::Cmyk) {
        return true;
    }
    doc.document_icc
        .as_deref()
        .and_then(|icc| Profile::from_icc(icc).ok())
        .is_some_and(|profile| !profile.is_srgb())
}

pub fn set_open_policy_override(mut view: Pin<&mut PictureView>, code: i32) {
    view.as_mut().rust_mut().open_policy_override = Policy::from_code(code);
}

pub fn picker_closest_rgb(r: i32, g: i32, b: i32) -> u32 {
    let profile = working_profile();
    let options = picker_options();
    let rgb = rgb_bytes(r, g, b);
    pictura_codec::rgb_to_cmyk_with_profile(&rgb, &profile, options)
        .and_then(|cmyk| pictura_codec::cmyk_to_rgb_with_profile(&cmyk, &profile, options))
        .map(|back| packed(&back))
        .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_service_cmyk_round_trips_through_the_working_profile() {
        let cmyk = picker_cmyk(200, 50, 100);
        assert_eq!(cmyk.len(), 4);
        assert!(cmyk.iter().all(|v| (0..=100).contains(v)));
        let back = picker_rgb(cmyk[0], cmyk[1], cmyk[2], cmyk[3]);
        assert_ne!(back, u32::MAX);
        let [r, g, b] = [
            ((back >> 16) & 0xFF) as i32,
            ((back >> 8) & 0xFF) as i32,
            (back & 0xFF) as i32,
        ];
        assert!((r - 200).abs() <= 8, "{r}");
        assert!((g - 50).abs() <= 8, "{g}");
        assert!((b - 100).abs() <= 8, "{b}");
    }

    #[test]
    fn picker_service_clamps_out_of_range_input() {
        let cmyk = picker_cmyk(300, -4, 999);
        assert!(cmyk.iter().all(|v| (0..=100).contains(v)));
        assert_ne!(picker_rgb(-10, 120, 50, 300), u32::MAX);
    }

    #[test]
    fn picker_service_gamut_is_the_round_trip_test() {
        let gray = picker_in_gamut(128, 128, 128);
        assert!(gray, "neutral grey is printable");
        assert_eq!(
            picker_closest_rgb(0, 0, 255),
            picker_rgb(
                picker_cmyk(0, 0, 255)[0],
                picker_cmyk(0, 0, 255)[1],
                picker_cmyk(0, 0, 255)[2],
                picker_cmyk(0, 0, 255)[3],
            )
        );
    }
}
