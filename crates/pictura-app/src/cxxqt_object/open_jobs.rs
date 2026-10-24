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

use super::helpers::decode_import;
use super::helpers_composite::{current_buffer, store_composite};
use super::impl_core::format_for_path;
use super::pdf_export::import::ffi::ImportTarget;
use super::qobject::PictureView;
use super::state::PictureViewRust;
use crate::history::History;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};
use pictura_codec::pdf::import::{self as pdf_import, PdfImages, PdfRect, Rendered};
use pictura_codec::{eps, Ghostscript, Policy, Raster, RasterError, Resolution};
use pictura_core::{BitDepth, ColorMode, Document, Layer, LockFlags};
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

        /// `File > Revert`: reload `path` in place, preserving history and
        /// named snapshots and appending one `Revert` state; `false` without
        /// mutating on any decode failure.
        fn revert(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}

/// What an open does to the view's file path.
pub(super) enum OpenedPath {
    /// Assign this path (a document open, or a path-less import).
    Set(Option<String>),
}

/// A decoded document and the view fields that belong with it.
pub(super) struct OpenedDocument {
    pub(super) doc: Document,
    pub(super) history: History,
    pub(super) target: LayerTarget,
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
    rust.replace_document(
        Some(opened.doc),
        opened.history,
        opened.target,
        opened.pdf_password,
    );
    let OpenedPath::Set(path) = opened.path;
    rust.path = path;
    rust.source_format = opened.source_format;
    rust.session.mark_saved();
}

/// `File > Revert`: the synchronous in-place reload.
fn revert(view: Pin<&mut PictureView>, path: &QString) -> bool {
    view.revert(path)
}

fn history_for(doc: &mut Document, target: &LayerTarget, label: &str) -> History {
    let mut history = History::default();
    history.capture_live(doc, &None, target, label);
    history
}

/// Cache the displayed composite after a decode, unless the file retained
/// native samples and a merged composite. A retained store is the conversion
/// source and keeps values above white; leaving the file's merged composite
/// (the store's 8-bit narrowing) in place lets `convert_depth` see that store
/// as current. A file without a merged composite still recomposites here.
fn store_opened_composite(doc: &mut Document, gpu: bool) {
    if doc.source_planes.is_some() && doc.merged_composite_present {
        return;
    }
    let rendered = current_buffer(doc, gpu);
    store_composite(doc, &rendered);
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
    store_opened_composite(&mut doc, gpu);
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, None);
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
        doc,
        history,
        target,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: "psd".to_string(),
        pdf_password: None,
    })
}

/// Finish an imported raster document. When every decoded pixel is opaque the
/// single `from_rgba` layer becomes the locked `Background` and its redundant
/// alpha channel is dropped; otherwise the regular alpha layer named from the
/// file stem is left untouched.
pub(super) fn finalize_import(doc: &mut Document, rgba: &[u8]) {
    if !rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 255) {
        return;
    }
    if let Some(layer) = doc.layers.last_mut() {
        make_background(layer);
    }
}

/// Turn the opaque `layer` into the locked `Background`, dropping its
/// redundant alpha channel.
pub(super) fn make_background(layer: &mut Layer) {
    layer.name = "Background".to_string();
    layer.background = true;
    layer.lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::POSITION, true);
    layer.channels.retain(|channel| channel.id != -1);
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
    let bytes =
        super::helpers::safe_io::read_capped(path, super::helpers::safe_io::import_file_limit())?;
    progress(25, "Decoding");
    let (rgba, width, height) = decode_import(&bytes)?;
    progress(70, "Compositing");
    let mut doc = Document::from_rgba(&name, width, height, &rgba);
    finalize_import(&mut doc, &rgba);
    if let Some(section) = pictura_codec::raster_image_resources(&bytes) {
        doc.image_resources = section;
    }
    store_opened_composite(&mut doc, gpu);
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, Some("0".to_string()));
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
        doc,
        history,
        target,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: format_for_path(path),
        pdf_password: None,
    })
}

/// Decode `path` for an in-place Revert without touching the view. A PDF uses
/// the stored password; a missing, corrupt, or unreadable file returns `None`.
pub(super) fn decode_for_revert(
    path: &str,
    pdf_password: Option<&str>,
    policy: Policy,
    gpu: bool,
) -> Option<OpenedDocument> {
    // Match the old C++ routing by the file's own extension: a path without a
    // known extension stays a raster import, not the PSD default used by Save.
    let suffix = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match suffix.as_deref() {
        Some("psd" | "psb") => decode_psd(path, policy, gpu, &|_, _| {}),
        Some("pdf" | "pdp") => match decode_pdf(path, pdf_password.unwrap_or(""), policy, gpu) {
            PdfOpen::Ok(opened) => Some(*opened),
            PdfOpen::None | PdfOpen::Password | PdfOpen::WrongPassword => None,
        },
        _ => decode_image(path, gpu, &|_, _| {}),
    }
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
    store_opened_composite(&mut doc, gpu);
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, Some("0".to_string()));
    let history = history_for(&mut doc, &target, "Open As Smart Object");
    Some(OpenedDocument {
        doc,
        history,
        target,
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
    store_opened_composite(&mut doc, gpu);
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, None);
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
        doc,
        history,
        target,
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
    store_opened_composite(&mut doc, gpu);
    progress(95, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, Some("0".to_string()));
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
        doc,
        history,
        target,
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

impl PictureViewRust {
    /// Start an open request and return its id. A result from an earlier
    /// request is stale and must not install.
    pub(super) fn begin_open_request(&mut self) -> u64 {
        self.open_cancelled = false;
        self.open_request = self.open_request.wrapping_add(1);
        self.open_request
    }
}

/// Apply a result unless the view was cancelled or the request is stale. True
/// when it was applied.
fn apply_if_current(
    rust: &mut PictureViewRust,
    request: u64,
    result: Option<OpenedDocument>,
) -> bool {
    if rust.open_cancelled || request != rust.open_request {
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
    request: u64,
    result: Option<OpenedDocument>,
    status: String,
    warnings: Vec<String>,
) {
    qt_thread
        .queue(move |mut view| {
            {
                let mut rust = view.as_mut().rust_mut();
                if !apply_if_current(&mut rust, request, result) {
                    return;
                }
            }
            view.as_mut().session_cancelled();
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
        let (result, status, warnings) = work(&progress);
        delay(delay_ms);
        finish_on_gui(qt_thread, request, result, status, warnings);
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
    let request = view.as_mut().rust_mut().begin_open_request();
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
                finish_on_gui(qt_thread, request, opened, status, Vec::new());
            }
            pictura_codec::pdf::EditingPsd::Absent => {
                delay(delay_ms);
                finish_on_gui(qt_thread, request, None, "none".to_string(), Vec::new());
            }
            pictura_codec::pdf::EditingPsd::Locked if password.is_empty() => {
                delay(delay_ms);
                finish_on_gui(qt_thread, request, None, "password".to_string(), Vec::new());
            }
            pictura_codec::pdf::EditingPsd::Locked => {
                delay(delay_ms);
                finish_on_gui(
                    qt_thread,
                    request,
                    None,
                    "wrongpassword".to_string(),
                    Vec::new(),
                );
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
