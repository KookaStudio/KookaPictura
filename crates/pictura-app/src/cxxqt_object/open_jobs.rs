//! Document opens off the GUI thread (#729).
//!
//! The open entry points used to read, decode, composite, and capture history on
//! the GUI thread. This module splits each open into a pure half that runs on a
//! worker and an apply half that runs on the GUI thread when the worker queues
//! its result. The worker never touches the view; cancellation is the view's
//! flag (checked in the queued closure) or the view's destruction (the queue
//! fails).
//!
//! The synchronous `open` / `open_image` / `open_as_smart_object` / `pdf_open`
//! and the import bridges share the decode helpers, so Revert, Edit Contents,
//! the self-test, and the existing tests keep their behaviour.

use super::helpers::{decode_import, topmost_pixel_layer_index};
use super::helpers_composite::{current_buffer, store_composite};
use super::impl_core::{finalize_import, format_for_path};
use super::pdf_export::import::ffi::ImportTarget;
use super::qobject::PictureView;
use super::state::PictureViewRust;
use crate::history::History;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};
use pictura_codec::pdf::import::{self as pdf_import, PdfImages, PdfRect, Rendered};
use pictura_codec::{eps, Ghostscript, Policy, Raster, RasterError, Resolution};
use pictura_core::{BitDepth, ColorMode, Document};
use std::path::Path;
use std::time::Duration;

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

        include!("pictura_app/src/cxxqt_object/pdf_export/import.cxxqt.h");
        #[namespace = "pictura"]
        type ImportTarget = super::ImportTarget;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start opening `path` as a PSD/PSB on a worker. The view's
        /// `open_finished` signal reports `"ok"` or `"error"`.
        fn open_psd_start(view: Pin<&mut PictureView>, path: &QString);

        /// Start opening `path` as a raster import on a worker.
        fn open_image_start(view: Pin<&mut PictureView>, path: &QString);

        /// Start opening `path` as a smart-object document on a worker.
        fn open_smart_object_start(view: Pin<&mut PictureView>, path: &QString);

        /// Start probing and decoding `path` as a Photoshop PDF on a worker.
        /// `open_detected` fires when it carries editing data; `open_finished`
        /// reports `"ok"`, `"none"`, `"password"`, or `"wrongpassword"`.
        fn open_pdf_start(view: Pin<&mut PictureView>, path: &QString, password: &QString);

        /// Start rendering PDF page `page` (1-based) cropped to `area` on a
        /// worker, then install it as an untitled document.
        fn open_pdf_page_start(
            view: Pin<&mut PictureView>,
            path: &QString,
            password: &QString,
            page: i32,
            area: &QString,
            target: &ImportTarget,
        );

        /// Start rendering embedded PDF image `index` on a worker.
        fn open_pdf_image_start(
            view: Pin<&mut PictureView>,
            path: &QString,
            index: i32,
            target: &ImportTarget,
        );

        /// Start rasterizing an EPS's bounding box on a worker.
        fn open_eps_start(view: Pin<&mut PictureView>, path: &QString, target: &ImportTarget);

        /// Mark a started open cancelled; the queued result is dropped.
        fn open_cancel(view: Pin<&mut PictureView>);

        /// Hold a started open's completion for `ms` milliseconds (test seam).
        fn open_set_delay(view: Pin<&mut PictureView>, ms: u64);
    }
}

/// What an open does to the view's file path.
pub(super) enum OpenedPath {
    /// Assign this path (a PSD/PDF open, or a path-less import).
    Set(Option<String>),
    /// Leave the existing path alone (a raster reload keeps its file
    /// association, so Revert can reload it again).
    Keep,
}

/// A decoded document and the view fields that belong with it.
pub(super) struct OpenedDocument {
    pub(super) doc: Document,
    pub(super) history: History,
    pub(super) active_layer: Option<String>,
    pub(super) path: OpenedPath,
    pub(super) source_format: String,
    pub(super) pdf_password: Option<String>,
}

/// A PDF probe's outcome.
pub(super) enum PdfOpen {
    Ok(Box<OpenedDocument>),
    None,
    Password,
    WrongPassword,
}

/// The import target as owned Rust data, so it can cross into a worker.
pub(super) struct ImportSpec {
    width: i32,
    height: i32,
    anti_alias: bool,
    mode: String,
    depth: i32,
    ppi: f64,
    per_cm: bool,
    name: String,
}

impl ImportSpec {
    pub(super) fn from_target(target: &ImportTarget) -> Self {
        Self {
            width: target.width,
            height: target.height,
            anti_alias: target.anti_alias,
            mode: target.mode.to_string(),
            depth: target.depth,
            ppi: target.ppi,
            per_cm: target.per_cm,
            name: target.name.to_string(),
        }
    }
}

/// Apply a decoded document to a view on the GUI thread.
pub(super) fn apply_opened(rust: &mut PictureViewRust, opened: OpenedDocument) {
    rust.doc = Some(opened.doc);
    rust.document_generation = rust.document_generation.wrapping_add(1);
    rust.reset_edit_state();
    rust.history = opened.history;
    rust.active_layer = opened.active_layer;
    match opened.path {
        OpenedPath::Set(path) => rust.path = path,
        OpenedPath::Keep => {}
    }
    rust.source_format = opened.source_format;
    rust.pdf_password = opened.pdf_password;
    rust.dirty = false;
}

fn history_for(doc: &mut Document, label: &str) -> History {
    let mut history = History::default();
    history.capture_live(doc, &None, label);
    history
}

/// Read and decode a PSD/PSB. `None` when the file is missing or unsupported.
pub(super) fn decode_psd(
    path: &str,
    policy: Policy,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    progress(5, "Reading file");
    let bytes = std::fs::read(path).ok()?;
    progress(25, "Decoding");
    let mut doc = pictura_codec::read_psd_with(&bytes, policy).ok()?;
    progress(65, "Compositing");
    let rendered = current_buffer(&doc, gpu);
    store_composite(&mut doc, &rendered);
    progress(90, "Preparing layers");
    let active_layer = topmost_pixel_layer_index(&doc).map(|i| i.to_string());
    let history = history_for(&mut doc, "Open");
    Some(OpenedDocument {
        doc,
        history,
        active_layer,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: "psd".to_string(),
        pdf_password: None,
    })
}

/// Read and decode a raster through the shared import path.
pub(super) fn decode_image(
    path: &str,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    let name = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    progress(5, "Reading file");
    let bytes = std::fs::read(path).ok()?;
    progress(25, "Decoding");
    let (rgba, width, height) = decode_import(&bytes)?;
    progress(70, "Compositing");
    let mut doc = Document::from_rgba(&name, width, height, &rgba);
    finalize_import(&mut doc, &rgba);
    let rendered = current_buffer(&doc, gpu);
    store_composite(&mut doc, &rendered);
    progress(90, "Preparing layers");
    let history = history_for(&mut doc, "Open");
    Some(OpenedDocument {
        doc,
        history,
        active_layer: Some("0".to_string()),
        path: OpenedPath::Keep,
        source_format: format_for_path(path),
        pdf_password: None,
    })
}

/// Read and embed `path` as a smart-object document.
pub(super) fn decode_smart_object(
    path: &str,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    let name = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    progress(5, "Reading file");
    let bytes = std::fs::read(path).ok();
    progress(35, "Decoding");
    let mut doc = match bytes
        .as_deref()
        .and_then(|bytes| pictura_render::open_as_smart_object(&name, bytes))
    {
        Some(doc) => doc,
        None => {
            let (rgba, width, height) = bytes.as_deref().and_then(decode_import)?;
            let mut doc = Document::from_rgba(&name, width, height, &rgba);
            if !pictura_render::convert_to_smart_object(&mut doc, "0") {
                return None;
            }
            doc
        }
    };
    progress(70, "Compositing");
    let rendered = current_buffer(&doc, gpu);
    store_composite(&mut doc, &rendered);
    progress(90, "Preparing layers");
    let history = history_for(&mut doc, "Open As Smart Object");
    Some(OpenedDocument {
        doc,
        history,
        active_layer: Some("0".to_string()),
        path: OpenedPath::Set(None),
        source_format: "psd".to_string(),
        pdf_password: None,
    })
}

fn decode_pdf_psd(
    psd: &[u8],
    path: &str,
    password: &str,
    policy: Policy,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    progress(35, "Decoding");
    let mut doc = pictura_codec::read_psd_with(psd, policy).ok()?;
    progress(65, "Compositing");
    let rendered = current_buffer(&doc, gpu);
    store_composite(&mut doc, &rendered);
    progress(90, "Preparing layers");
    let active_layer = topmost_pixel_layer_index(&doc).map(|i| i.to_string());
    let history = history_for(&mut doc, "Open");
    Some(OpenedDocument {
        doc,
        history,
        active_layer,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: "pdf".to_string(),
        pdf_password: (!password.is_empty()).then(|| password.to_string()),
    })
}

/// Probe a Photoshop PDF and decode its editing data. A decode failure reads as
/// `None`, so the caller falls back to the import dialog, as the synchronous
/// `pdf_open` did.
pub(super) fn decode_pdf(path: &str, password: &str, policy: Policy, gpu: bool) -> PdfOpen {
    let bytes = std::fs::read(path).unwrap_or_default();
    match pictura_codec::pdf::editing_psd(&bytes, password) {
        pictura_codec::pdf::EditingPsd::Psd(psd) => {
            match decode_pdf_psd(&psd, path, password, policy, gpu, &|_, _| {}) {
                Some(opened) => PdfOpen::Ok(Box::new(opened)),
                None => PdfOpen::None,
            }
        }
        pictura_codec::pdf::EditingPsd::Absent => PdfOpen::None,
        pictura_codec::pdf::EditingPsd::Locked if password.is_empty() => PdfOpen::Password,
        pictura_codec::pdf::EditingPsd::Locked => PdfOpen::WrongPassword,
    }
}

/// Render PDF page `page` (1-based) cropped to `area`.
pub(super) fn render_pdf_page(
    path: &str,
    password: &str,
    page: i32,
    area: &str,
    spec: &ImportSpec,
) -> Result<Rendered, RasterError> {
    let numbers: Vec<f64> = area
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    let [x0, y0, x1, y1, view_height] = numbers[..] else {
        return Err(RasterError::Failed("bad page area".into()));
    };
    Ghostscript::locate().and_then(|gs| {
        pdf_import::render_area(
            &gs,
            Path::new(path),
            (!password.is_empty()).then_some(password),
            page.max(1) as u32,
            PdfRect::new(x0, y0, x1, y1),
            view_height,
            (spec.width.max(1) as u32, spec.height.max(1) as u32),
            spec.anti_alias,
        )
    })
}

/// Render embedded PDF image `index` at its native size.
pub(super) fn render_pdf_image(
    path: &str,
    index: i32,
    _spec: &ImportSpec,
) -> Result<Rendered, RasterError> {
    Ghostscript::locate().and_then(|gs| {
        let found = std::fs::read(path)
            .ok()
            .as_deref()
            .and_then(PdfImages::read)
            .ok_or_else(|| RasterError::Failed("no readable images".into()))?;
        found.render(&gs, index.max(0) as usize, None)
    })
}

/// Rasterize an EPS's bounding box.
pub(super) fn render_eps(path: &str, spec: &ImportSpec) -> Result<Rendered, RasterError> {
    Ghostscript::locate().and_then(|gs| {
        let bounds = std::fs::read(path)
            .ok()
            .as_deref()
            .and_then(eps::bounding_box)
            .ok_or_else(|| RasterError::Failed("no bounding box".into()))?;
        eps::render_eps(
            &gs,
            Path::new(path),
            bounds,
            (spec.width.max(1) as u32, spec.height.max(1) as u32),
            spec.anti_alias,
        )
    })
}

/// Build the untitled document an import's raster becomes.
pub(super) fn imported_document(
    raster: &Raster,
    spec: &ImportSpec,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    progress(80, "Preparing layers");
    let mut doc = Document::from_rgba(&spec.name, raster.width, raster.height, &raster.rgba);
    if let Some(layer) = doc.layers.last_mut() {
        layer.name = "Layer 1".to_string();
    }
    pictura_codec::set_document_resolution(
        &mut doc,
        Resolution {
            ppi: if spec.ppi > 0.0 { spec.ppi } else { 72.0 },
            per_cm: spec.per_cm,
        },
    );
    let mode = match spec.mode.as_str() {
        "grayscale" => ColorMode::Grayscale,
        "cmyk" => ColorMode::Cmyk,
        "lab" => ColorMode::Lab,
        _ => ColorMode::Rgb,
    };
    if mode != ColorMode::Rgb {
        pictura_render::convert_mode(&mut doc, mode).ok()?;
    }
    if spec.depth == 16 {
        pictura_render::convert_bit_depth(&mut doc, BitDepth::Sixteen).ok()?;
    }
    progress(90, "Compositing");
    let rendered = current_buffer(&doc, gpu);
    store_composite(&mut doc, &rendered);
    progress(95, "Preparing layers");
    let history = history_for(&mut doc, "Open");
    Some(OpenedDocument {
        doc,
        history,
        active_layer: Some("0".to_string()),
        path: OpenedPath::Set(None),
        source_format: "psd".to_string(),
        pdf_password: None,
    })
}

fn warnings_list(warnings: Vec<String>) -> QStringList {
    let mut list = QStringList::default();
    for warning in warnings {
        list.append(QString::from(warning.as_str()));
    }
    list
}

/// Apply a result unless the view was cancelled. True when it was applied.
fn apply_if_live(rust: &mut PictureViewRust, result: Option<OpenedDocument>) -> bool {
    if rust.open_cancelled {
        return false;
    }
    if let Some(opened) = result {
        apply_opened(rust, opened);
    }
    true
}

/// Queue the worker's result onto the GUI thread. A cancelled or destroyed
/// view drops it.
fn finish_on_gui(
    qt_thread: CxxQtThread<PictureView>,
    result: Option<OpenedDocument>,
    status: String,
    warnings: Vec<String>,
) {
    qt_thread
        .queue(move |mut view| {
            {
                let mut rust = view.as_mut().rust_mut();
                if !apply_if_live(&mut rust, result) {
                    return;
                }
            }
            view.open_finished(QString::from(status.as_str()), warnings_list(warnings));
        })
        .ok();
}

fn delay(ms: u64) {
    if ms > 0 {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

fn error_status(error: &RasterError) -> String {
    format!("error:{error}")
}

fn start(
    view: &mut Pin<&mut PictureView>,
    work: impl FnOnce(&dyn Fn(u8, &str)) -> (Option<OpenedDocument>, String, Vec<String>)
        + Send
        + 'static,
) {
    view.as_mut().rust_mut().open_cancelled = false;
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
        let (result, status, warnings) = work(&progress);
        delay(delay_ms);
        finish_on_gui(qt_thread, result, status, warnings);
    });
}

pub fn open_psd_start(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
    let policy = view.rust().color_policy;
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        match decode_psd(&path, policy, gpu, progress) {
            Some(opened) => (Some(opened), "ok".to_string(), Vec::new()),
            None => (None, "error".to_string(), Vec::new()),
        }
    });
}

pub fn open_image_start(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        match decode_image(&path, gpu, progress) {
            Some(opened) => (Some(opened), "ok".to_string(), Vec::new()),
            None => (None, "error".to_string(), Vec::new()),
        }
    });
}

pub fn open_smart_object_start(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        match decode_smart_object(&path, gpu, progress) {
            Some(opened) => (Some(opened), "ok".to_string(), Vec::new()),
            None => (None, "error".to_string(), Vec::new()),
        }
    });
}

pub fn open_pdf_start(mut view: Pin<&mut PictureView>, path: &QString, password: &QString) {
    let path = path.to_string();
    let password = password.to_string();
    let policy = view.rust().color_policy;
    let gpu = view.rust().gpu_compute;
    view.as_mut().rust_mut().open_cancelled = false;
    let delay_ms = view.rust().open_delay_for_test;
    let qt_thread = view.as_ref().get_ref().qt_thread();
    let detected = qt_thread.clone();
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
        let bytes = std::fs::read(&path).unwrap_or_default();
        progress(15, "Checking PDF");
        match pictura_codec::pdf::editing_psd(&bytes, &password) {
            pictura_codec::pdf::EditingPsd::Psd(psd) => {
                detected
                    .queue(|view| {
                        if !view.rust().open_cancelled {
                            view.open_detected();
                        }
                    })
                    .ok();
                let opened = decode_pdf_psd(&psd, &path, &password, policy, gpu, &progress);
                let status = if opened.is_some() {
                    "ok".to_string()
                } else {
                    "none".to_string()
                };
                delay(delay_ms);
                finish_on_gui(qt_thread, opened, status, Vec::new());
            }
            pictura_codec::pdf::EditingPsd::Absent => {
                delay(delay_ms);
                finish_on_gui(qt_thread, None, "none".to_string(), Vec::new());
            }
            pictura_codec::pdf::EditingPsd::Locked if password.is_empty() => {
                delay(delay_ms);
                finish_on_gui(qt_thread, None, "password".to_string(), Vec::new());
            }
            pictura_codec::pdf::EditingPsd::Locked => {
                delay(delay_ms);
                finish_on_gui(qt_thread, None, "wrongpassword".to_string(), Vec::new());
            }
        }
    });
}

pub fn open_pdf_page_start(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    password: &QString,
    page: i32,
    area: &QString,
    target: &ImportTarget,
) {
    let path = path.to_string();
    let password = password.to_string();
    let area = area.to_string();
    let spec = ImportSpec::from_target(target);
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        progress(10, "Rendering page");
        match render_pdf_page(&path, &password, page, &area, &spec) {
            Ok(rendered) => match imported_document(&rendered.raster, &spec, gpu, progress) {
                Some(opened) => (Some(opened), "ok".to_string(), rendered.warnings),
                None => (
                    None,
                    "error:the image could not be converted".to_string(),
                    Vec::new(),
                ),
            },
            Err(error) => (None, error_status(&error), Vec::new()),
        }
    });
}

pub fn open_pdf_image_start(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    index: i32,
    target: &ImportTarget,
) {
    let path = path.to_string();
    let spec = ImportSpec::from_target(target);
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        progress(10, "Rendering image");
        match render_pdf_image(&path, index, &spec) {
            Ok(rendered) => match imported_document(&rendered.raster, &spec, gpu, progress) {
                Some(opened) => (Some(opened), "ok".to_string(), rendered.warnings),
                None => (
                    None,
                    "error:the image could not be converted".to_string(),
                    Vec::new(),
                ),
            },
            Err(error) => (None, error_status(&error), Vec::new()),
        }
    });
}

pub fn open_eps_start(mut view: Pin<&mut PictureView>, path: &QString, target: &ImportTarget) {
    let path = path.to_string();
    let spec = ImportSpec::from_target(target);
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        progress(10, "Rendering EPS");
        match render_eps(&path, &spec) {
            Ok(rendered) => match imported_document(&rendered.raster, &spec, gpu, progress) {
                Some(opened) => (Some(opened), "ok".to_string(), rendered.warnings),
                None => (
                    None,
                    "error:the image could not be converted".to_string(),
                    Vec::new(),
                ),
            },
            Err(error) => (None, error_status(&error), Vec::new()),
        }
    });
}

pub fn open_cancel(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().open_cancelled = true;
}

pub fn open_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().open_delay_for_test = ms;
}

#[cfg(test)]
mod tests;
