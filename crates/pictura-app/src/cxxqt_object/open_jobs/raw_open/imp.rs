use super::super::compatibility::NativeCompatibility;
use super::super::imp::{delay, history_for, open_budget, start, OpenedDocument, OpenedPath};
use super::PictureView;
use crate::cxxqt_object::helpers::safe_io;
use crate::cxxqt_object::helpers_composite::rgba_image;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_codec::{
    CameraRgb, DevelopedRgb, ImageBudget, PsdError, RawLook, RawSource, WhiteBalance,
};
use pictura_core::{
    BitDepth, CrsSettings, Document, PicturaRawSettings, PsdRect, Samples, SourceChannels,
    SourcePlanes,
};
use pictura_render::DocumentComposite;
use std::io::Read;
use std::path::Path;

/// The prefix read to recognise a raw container.
const SNIFF_BYTES: u64 = 256 * 1024;
/// The long side of the preview copy kept beside the full source.
const PREVIEW_EDGE: usize = 1024;
/// A sidecar larger than this is not read.
const SIDECAR_LIMIT: u64 = 4 * 1024 * 1024;
const MIN_TEMPERATURE: f64 = 2000.0;
const MAX_TEMPERATURE: f64 = 50000.0;
const TINT_REACH: f64 = 150.0;

/// A decoded raw file waiting for the dialog's settings.
pub(in crate::cxxqt_object) struct PendingRaw {
    pub(super) path: String,
    pub(super) source: RawSource,
    pub(super) preview: CameraRgb,
    pub(super) preview_look: RawLook,
    pub(super) seed: PicturaRawSettings,
}

pub fn raw_sniff_path(path: &QString) -> bool {
    let Ok(file) = std::fs::File::open(path.to_string()) else {
        return false;
    };
    let mut prefix = Vec::new();
    file.take(SNIFF_BYTES).read_to_end(&mut prefix).is_ok()
        && pictura_codec::sniff_raw_prefix(&prefix)
}

fn reason(error: PsdError) -> String {
    match error {
        PsdError::Unsupported(text) | PsdError::Invalid(text) => text,
        other => other.to_string(),
    }
}

/// Read and decode `path`, keeping a preview copy and the seed settings.
pub(super) fn decode_pending(
    path: &str,
    budget: &ImageBudget,
    progress: &dyn Fn(u8, &str),
) -> Result<PendingRaw, String> {
    let bytes = safe_io::read_capped(path, safe_io::import_file_limit())
        .ok_or_else(|| "the file could not be read".to_string())?;
    progress(25, "Decoding raw data");
    let source = pictura_codec::decode_raw_source(&bytes, budget).map_err(reason)?;
    progress(85, "Preparing preview");
    let preview = pictura_codec::downscale_camera(&source.camera, PREVIEW_EDGE);
    let preview_look = scaled_look(&source.look, &source.camera, &preview);
    let seed = seed_settings(Path::new(path), &bytes, source.as_shot);
    Ok(PendingRaw {
        path: path.to_string(),
        source,
        preview,
        preview_look,
        seed,
    })
}

fn scaled_look(look: &RawLook, full: &CameraRgb, copy: &CameraRgb) -> RawLook {
    look.scaled(
        copy.width as f64 / full.width.max(1) as f64,
        copy.height as f64 / full.height.max(1) as f64,
    )
}

/// The as-shot white balance, then any `crs:` Basic control the sidecar
/// `<stem>.xmp` sets, or, without a sidecar, the file's embedded packet.
pub(super) fn seed_settings(
    path: &Path,
    bytes: &[u8],
    as_shot: WhiteBalance,
) -> PicturaRawSettings {
    let mut basic = CrsSettings {
        temperature: Some(as_shot.temperature.round()),
        tint: Some(as_shot.tint.round()),
        ..CrsSettings::default()
    };
    let packet = safe_io::read_capped(path.with_extension("xmp"), SIDECAR_LIMIT)
        .or_else(|| pictura_codec::raw_xmp(bytes));
    if let Some(packet) = packet {
        let found = pictura_codec::crs_settings_from_xmp(&packet);
        let fields = [
            (&mut basic.exposure, found.exposure),
            (&mut basic.contrast, found.contrast),
            (&mut basic.highlights, found.highlights),
            (&mut basic.shadows, found.shadows),
            (&mut basic.whites, found.whites),
            (&mut basic.blacks, found.blacks),
            (&mut basic.clarity, found.clarity),
            (&mut basic.vibrance, found.vibrance),
            (&mut basic.saturation, found.saturation),
            (&mut basic.temperature, found.temperature),
            (&mut basic.tint, found.tint),
        ];
        for (slot, value) in fields {
            if value.is_some() {
                *slot = value;
            }
        }
    }
    PicturaRawSettings {
        basic,
        ..PicturaRawSettings::default()
    }
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

fn to8(v: u16) -> u8 {
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

pub fn open_raw_start(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
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
        let decoded = decode_pending(&path, &budget, &progress);
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
                view.open_finished(QString::from(status.as_str()), QStringList::default());
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
