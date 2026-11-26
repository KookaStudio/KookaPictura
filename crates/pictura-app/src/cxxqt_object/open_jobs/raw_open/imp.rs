use super::super::compatibility::NativeCompatibility;
use super::super::imp::{delay, history_for, open_budget, start, OpenedDocument, OpenedPath};
use super::store::{self, RawStores, Saved, StoreKind};
use super::PictureView;
use crate::cxxqt_object::helpers::safe_io;
use crate::cxxqt_object::helpers_composite::rgba_image;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_codec::{
    parse_exif, CameraRgb, DevelopedRgb, ImageBudget, PsdError, RawLook, RawSource, WhiteBalance,
};
use pictura_core::{
    BitDepth, Document, PicturaRawSettings, PsdRect, Samples, SourceChannels, SourcePlanes,
};
use pictura_render::DocumentComposite;
use std::io::Read;
use std::path::Path;

/// The prefix read to recognise a raw container.
const SNIFF_BYTES: u64 = 256 * 1024;
/// The long side of the preview copy kept beside the full source.
const PREVIEW_EDGE: usize = 1024;
const MIN_TEMPERATURE: f64 = 2000.0;
const MAX_TEMPERATURE: f64 = 50000.0;
const TINT_REACH: f64 = 150.0;

/// The camera metadata the dialog shows beside the histogram and in its status
/// bar, captured while the raw bytes are already in hand.
pub(in crate::cxxqt_object) struct RawPhotoInfo {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub ppi: Option<f64>,
    pub file_size: u64,
    /// `(row key, formatted value)` for the exposure tags the file carries.
    pub exposure: Vec<(&'static str, String)>,
}

/// A decoded raw file waiting for the dialog's settings.
pub(in crate::cxxqt_object) struct PendingRaw {
    pub(super) path: String,
    pub(super) source: RawSource,
    pub(super) preview: CameraRgb,
    pub(super) preview_look: RawLook,
    pub(super) seed: PicturaRawSettings,
    /// The file's settings-database key (its content hash).
    pub(super) key: String,
    /// The settings record the seed came from, the base of a later save.
    pub(super) record: Option<Vec<u8>>,
    /// The photo information the dialog's status bar and histogram show.
    pub(in crate::cxxqt_object) info: RawPhotoInfo,
    /// A Save Image running while the dialog is open (#402).
    pub(super) dng_job: Option<super::save_dng::DngJob>,
}

/// The size the develop produces: the default crop, turned by the
/// orientation.
fn developed_size(look: &RawLook, camera: &CameraRgb) -> (usize, usize) {
    let side = |v: f64, max: usize| {
        if v.is_finite() {
            (v.round().max(1.0) as usize).min(max)
        } else {
            max
        }
    };
    let (w, h) = (
        side(look.crop.width, camera.width),
        side(look.crop.height, camera.height),
    );
    if (5..=8).contains(&look.orientation) {
        (h, w)
    } else {
        (w, h)
    }
}

fn raw_photo_info(path: &str, bytes: &[u8], width: usize, height: usize) -> RawPhotoInfo {
    let exif = parse_exif(pictura_codec::raw_exif(bytes));
    RawPhotoInfo {
        name: Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        width,
        height,
        ppi: pictura_codec::resolution_ppi(&exif),
        file_size: bytes.len() as u64,
        exposure: pictura_codec::camera_exposure_rows(&exif),
    }
}

pub fn raw_sniff_path(path: &QString) -> bool {
    let Ok(file) = std::fs::File::open(path.to_string()) else {
        return false;
    };
    let mut prefix = Vec::new();
    file.take(SNIFF_BYTES).read_to_end(&mut prefix).is_ok()
        && pictura_codec::sniff_raw_prefix(&prefix)
}

pub(super) fn reason(error: PsdError) -> String {
    match error {
        PsdError::Unsupported(text) | PsdError::Invalid(text) => text,
        other => other.to_string(),
    }
}

/// Read and decode `path`, keeping a preview copy and the seed settings from
/// its record in `stores`, else its embedded XMP.
pub(super) fn decode_pending(
    path: &str,
    budget: &ImageBudget,
    stores: &RawStores,
    progress: &dyn Fn(u8, &str),
) -> Result<PendingRaw, String> {
    let bytes = safe_io::read_capped(path, safe_io::import_file_limit())
        .ok_or_else(|| "the file could not be read".to_string())?;
    progress(25, "Decoding raw data");
    let source = pictura_codec::decode_raw_source(&bytes, budget).map_err(reason)?;
    progress(85, "Preparing preview");
    let preview = pictura_codec::downscale_camera(&source.camera, PREVIEW_EDGE);
    let preview_look = scaled_look(&source.look, &source.camera, &preview);
    let key = pictura_codec::raw_settings_key(&bytes);
    let record = store::read_record(Path::new(path), &key, stores)
        .or_else(|| pictura_codec::raw_xmp(&bytes));
    let seed = seed_settings(record.as_deref(), source.as_shot);
    let (width, height) = developed_size(&source.look, &source.camera);
    let info = raw_photo_info(path, &bytes, width, height);
    Ok(PendingRaw {
        path: path.to_string(),
        source,
        preview,
        preview_look,
        seed,
        key,
        record,
        info,
        dng_job: None,
    })
}

fn scaled_look(look: &RawLook, full: &CameraRgb, copy: &CameraRgb) -> RawLook {
    look.scaled(
        copy.width as f64 / full.width.max(1) as f64,
        copy.height as f64 / full.height.max(1) as f64,
    )
}

/// Every control `record` sets, over Temperature and Tint at the rounded
/// as-shot white balance; a record labelled As Shot keeps the as-shot values.
pub(super) fn seed_settings(record: Option<&[u8]>, as_shot: WhiteBalance) -> PicturaRawSettings {
    let mut seed = record
        .map(pictura_codec::pictura_raw_from_xmp)
        .unwrap_or_default();
    let keep_as_shot = seed.mode.white_balance == Some(*b"AsSh");
    let basic = &mut seed.basic;
    if keep_as_shot || basic.temperature.is_none() {
        basic.temperature = Some(as_shot.temperature.round());
    }
    if keep_as_shot || basic.tint.is_none() {
        basic.tint = Some(as_shot.tint.round());
    }
    seed
}

/// Save `settings` as `pending`'s record in `stores`, labelled with the white
/// balance its values call for.
pub(super) fn save_settings(
    pending: &PendingRaw,
    settings: &PicturaRawSettings,
    stores: &RawStores,
) -> Result<Saved, String> {
    let labelled = store::with_white_balance_label(settings, pending.source.as_shot);
    store::save_record(
        Path::new(&pending.path),
        &pending.key,
        pending.record.as_deref(),
        &labelled,
        stores,
    )
}

/// Split the absolute white balance out of `settings` for the colour stage;
/// what remains runs in the relative develop stages.
pub(super) fn split_white_balance(
    settings: &PicturaRawSettings,
    as_shot: WhiteBalance,
) -> (WhiteBalance, PicturaRawSettings) {
    let white_balance = WhiteBalance {
        temperature: settings
            .basic
            .temperature
            .unwrap_or(as_shot.temperature)
            .clamp(MIN_TEMPERATURE, MAX_TEMPERATURE),
        tint: settings
            .basic
            .tint
            .unwrap_or(as_shot.tint)
            .clamp(-TINT_REACH, TINT_REACH),
    };
    let mut rest = settings.clone();
    rest.basic.temperature = None;
    rest.basic.tint = None;
    (white_balance, rest)
}

/// Develop `camera` with `settings`: the colour stage at the settings' white
/// balance, then the Pictura Raw controls at 16 bits.
pub(super) fn develop_pixels(
    camera: CameraRgb,
    look: &RawLook,
    as_shot: WhiteBalance,
    settings: &PicturaRawSettings,
) -> Result<DevelopedRgb, String> {
    let (white_balance, rest) = split_white_balance(settings, as_shot);
    let mut developed = pictura_codec::develop_raw(camera, look, white_balance).map_err(reason)?;
    developed.rgb = pictura_render::render_pictura_raw_rgb16(
        developed.width,
        developed.height,
        &developed.rgb,
        &rest,
    )
    .map_err(|error| error.to_string())?;
    Ok(developed)
}

pub(super) fn to8(v: u16) -> u8 {
    ((v as u32 + 128) / 257) as u8
}

/// The preview copy developed with `settings`, as RGBA8, its long side at
/// most `max_dim`.
pub(super) fn preview_rgba(
    pending: &PendingRaw,
    settings: &PicturaRawSettings,
    max_dim: usize,
) -> Option<(Vec<u8>, usize, usize)> {
    let copy = pictura_codec::downscale_camera(&pending.preview, max_dim.max(1));
    let look = scaled_look(&pending.preview_look, &pending.preview, &copy);
    let developed = develop_pixels(copy, &look, pending.source.as_shot, settings).ok()?;
    let mut rgba = Vec::with_capacity(developed.width * developed.height * 4);
    for px in developed.rgb.as_chunks::<3>().0.iter() {
        rgba.extend([to8(px[0]), to8(px[1]), to8(px[2]), 255]);
    }
    Some((rgba, developed.width, developed.height))
}

/// A new document from developed 16-bit sRGB: the 16-bit samples retained as
/// the native source and the layer's channels, an 8-bit display, opaque.
pub(super) fn raw_document(name: &str, developed: &DevelopedRgb, gpu: bool) -> Option<Document> {
    let (width, height) = (developed.width as u32, developed.height as u32);
    let plane = developed.width * developed.height;
    if plane == 0 || developed.rgb.len() != plane * 3 {
        return None;
    }
    let mut rgba = vec![255u8; plane * 4];
    let mut planar = vec![0u16; plane * 3];
    for (i, px) in developed.rgb.as_chunks::<3>().0.iter().enumerate() {
        for c in 0..3 {
            rgba[i * 4 + c] = to8(px[c]);
            planar[c * plane + i] = px[c];
        }
    }
    let mut doc = Document::from_rgba(name, width, height, &rgba);
    let samples = Samples::U16(planar.into());
    doc.source
        .set_native(
            Some(BitDepth::Sixteen),
            Some(SourcePlanes {
                depth: BitDepth::Sixteen,
                width,
                height,
                samples: samples.clone(),
            }),
        )
        .ok()?;
    let mut planes: Vec<(i16, Samples)> = Vec::with_capacity(4);
    for c in 0..3 {
        planes.push((c as i16, samples.slice(c * plane..(c + 1) * plane)?));
    }
    planes.push((-1, Samples::U16(vec![u16::MAX; plane].into())));
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    let layer = doc.layers.last_mut()?;
    layer.name = "Background".to_string();
    layer.source_channels = Some(SourceChannels::new(BitDepth::Sixteen, rect, planes));
    let _ = DocumentComposite::new(&mut doc).refresh(pictura_render::CompositeScope::Full, gpu);
    Some(doc)
}

fn document_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(super) fn develop_document(
    pending: PendingRaw,
    settings: &PicturaRawSettings,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Result<OpenedDocument, String> {
    progress(10, "Developing");
    let name = document_name(&pending.path);
    let PendingRaw { source, .. } = pending;
    let developed = develop_pixels(source.camera, &source.look, source.as_shot, settings)?;
    progress(75, "Preparing layers");
    let mut doc = raw_document(&name, &developed, gpu)
        .ok_or_else(|| "the developed image could not be built".to_string())?;
    let target = LayerTarget::for_opened(&doc, Some("0".to_string()));
    let history = history_for(&mut doc, &target, "Open");
    Ok(OpenedDocument {
        preview: None,
        doc,
        history,
        target,
        path: OpenedPath::Set(None),
        source_format: "psd".to_string(),
        pdf_password: None,
        compatibility: NativeCompatibility::default(),
    })
}

pub fn open_raw_start(mut view: Pin<&mut PictureView>, path: &QString, store: i32) {
    let path = path.to_string();
    let stores = RawStores::for_user(StoreKind::from_index(store));
    view.as_mut().rust_mut().pending_raw = None;
    let (budget, _) = open_budget();
    let request = view.as_mut().rust_mut().begin_open_request();
    let delay_ms = view.rust().open_delay_for_test;
    let qt_thread = view.as_ref().get_ref().qt_thread();
    let progress_thread = qt_thread.clone();
    let progress = move |percent: u8, phase: &str| {
        let phase = phase.to_string();
        progress_thread
            .queue(move |view| {
                if !view.rust().open_cancelled {
                    view.open_progress(i32::from(percent), QString::from(phase.as_str()));
                }
            })
            .ok();
    };
    std::thread::spawn(move || {
        progress(5, "Reading file");
        let decoded = decode_pending(&path, &budget, &stores, &progress);
        delay(delay_ms);
        qt_thread
            .queue(move |mut view| {
                let status = {
                    let mut rust = view.as_mut().rust_mut();
                    if rust.open_cancelled || request != rust.open_request {
                        return;
                    }
                    match decoded {
                        Ok(pending) => {
                            rust.pending_raw = Some(Box::new(pending));
                            "raw".to_string()
                        }
                        Err(reason) => format!("error:{reason}"),
                    }
                };
                view.open_finished(
                    QString::from(status.as_str()),
                    QStringList::default(),
                    0,
                    false,
                );
            })
            .ok();
    });
}

pub fn open_raw_develop_start(mut view: Pin<&mut PictureView>, settings: &QString) {
    let settings = pictura_codec::pictura_raw_from_text(&settings.to_string());
    let pending = view.as_mut().rust_mut().pending_raw.take();
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        let Some(pending) = pending else {
            return (
                None,
                "error:the raw file is no longer loaded".to_string(),
                Vec::new(),
            );
        };
        match develop_document(*pending, &settings, gpu, progress) {
            Ok(opened) => (Some(opened), "ok".to_string(), Vec::new()),
            Err(reason) => (None, format!("error:{reason}"), Vec::new()),
        }
    });
}

pub fn raw_save_settings(view: &PictureView, settings: &QString, store: i32) -> QString {
    let settings = pictura_codec::pictura_raw_from_text(&settings.to_string());
    let Some(pending) = view.rust().pending_raw.as_ref() else {
        return QString::from("error:the raw file is no longer loaded");
    };
    let stores = RawStores::for_user(StoreKind::from_index(store));
    let status = match save_settings(pending, &settings, &stores) {
        Ok(Saved::Sidecar) => "sidecar".to_string(),
        Ok(Saved::Database) => "database".to_string(),
        Ok(Saved::Fallback(reason)) => format!("fallback:{reason}"),
        Err(reason) => format!("error:{reason}"),
    };
    QString::from(status.as_str())
}

pub fn raw_seed_settings(view: &PictureView) -> QString {
    match view.rust().pending_raw.as_ref() {
        Some(pending) => QString::from(pictura_codec::pictura_raw_to_text(&pending.seed)),
        None => QString::default(),
    }
}

pub fn raw_as_shot_temperature(view: &PictureView) -> i32 {
    view.rust().pending_raw.as_ref().map_or(0, |pending| {
        pending.source.as_shot.temperature.round() as i32
    })
}

pub fn raw_as_shot_tint(view: &PictureView) -> i32 {
    view.rust()
        .pending_raw
        .as_ref()
        .map_or(0, |pending| pending.source.as_shot.tint.round() as i32)
}

pub fn raw_preview(view: &PictureView, settings: &QString, max_dim: i32) -> QImage {
    let settings = pictura_codec::pictura_raw_from_text(&settings.to_string());
    let rust = view.rust();
    let Some(pending) = rust.pending_raw.as_ref() else {
        return QImage::default();
    };
    match preview_rgba(pending, &settings, max_dim.max(1) as usize) {
        Some((rgba, width, height)) => rgba_image(rgba, width as i32, height as i32),
        None => QImage::default(),
    }
}

#[cfg(test)]
mod photo_info_tests {
    use super::*;

    /// A little-endian TIFF with IFD0 (XResolution 240/1, inches) and an Exif
    /// IFD (f/8, 1/30 s, ISO 100, 50 mm).
    fn tiff_with_exposure() -> Vec<u8> {
        let mut data = Vec::new();
        let u16v = |data: &mut Vec<u8>, v: u16| data.extend_from_slice(&v.to_le_bytes());
        let u32v = |data: &mut Vec<u8>, v: u32| data.extend_from_slice(&v.to_le_bytes());
        data.extend_from_slice(b"II");
        u16v(&mut data, 42);
        u32v(&mut data, 8);
        u16v(&mut data, 3);
        u16v(&mut data, 0x011A);
        u16v(&mut data, 5);
        u32v(&mut data, 1);
        u32v(&mut data, 104);
        u16v(&mut data, 0x0128);
        u16v(&mut data, 3);
        u32v(&mut data, 1);
        u32v(&mut data, 2);
        u16v(&mut data, 0x8769);
        u16v(&mut data, 4);
        u32v(&mut data, 1);
        u32v(&mut data, 50);
        u32v(&mut data, 0);
        u16v(&mut data, 4);
        u16v(&mut data, 0x829D);
        u16v(&mut data, 5);
        u32v(&mut data, 1);
        u32v(&mut data, 112);
        u16v(&mut data, 0x829A);
        u16v(&mut data, 5);
        u32v(&mut data, 1);
        u32v(&mut data, 120);
        u16v(&mut data, 0x8827);
        u16v(&mut data, 3);
        u32v(&mut data, 1);
        u32v(&mut data, 100);
        u16v(&mut data, 0x920A);
        u16v(&mut data, 5);
        u32v(&mut data, 1);
        u32v(&mut data, 128);
        u32v(&mut data, 0);
        u32v(&mut data, 240);
        u32v(&mut data, 1);
        u32v(&mut data, 8);
        u32v(&mut data, 1);
        u32v(&mut data, 1);
        u32v(&mut data, 30);
        u32v(&mut data, 50);
        u32v(&mut data, 1);
        data
    }

    #[test]
    fn raw_photo_info_reads_camera_exposure() {
        let info = raw_photo_info("shot.dng", &tiff_with_exposure(), 16, 12);
        assert_eq!(info.name, "shot.dng");
        assert_eq!((info.width, info.height), (16, 12));
        let row = |key: &str| {
            info.exposure
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.as_str())
        };
        assert_eq!(row("f_number"), Some("f/8"));
        assert_eq!(row("exposure"), Some("1/30 s"));
        assert_eq!(row("iso"), Some("ISO 100"));
        assert_eq!(row("focal_length"), Some("50 mm"));
        assert_eq!(info.ppi, Some(240.0));
    }

    /// A RAF header naming a camera JPEG whose Exif is `tiff`.
    fn raf_with_exif(tiff: &[u8]) -> Vec<u8> {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend(((tiff.len() + 8) as u16).to_be_bytes());
        jpeg.extend(b"Exif\0\0");
        jpeg.extend(tiff);
        jpeg.extend([0xFF, 0xD9]);
        let mut raf = b"FUJIFILMCCD-RAW 0201FF000000".to_vec();
        raf.resize(0x94, 0);
        raf[0x54..0x58].copy_from_slice(&0x94u32.to_be_bytes());
        raf[0x58..0x5C].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
        raf.extend(jpeg);
        raf
    }

    #[test]
    fn raw_photo_info_reads_a_raf_cameras_jpeg_exif() {
        let info = raw_photo_info("shot.raf", &raf_with_exif(&tiff_with_exposure()), 16, 12);
        let row = |key: &str| {
            info.exposure
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.as_str())
        };
        assert_eq!(row("f_number"), Some("f/8"));
        assert_eq!(row("iso"), Some("ISO 100"));
        assert_eq!(info.ppi, Some(240.0));
    }

    #[test]
    fn photo_info_size_is_the_developed_size() {
        let camera = CameraRgb {
            width: 40,
            height: 30,
            rgb: vec![0; 40 * 30 * 3],
        };
        let mut look = RawLook {
            color: pictura_codec::DngColor::default(),
            crop: pictura_codec::DngCrop {
                x: 4.0,
                y: 2.0,
                width: 32.0,
                height: 24.0,
            },
            orientation: 1,
        };
        assert_eq!(developed_size(&look, &camera), (32, 24));
        look.orientation = 6;
        assert_eq!(developed_size(&look, &camera), (24, 32));
    }

    #[test]
    fn raw_photo_info_omits_missing_tags() {
        let info = raw_photo_info("shot.dng", b"not a tiff", 4, 3);
        assert!(info.exposure.is_empty());
        assert!(info.ppi.is_none());
    }
}
