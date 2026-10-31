use super::*;

use super::super::state::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_codec::{ConversionOptions, Intent, Policy, Profile};
use pictura_color::SpaceTransform;
use pictura_core::ColorMode;

use std::sync::{OnceLock, RwLock};

/// The picker's sRGB <-> working-CMYK transforms. A drag in the picker asks for
/// the gamut test on every mouse move, so they are built once per working
/// profile and conversion-option pair rather than per call.
pub(super) struct PickerTransforms {
    to_cmyk: SpaceTransform,
    to_rgb: SpaceTransform,
}

impl PickerTransforms {
    fn cmyk(&self, rgb: [u8; 3]) -> [u8; 4] {
        let mut inks = [0; 4];
        self.to_cmyk.apply(&rgb, &mut inks);
        inks
    }

    fn rgb(&self, cmyk: [u8; 4]) -> [u8; 3] {
        let mut rgb = [0; 3];
        self.to_rgb.apply(&cmyk, &mut rgb);
        rgb
    }
}

pub(super) static PICKER: OnceLock<RwLock<Option<PickerTransforms>>> = OnceLock::new();

#[cfg(test)]
pub(super) static PICKER_BUILDS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub(super) fn picker_slot() -> &'static RwLock<Option<PickerTransforms>> {
    PICKER.get_or_init(|| {
        RwLock::new(picker_transforms(
            &Profile::coated_cmyk().to_icc(),
            default_picker_options(),
        ))
    })
}

/// The picker's default conversion options: relative colorimetric with
/// black-point compensation, and dither off.
pub(super) fn default_picker_options() -> ConversionOptions {
    ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    }
}

pub(super) fn picker_transforms(
    icc: &[u8],
    options: ConversionOptions,
) -> Option<PickerTransforms> {
    #[cfg(test)]
    PICKER_BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let profile = Profile::from_icc(icc).unwrap_or_else(|_| Profile::coated_cmyk());
    let srgb = Profile::srgb();
    let ConversionOptions {
        intent,
        black_point_compensation: bpc,
        ..
    } = options;
    Some(PickerTransforms {
        to_cmyk: SpaceTransform::new(&srgb, &profile, intent, bpc).ok()?,
        to_rgb: SpaceTransform::new(&profile, &srgb, intent, bpc).ok()?,
    })
}

/// Rebuild the picker pair from an installed profile and its conversion options.
/// Dither is never applied to the pair: it converts one colour, not a plane.
pub(super) fn install_picker(icc: &[u8], options: ConversionOptions) {
    let options = ConversionOptions {
        dither: false,
        ..options
    };
    let transforms = picker_transforms(icc, options);
    if let Ok(mut slot) = picker_slot().write() {
        *slot = transforms;
    }
}

/// `None` when the picker transforms could not be built (or the lock is
/// poisoned); each service then returns its fallback.
pub(super) fn with_picker<T>(f: impl FnOnce(&PickerTransforms) -> T) -> Option<T> {
    picker_slot().read().ok()?.as_ref().map(f)
}

pub(super) fn intent_from_code(code: i32) -> Intent {
    match code {
        0 => Intent::Perceptual,
        2 => Intent::Saturation,
        3 => Intent::AbsoluteColorimetric,
        _ => Intent::RelativeColorimetric,
    }
}

pub(super) fn builtin_profile(index: i32) -> Option<Profile> {
    match index {
        0 => Some(Profile::srgb()),
        1 => Some(Profile::adobe_rgb()),
        2 => Some(Profile::pro_photo()),
        _ => None,
    }
}

pub(super) fn rgb_bytes(r: i32, g: i32, b: i32) -> [u8; 3] {
    [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    ]
}

pub(super) fn cmyk_bytes(c: i32, m: i32, y: i32, k: i32) -> [u8; 4] {
    [
        (c.clamp(0, 100) * 255 / 100) as u8,
        (m.clamp(0, 100) * 255 / 100) as u8,
        (y.clamp(0, 100) * 255 / 100) as u8,
        (k.clamp(0, 100) * 255 / 100) as u8,
    ]
}

pub(super) fn packed(rgb: &[u8]) -> u32 {
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
    let options = ConversionOptions {
        intent: intent_from_code(intent),
        black_point_compensation: bpc,
        dither,
    };
    {
        let mut rust = view.as_mut().rust_mut();
        if let Some(policy) = Policy::from_code(cmyk_policy) {
            rust.cmyk_policy = policy;
        }
        rust.working_cmyk = profile.clone();
        rust.conversion_options = options;
        rust.ask_mismatch_open = ask_mismatch;
        rust.ask_missing_open = ask_missing;
    }
    install_picker(&profile, options);
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

pub(super) fn convert_preview(
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
pub(super) fn refresh_target(mut view: Pin<&mut PictureView>) {
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
    with_picker(|picker| {
        picker
            .cmyk(rgb_bytes(r, g, b))
            .into_iter()
            .map(|v| (u32::from(v) * 100 + 127) as i32 / 255)
            .collect()
    })
    .unwrap_or_else(|| vec![0, 0, 0, 100])
}

pub fn picker_rgb(c: i32, m: i32, y: i32, k: i32) -> u32 {
    with_picker(|picker| packed(&picker.rgb(cmyk_bytes(c, m, y, k)))).unwrap_or(u32::MAX)
}

pub fn picker_in_gamut(r: i32, g: i32, b: i32) -> bool {
    let rgb = rgb_bytes(r, g, b);
    with_picker(|picker| {
        let back = picker.rgb(picker.cmyk(rgb));
        rgb.iter()
            .zip(back.iter())
            .all(|(a, b)| (*a as i16 - *b as i16).abs() <= 1)
    })
    .unwrap_or(true)
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

pub(in crate::cxxqt_object) fn save_embed_default(rust: &PictureViewRust, native: bool) -> bool {
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
    with_picker(|picker| packed(&picker.rgb(picker.cmyk(rgb_bytes(r, g, b))))).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_service_cmyk_round_trips_through_the_working_profile() {
        let _turn = take_turn();
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
        let _turn = take_turn();
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

    // nextest isolates each test in a process; under `cargo test` these share
    // one installed pair and build counter, so they take turns.
    static PICKER_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn take_turn() -> std::sync::MutexGuard<'static, ()> {
        let guard = PICKER_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        install_picker(&Profile::coated_cmyk().to_icc(), default_picker_options());
        guard
    }

    fn percents(inks: &[u8]) -> Vec<i32> {
        inks.iter()
            .map(|v| (u32::from(*v) * 100 + 127) as i32 / 255)
            .collect()
    }

    // The repo has one CMYK ICC: repoint its relative-colorimetric `B2A1` tag
    // at the different `B2A0` table to get a second profile.
    fn second_cmyk_icc() -> Vec<u8> {
        let mut icc = Profile::coated_cmyk_icc().to_vec();
        let count = u32::from_be_bytes(icc[128..132].try_into().unwrap()) as usize;
        let entry = |tag: &[u8; 4]| {
            (0..count)
                .map(|i| 132 + 12 * i)
                .find(|at| &icc[*at..*at + 4] == tag)
                .expect("bundled profile carries the tag")
        };
        let (from, to) = (entry(b"B2A0"), entry(b"B2A1"));
        icc.copy_within(from + 4..from + 12, to + 4);
        icc
    }

    fn builds() -> usize {
        PICKER_BUILDS.load(std::sync::atomic::Ordering::Relaxed)
    }

    // The services' values for a sweep of colours, through whatever pair is
    // installed.
    fn sweep() -> Vec<(bool, u32, Vec<i32>, u32)> {
        let mut out = Vec::new();
        for r in (0..=255).step_by(51) {
            for g in (0..=255).step_by(85) {
                for b in (0..=255).step_by(85) {
                    let cmyk = picker_cmyk(r, g, b);
                    let back = picker_rgb(cmyk[0], cmyk[1], cmyk[2], cmyk[3]);
                    out.push((
                        picker_in_gamut(r, g, b),
                        picker_closest_rgb(r, g, b),
                        cmyk,
                        back,
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn picker_services_reuse_their_transforms() {
        let _turn = take_turn();
        assert!(picker_in_gamut(128, 128, 128));
        let first = builds();
        assert!(first >= 1, "the first call builds the pair");

        let options = default_picker_options();
        let coated = Profile::coated_cmyk();
        for r in (0..=255).step_by(51) {
            for b in (0..=255).step_by(51) {
                let rgb = rgb_bytes(r, 40, b);
                let inks = pictura_codec::rgb_to_cmyk_with_profile(&rgb, &coated, options)
                    .expect("one-shot cmyk");
                let back = pictura_codec::cmyk_to_rgb_with_profile(&inks, &coated, options)
                    .expect("one-shot rgb");
                let in_gamut = rgb.iter().zip(&back).all(|(a, b)| a.abs_diff(*b) <= 1);
                assert_eq!(picker_in_gamut(r, 40, b), in_gamut, "{rgb:?}");
                assert_eq!(picker_closest_rgb(r, 40, b), packed(&back), "{rgb:?}");
                assert_eq!(picker_cmyk(r, 40, b), percents(&inks), "{rgb:?}");
            }
        }
        sweep();
        assert_eq!(
            builds(),
            first,
            "no call after the first builds a transform"
        );
    }

    #[test]
    fn picker_cmyk_white_has_no_ink() {
        let _turn = take_turn();
        assert_eq!(picker_cmyk(255, 255, 255), vec![0, 0, 0, 0]);
    }

    #[test]
    fn picker_cmyk_black_follows_profile() {
        let _turn = take_turn();
        let inks = picker_cmyk(0, 0, 0);
        let direct = pictura_codec::rgb_to_cmyk_with_profile(
            &[0, 0, 0],
            &Profile::coated_cmyk(),
            default_picker_options(),
        )
        .expect("one-shot cmyk");
        assert_eq!(inks, percents(&direct));
        assert!(inks[3] > 50, "black carries heavy K: {inks:?}");
        assert_ne!(inks, vec![0, 0, 0, 100], "not the naive formula");
    }

    #[test]
    fn picker_cmyk_round_trips_in_gamut() {
        let _turn = take_turn();
        // Muted colours a coated press reproduces; saturated sRGB blues and
        // yellows fall outside its gamut and map to the nearest ink.
        for [r, g, b] in [
            [200, 120, 80],
            [128, 128, 128],
            [150, 130, 110],
            [120, 140, 160],
            [180, 170, 150],
        ] {
            let back = picker_closest_rgb(r, g, b);
            for (channel, shift) in [(r, 16), (g, 8), (b, 0)] {
                let came_back = ((back >> shift) & 0xFF) as i32;
                assert!(
                    (came_back - channel).abs() <= 3,
                    "{r},{g},{b} came back as {back:06x}"
                );
            }
        }
    }

    #[test]
    fn picker_services_follow_the_installed_profile() {
        let _turn = take_turn();
        let options = default_picker_options();
        let icc = second_cmyk_icc();
        let second = Profile::from_icc(&icc).expect("patched profile parses");
        let coated = Profile::coated_cmyk();
        install_picker(&icc, options);
        let mut differs = false;
        for r in (0..=255).step_by(51) {
            for b in (0..=255).step_by(51) {
                let rgb = rgb_bytes(r, 40, b);
                let inks = pictura_codec::rgb_to_cmyk_with_profile(&rgb, &second, options)
                    .expect("one-shot cmyk");
                let back = pictura_codec::cmyk_to_rgb_with_profile(&inks, &second, options)
                    .expect("one-shot rgb");
                let shown = percents(&inks);
                let edited = pictura_codec::cmyk_to_rgb_with_profile(
                    &cmyk_bytes(shown[0], shown[1], shown[2], shown[3]),
                    &second,
                    options,
                )
                .expect("one-shot edit");
                let in_gamut = rgb.iter().zip(&back).all(|(a, b)| a.abs_diff(*b) <= 1);
                assert_eq!(picker_cmyk(r, 40, b), shown, "{rgb:?}");
                assert_eq!(
                    picker_rgb(shown[0], shown[1], shown[2], shown[3]),
                    packed(&edited),
                    "{rgb:?}"
                );
                assert_eq!(picker_in_gamut(r, 40, b), in_gamut, "{rgb:?}");
                assert_eq!(picker_closest_rgb(r, 40, b), packed(&back), "{rgb:?}");
                differs |= pictura_codec::rgb_to_cmyk_with_profile(&rgb, &coated, options)
                    .expect("one-shot coated")
                    != inks;
            }
        }
        install_picker(&coated.to_icc(), default_picker_options());
        assert!(differs, "the second profile converts differently");
    }

    #[test]
    fn picker_services_follow_the_installed_options() {
        let _turn = take_turn();
        let icc = Profile::coated_cmyk().to_icc();
        let perceptual = ConversionOptions {
            intent: Intent::Perceptual,
            black_point_compensation: false,
            dither: false,
        };
        install_picker(&icc, perceptual);
        let options = perceptual;
        let coated = Profile::coated_cmyk();
        let mut differs = false;
        for r in (0..=255).step_by(51) {
            for b in (0..=255).step_by(51) {
                let rgb = rgb_bytes(r, 40, b);
                let inks = pictura_codec::rgb_to_cmyk_with_profile(&rgb, &coated, options)
                    .expect("one-shot cmyk");
                let back = pictura_codec::cmyk_to_rgb_with_profile(&inks, &coated, options)
                    .expect("one-shot rgb");
                let shown = percents(&inks);
                let edited = pictura_codec::cmyk_to_rgb_with_profile(
                    &cmyk_bytes(shown[0], shown[1], shown[2], shown[3]),
                    &coated,
                    options,
                )
                .expect("one-shot edit");
                let in_gamut = rgb.iter().zip(&back).all(|(a, b)| a.abs_diff(*b) <= 1);
                assert_eq!(picker_cmyk(r, 40, b), shown, "{rgb:?}");
                assert_eq!(
                    picker_rgb(shown[0], shown[1], shown[2], shown[3]),
                    packed(&edited),
                    "{rgb:?}"
                );
                assert_eq!(picker_in_gamut(r, 40, b), in_gamut, "{rgb:?}");
                assert_eq!(picker_closest_rgb(r, 40, b), packed(&back), "{rgb:?}");
                differs |= pictura_codec::rgb_to_cmyk_with_profile(
                    &rgb,
                    &coated,
                    default_picker_options(),
                )
                .expect("one-shot default")
                    != inks;
            }
        }
        assert!(differs, "Perceptual without BPC changes a picker readout");
    }

    #[test]
    fn installing_a_profile_rebuilds_the_picker_pair() {
        let _turn = take_turn();
        let before = sweep();
        let built = builds();
        install_picker(&Profile::coated_cmyk().to_icc(), default_picker_options());
        assert_eq!(builds(), built + 1, "an install rebuilds the pair once");
        assert_eq!(sweep(), before, "the rebuilt pair converts alike");
        assert_eq!(builds(), built + 1, "calls after the install reuse it");
    }
}
