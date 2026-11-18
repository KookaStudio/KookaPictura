use super::*;

use super::super::helpers::decode_import;
use super::super::impl_core::format_for_path;
use super::super::state::PictureViewRust;
use crate::history::History;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};
use pictura_codec::pdf::import::{self as pdf_import, NativeRender, PdfImages, PdfRect, Rendered};
use pictura_codec::{eps, Ghostscript, Policy, Raster, RasterError, Resolution};
use pictura_core::{
    BitDepth, ColorMode, Document, Layer, LockFlags, PsdRect, Samples, SourceChannels, SourcePlanes,
};
use pictura_render::DocumentComposite;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::compatibility::{native_access, source_pixels, NativeCompatibility};

pub(super) fn output_path_protected(path: &QString) -> bool {
    super::compatibility::destination_is_protected(Path::new(&path.to_string()))
}

/// What an open does to the view's file path.
pub(in crate::cxxqt_object) enum OpenedPath {
    /// Assign this path (a document open, or a path-less import).
    Set(Option<String>),
}

/// A decoded document and the view fields that belong with it.
pub(in crate::cxxqt_object) struct OpenedDocument {
    pub(in crate::cxxqt_object) doc: Document,
    pub(in crate::cxxqt_object) history: History,
    pub(in crate::cxxqt_object) target: LayerTarget,
    pub(in crate::cxxqt_object) path: OpenedPath,
    pub(in crate::cxxqt_object) source_format: String,
    pub(in crate::cxxqt_object) pdf_password: Option<String>,
    /// The file's merged image, shown until the first edit (#1218).
    pub(in crate::cxxqt_object) preview: Option<pictura_codec::MergedPreview>,
    pub(in crate::cxxqt_object) compatibility: NativeCompatibility,
}

/// A PDF probe's outcome.
pub(in crate::cxxqt_object) enum PdfOpen {
    Ok(Box<OpenedDocument>),
    None,
    Password,
    WrongPassword,
}

/// What an import render produces: the legacy RGBA8 raster or a native render.
pub(in crate::cxxqt_object) enum Imported {
    Rgba(Rendered),
    Native(NativeRender),
}

/// The import target as owned Rust data, so it can cross into a worker.
pub(in crate::cxxqt_object) struct ImportSpec {
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
    pub(in crate::cxxqt_object) fn from_target(target: &ImportTarget) -> Self {
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

    fn color_mode(&self) -> ColorMode {
        match self.mode.as_str() {
            "grayscale" => ColorMode::Grayscale,
            "cmyk" => ColorMode::Cmyk,
            "lab" => ColorMode::Lab,
            _ => ColorMode::Rgb,
        }
    }

    fn bit_depth(&self) -> BitDepth {
        if self.depth >= 16 {
            BitDepth::Sixteen
        } else {
            BitDepth::Eight
        }
    }

    fn wants_native(&self) -> bool {
        self.color_mode() == ColorMode::Cmyk || self.bit_depth() == BitDepth::Sixteen
    }
}

/// Apply a decoded document to a view on the GUI thread.
pub(in crate::cxxqt_object) fn apply_opened(rust: &mut PictureViewRust, opened: OpenedDocument) {
    rust.replace_document(
        Some(opened.doc),
        opened.history,
        opened.target,
        opened.pdf_password,
    );
    // After the replace, whose display reset drops any preview.
    *rust.tiles.preview.get_mut() = opened.preview;
    rust.session.set_compatibility(opened.compatibility);
    let OpenedPath::Set(path) = opened.path;
    rust.path = path;
    rust.source_format = opened.source_format;
    rust.session.mark_saved();
}

/// `File > Revert`: the synchronous in-place reload.
pub(super) fn revert(view: Pin<&mut PictureView>, path: &QString) -> bool {
    view.revert(path)
}

pub(super) fn history_for(doc: &mut Document, target: &LayerTarget, label: &str) -> History {
    let mut history = History::default();
    history.capture_live(doc, &None, target, label);
    history
}

/// Bring the stored composite up to the layers after a decode, unless the
/// file retained native samples and a merged composite. A retained store is
/// the conversion source and keeps values above white; leaving the file's
/// merged composite (the store's 8-bit narrowing) in place lets
/// `convert_depth` see that store as current. A layered RGB document keeps no
/// composite: the file's merged image is released and the display tiles
/// composite what is shown (#1216).
pub(super) fn store_opened_composite(doc: &mut Document, gpu: bool) {
    let started = std::time::Instant::now();
    if doc.source.planes().is_some() && doc.composite.merged_output() {
        crate::cxxqt_object::helpers::paint_timing::record(
            "opening_composite_skipped",
            started.elapsed(),
        );
        return;
    }
    DocumentComposite::new(doc).refresh(pictura_render::CompositeScope::Full, gpu);
    crate::cxxqt_object::helpers::paint_timing::record("opening_composite", started.elapsed());
}

/// The application's default open options. The working CMYK profile is the
/// bundled synthetic coated profile until Color Settings carries a choice.
pub(super) fn open_options(policy: Policy) -> pictura_codec::OpenOptions {
    pictura_codec::OpenOptions {
        policy,
        working_cmyk: Some(pictura_codec::Profile::coated_cmyk().to_icc()),
        conversion: pictura_codec::ConversionOptions::default(),
        skip_layered_merged: true,
        source: None,
    }
}

/// Read and decode a PSD/PSB. `None` when the file is missing or unsupported.
pub(in crate::cxxqt_object) fn decode_psd(
    path: &str,
    policy: Policy,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    decode_psd_with(path, open_options(policy), open_budget(), gpu, progress).ok()
}

/// Whether the last PSD open was refused because its decode did not fit the
/// memory governor's room. Set on the open's worker, read on the GUI thread.
static OPEN_REFUSED_FOR_MEMORY: AtomicBool = AtomicBool::new(false);

pub(in crate::cxxqt_object) fn open_refused_for_memory() -> bool {
    OPEN_REFUSED_FOR_MEMORY.load(Ordering::Relaxed)
}

/// The decode budget of an open, read on the calling (GUI) thread, and
/// whether the governor's room bounded it. Contiguous allocations get two
/// thirds of the room (the rest is the composite, history and display the open
/// builds after the decode, #1211). The layer tiles get that plus the scratch
/// disks' free space, since the evictor can pack and spill them (#1273).
pub(in crate::cxxqt_object) fn open_budget() -> (pictura_codec::ImageBudget, bool) {
    let default = pictura_codec::ImageBudget::default();
    let Some(room) = pictura_core::memory::governor()
        .room()
        .map(|room| room / 3 * 2)
    else {
        return (default, false);
    };
    let mut budget = super::super::helpers::bounded_budget(default, Some(room));
    budget.max_tiled_bytes =
        room.saturating_add(super::super::helpers::memory_gate::scratch_available());
    (budget, true)
}

pub(super) fn decode_psd_with(
    path: &str,
    mut options: pictura_codec::OpenOptions,
    (budget, bounded): (pictura_codec::ImageBudget, bool),
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Result<OpenedDocument, pictura_codec::PsdError> {
    OPEN_REFUSED_FOR_MEMORY.store(false, Ordering::Relaxed);
    progress(5, "Reading file");
    // The file stays open behind the document: its 8-bit raw and RLE layer
    // channels are read from it on demand (#1218). Without a pin (no
    // positioned reads on this platform) the open decodes everything.
    let pin = pictura_codec::SourcePin::open(path).ok();
    let bytes = match pin.as_ref().map(|pin| pin.map()) {
        Some(mapped) => mapped,
        None => pictura_codec::MappedFile::open(path),
    }
    .map_err(|error| pictura_codec::PsdError::Invalid(error.to_string()))?;
    options.source = pin.clone();
    progress(25, "Decoding");
    let (mut doc, preview) = match pictura_codec::read_psd_lazy(&bytes, &options, budget) {
        Ok(read) => read,
        Err(error) => {
            // The codec names a budget refusal "… exceeds the decode budget".
            let memory = bounded && error.to_string().contains("decode budget");
            OPEN_REFUSED_FOR_MEMORY.store(memory, Ordering::Relaxed);
            if memory
                || matches!(
                    error,
                    pictura_codec::PsdError::NativeVersion(_)
                        | pictura_codec::PsdError::BadSignature(_)
                )
            {
                return Err(error);
            }
            let Some(doc) = pictura_codec::read_psd_layer_structure(&bytes, &options, budget)
                .ok()
                .filter(|doc| {
                    native_access(doc, &pictura_codec::document_version(doc)).is_read_only()
                })
            else {
                return Err(error);
            };
            (doc, None)
        }
    };
    let version = pictura_codec::document_version(&doc);
    let access = native_access(&doc, &version);
    let compatibility = if access.is_read_only() {
        let has_real_merge = version
            .version_info
            .as_ref()
            .is_some_and(|info| info.has_real_merged_data)
            || version.version_info.is_none() && doc.layers.is_empty();
        let merged_alpha = pictura_codec::native_merged_has_transparency(&bytes, doc.is_psb);
        let pixels = if !has_real_merge || !doc.composite.merged_output() {
            None
        } else if let Some(pixels) = source_pixels(&doc, merged_alpha) {
            Some(pixels)
        } else {
            options.skip_layered_merged = false;
            pictura_codec::read_psd_within_options(&bytes, &options, budget)
                .ok()
                .and_then(|source| source_pixels(&source, merged_alpha))
        };
        NativeCompatibility::new(access, Path::new(path), pin, pixels)
    } else {
        NativeCompatibility::new(access, Path::new(path), None, None)
    };
    progress(65, "Compositing");
    if !compatibility.access.is_read_only() {
        store_opened_composite(&mut doc, gpu);
    }
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, None);
    let history = history_for(&mut doc, &target, "Open");
    Ok(OpenedDocument {
        preview,
        doc,
        history,
        target,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: "psd".to_string(),
        pdf_password: None,
        compatibility,
    })
}

/// Finish an imported raster document. When every decoded pixel is opaque the
/// single `from_rgba` layer becomes the locked `Background` and its redundant
/// alpha channel is dropped; otherwise the regular alpha layer named from the
/// file stem is left untouched.
pub(in crate::cxxqt_object) fn finalize_import(doc: &mut Document, rgba: &[u8]) {
    if !rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 255) {
        return;
    }
    if let Some(layer) = doc.layers.last_mut() {
        make_background(layer);
    }
}

/// Turn the opaque `layer` into the locked `Background`, dropping its
/// redundant alpha channel.
pub(in crate::cxxqt_object) fn make_background(layer: &mut Layer) {
    layer.name = "Background".to_string();
    layer.background = true;
    layer.lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::POSITION, true);
    layer.channels_mut().retain(|channel| channel.id != -1);
}

/// Apply a raster's embedded RGB ICC profile under the incoming policy. A
/// CMYK profile is not applied to the decoded RGB bytes; Qt has already
/// produced RGB values and the profile cannot describe them.
pub(super) fn apply_raster_profile(
    doc: &mut Document,
    bytes: &[u8],
    policy: Policy,
    options: pictura_codec::ConversionOptions,
) {
    let probe = pictura_codec::probe_raster_profile(bytes);
    let Some(icc) = probe.embedded else {
        return;
    };
    if probe.is_srgb || probe.channels != Some(3) {
        return;
    }
    match policy {
        Policy::Preserve => doc.source.set_working_profile(Some(icc.into())),
        Policy::Convert => {
            doc.source.set_working_profile(Some(icc.clone().into()));
            let _ =
                pictura_codec::convert_document_with(doc, &pictura_codec::Profile::srgb(), options);
            doc.source.set_original_profile(Some(icc.into()));
        }
        Policy::Off => {}
    }
}

/// Convert a placed raster's decoded pixels to the host sRGB working space
/// when it carries a non-sRGB RGB ICC profile. Place always converts, so the
/// resulting layer is in the document's working space.
pub(in crate::cxxqt_object) fn convert_placed_rgba(
    bytes: &[u8],
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    policy: Policy,
) -> Vec<u8> {
    if policy == Policy::Off {
        return rgba;
    }
    let probe = pictura_codec::probe_raster_profile(bytes);
    let Some(icc) = probe.embedded else {
        return rgba;
    };
    if probe.is_srgb || probe.channels != Some(3) {
        return rgba;
    }
    let mut doc = Document::from_rgba("place", width, height, &rgba);
    doc.source.set_working_profile(Some(icc.into()));
    if !pictura_codec::convert_document_with(
        &mut doc,
        &pictura_codec::Profile::srgb(),
        pictura_codec::ConversionOptions::default(),
    ) {
        return rgba;
    }
    super::super::helpers_composite::buffer_to_rgba_bytes(doc.composite.buffer())
}

/// Read and decode a raster through the shared import path.
pub(in crate::cxxqt_object) fn decode_image(
    path: &str,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    decode_image_with_policy(path, Policy::Convert, gpu, progress)
}

pub(in crate::cxxqt_object) fn decode_image_with_policy(
    path: &str,
    policy: Policy,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    decode_image_with_options(
        path,
        policy,
        pictura_codec::ConversionOptions::default(),
        gpu,
        progress,
    )
}

pub(in crate::cxxqt_object) fn decode_image_with_options(
    path: &str,
    policy: Policy,
    options: pictura_codec::ConversionOptions,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    let name = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    progress(5, "Reading file");
    let bytes = super::super::helpers::safe_io::read_capped(
        path,
        super::super::helpers::safe_io::import_file_limit(),
    )?;
    progress(25, "Decoding");
    let (rgba, width, height) = decode_import(&bytes)?;
    progress(70, "Compositing");
    let mut doc = Document::from_rgba(&name, width, height, &rgba);
    apply_raster_profile(&mut doc, &bytes, policy, options);
    finalize_import(&mut doc, &rgba);
    if let Some(section) = pictura_codec::raster_image_resources(&bytes) {
        doc.image_resources = section.into();
    }
    store_opened_composite(&mut doc, gpu);
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, Some("0".to_string()));
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
        preview: None,
        doc,
        history,
        target,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: format_for_path(path),
        pdf_password: None,
        compatibility: NativeCompatibility::default(),
    })
}

/// Decode `path` for an in-place Revert without touching the view. A PDF uses
/// the stored password; a missing, corrupt, or unreadable file returns `None`.
pub(in crate::cxxqt_object) fn decode_for_revert(
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
pub(in crate::cxxqt_object) fn decode_smart_object(
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

pub(super) fn decode_pdf_psd(
    psd: &[u8],
    path: &str,
    password: &str,
    options: pictura_codec::OpenOptions,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    progress(35, "Decoding");
    let mut doc = pictura_codec::read_psd_with_options(psd, &options).ok()?;
    progress(65, "Compositing");
    store_opened_composite(&mut doc, gpu);
    progress(90, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, None);
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
        preview: None,
        doc,
        history,
        target,
        path: OpenedPath::Set(Some(path.to_string())),
        source_format: "pdf".to_string(),
        pdf_password: (!password.is_empty()).then(|| password.to_string()),
        compatibility: NativeCompatibility::default(),
    })
}

/// Probe a Photoshop PDF and decode its editing data. A decode failure reads as
/// `None`, so the caller falls back to the import dialog, as the synchronous
/// `pdf_open` did.
pub(in crate::cxxqt_object) fn decode_pdf(
    path: &str,
    password: &str,
    policy: Policy,
    gpu: bool,
) -> PdfOpen {
    let bytes = std::fs::read(path).unwrap_or_default();
    match pictura_codec::pdf::editing_psd(&bytes, password) {
        pictura_codec::pdf::EditingPsd::Psd(psd) => {
            match decode_pdf_psd(&psd, path, password, open_options(policy), gpu, &|_, _| {}) {
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
pub(in crate::cxxqt_object) fn render_pdf_page(
    path: &str,
    password: &str,
    page: i32,
    area: &str,
    spec: &ImportSpec,
) -> Result<Imported, RasterError> {
    let numbers: Vec<f64> = area
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    let [x0, y0, x1, y1, view_height] = numbers[..] else {
        return Err(RasterError::Failed("bad page area".into()));
    };
    Ghostscript::locate().and_then(|gs| {
        let password = (!password.is_empty()).then_some(password);
        let size = (spec.width.max(1) as u32, spec.height.max(1) as u32);
        if spec.wants_native() {
            pdf_import::native::render_native_area(
                &gs,
                Path::new(path),
                password,
                page.max(1) as u32,
                PdfRect::new(x0, y0, x1, y1),
                view_height,
                size,
                spec.anti_alias,
                spec.color_mode(),
                spec.bit_depth(),
            )
            .map(Imported::Native)
        } else {
            pdf_import::render_area(
                &gs,
                Path::new(path),
                password,
                page.max(1) as u32,
                PdfRect::new(x0, y0, x1, y1),
                view_height,
                size,
                spec.anti_alias,
            )
            .map(Imported::Rgba)
        }
    })
}

/// Render embedded PDF image `index` at its native size.
pub(in crate::cxxqt_object) fn render_pdf_image(
    path: &str,
    password: &str,
    index: i32,
    spec: &ImportSpec,
) -> Result<Imported, RasterError> {
    let bytes = std::fs::read(path).map_err(|e| RasterError::Failed(e.to_string()))?;
    let found = PdfImages::read(&bytes, password)
        .ok_or_else(|| RasterError::Failed("no readable images".into()))?;
    Ghostscript::locate().and_then(|gs| {
        if spec.wants_native() {
            found
                .render_native(
                    &gs,
                    index.max(0) as usize,
                    spec.color_mode(),
                    spec.bit_depth(),
                )
                .map(Imported::Native)
        } else {
            found
                .render(&gs, index.max(0) as usize, None)
                .map(Imported::Rgba)
        }
    })
}

/// Rasterize an EPS's bounding box.
pub(in crate::cxxqt_object) fn render_eps(
    path: &str,
    spec: &ImportSpec,
) -> Result<Imported, RasterError> {
    Ghostscript::locate().and_then(|gs| {
        let bounds = std::fs::read(path)
            .ok()
            .as_deref()
            .and_then(eps::bounding_box)
            .ok_or_else(|| RasterError::Failed("no bounding box".into()))?;
        let size = (spec.width.max(1) as u32, spec.height.max(1) as u32);
        if spec.wants_native() {
            eps::render_native_eps(
                &gs,
                Path::new(path),
                bounds,
                size,
                spec.anti_alias,
                spec.color_mode(),
                spec.bit_depth(),
            )
            .map(Imported::Native)
        } else {
            eps::render_eps(&gs, Path::new(path), bounds, size, spec.anti_alias).map(Imported::Rgba)
        }
    })
}

/// Build the untitled document an import's raster becomes.
pub(in crate::cxxqt_object) fn imported_document(
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

/// Build the untitled document a native render becomes: the working RGB8
/// display plus the retained native samples.
pub(in crate::cxxqt_object) fn imported_native_document(
    native: &NativeRender,
    spec: &ImportSpec,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> Option<OpenedDocument> {
    progress(80, "Preparing layers");
    let (width, height) = (native.width, native.height);
    let plane = width as usize * height as usize;
    if plane == 0 || native.display.len() < plane * 3 || native.alpha.len() < plane {
        return None;
    }
    let mut rgba = vec![0u8; plane * 4];
    for i in 0..plane {
        rgba[i * 4] = native.display[i];
        rgba[i * 4 + 1] = native.display[plane + i];
        rgba[i * 4 + 2] = native.display[2 * plane + i];
        rgba[i * 4 + 3] = native.alpha[i];
    }
    let mut doc = Document::from_rgba(&spec.name, width, height, &rgba);
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
    doc.source
        .set_encoding(
            (native.mode == ColorMode::Cmyk).then_some(ColorMode::Cmyk),
            doc.source.palette(),
        )
        .expect("valid source encoding");
    doc.source
        .set_native(
            (native.depth == BitDepth::Sixteen).then_some(BitDepth::Sixteen),
            Some(SourcePlanes {
                depth: native.depth,
                width,
                height,
                samples: native.samples.clone(),
            }),
        )
        .expect("valid native source");
    let colours = native.mode.color_channels() as usize;
    let mut planes: Vec<(i16, Samples)> = Vec::with_capacity(colours + 1);
    for c in 0..colours {
        let start = c * plane;
        let end = start + plane;
        planes.push((c as i16, native.samples.slice(start..end)?));
    }
    let alpha = if native.depth == BitDepth::Sixteen {
        Samples::U16(
            native
                .alpha
                .iter()
                .map(|&a| u16::from(a) * 257)
                .collect::<Vec<_>>()
                .into(),
        )
    } else {
        Samples::U8(native.alpha.clone().into())
    };
    planes.push((-1, alpha));
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    if let Some(layer) = doc.layers.last_mut() {
        layer.source_channels = Some(SourceChannels::new(native.depth, rect, planes));
    }
    let target = spec.color_mode();
    if target == ColorMode::Grayscale || target == ColorMode::Lab {
        pictura_render::convert_mode(&mut doc, target).ok()?;
    }
    progress(90, "Compositing");
    let _ = DocumentComposite::new(&mut doc).refresh(pictura_render::CompositeScope::Full, gpu);
    progress(95, "Preparing layers");
    let target = LayerTarget::for_opened(&doc, Some("0".to_string()));
    let history = history_for(&mut doc, &target, "Open");
    Some(OpenedDocument {
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

pub(super) fn warnings_list(warnings: Vec<String>) -> QStringList {
    let mut list = QStringList::default();
    for warning in warnings {
        list.append(QString::from(warning.as_str()));
    }
    list
}

impl PictureViewRust {
    /// Start an open request and return its id. A result from an earlier
    /// request is stale and must not install.
    pub(in crate::cxxqt_object) fn begin_open_request(&mut self) -> u64 {
        self.open_cancelled = false;
        self.open_request = self.open_request.wrapping_add(1);
        self.open_request
    }
}

/// Apply a result unless the view was cancelled or the request is stale. True
/// when it was applied.
pub(super) fn apply_if_current(
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
pub(super) fn finish_on_gui(
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

pub(super) fn delay(ms: u64) {
    if ms > 0 {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

pub(super) fn error_status(error: &RasterError) -> String {
    format!("error:{error}")
}

pub(super) fn start(
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
    let (options, gpu) = {
        let mut rust = view.as_mut().rust_mut();
        let policy = rust
            .open_policy_override
            .take()
            .unwrap_or(rust.color_policy);
        let options = pictura_codec::OpenOptions {
            policy,
            working_cmyk: Some(rust.working_cmyk.clone()),
            conversion: rust.conversion_options,
            skip_layered_merged: true,
            source: None,
        };
        (options, rust.gpu_compute)
    };
    let budget = open_budget();
    OPEN_REFUSED_FOR_MEMORY.store(false, Ordering::Relaxed);
    start(&mut view, move |progress| {
        match decode_psd_with(&path, options, budget, gpu, progress) {
            Ok(opened) => (Some(opened), "ok".to_string(), Vec::new()),
            Err(_) if open_refused_for_memory() => (None, "error:memory".to_string(), Vec::new()),
            Err(error @ pictura_codec::PsdError::NativeVersion(_)) => {
                (None, format!("error:{error}"), Vec::new())
            }
            Err(_) => (None, "error".to_string(), Vec::new()),
        }
    });
}

pub fn open_image_start(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
    let (policy, options, gpu) = {
        let mut rust = view.as_mut().rust_mut();
        let policy = rust
            .open_policy_override
            .take()
            .unwrap_or(rust.color_policy);
        (policy, rust.conversion_options, rust.gpu_compute)
    };
    start(&mut view, move |progress| {
        match decode_image_with_options(&path, policy, options, gpu, progress) {
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
    let options = {
        let rust = view.rust();
        pictura_codec::OpenOptions {
            policy: rust.color_policy,
            working_cmyk: Some(rust.working_cmyk.clone()),
            conversion: rust.conversion_options,
            skip_layered_merged: true,
            source: None,
        }
    };
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
                let opened = decode_pdf_psd(&psd, &path, &password, options, gpu, &progress);
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

/// Turn one import render into the result the open worker reports.
fn finish_import(
    rendered: Imported,
    spec: &ImportSpec,
    gpu: bool,
    progress: &dyn Fn(u8, &str),
) -> (Option<OpenedDocument>, String, Vec<String>) {
    match rendered {
        Imported::Rgba(rendered) => {
            let warnings = rendered.warnings.clone();
            match imported_document(&rendered.raster, spec, gpu, progress) {
                Some(opened) => (Some(opened), "ok".to_string(), warnings),
                None => (
                    None,
                    "error:the image could not be converted".to_string(),
                    Vec::new(),
                ),
            }
        }
        Imported::Native(native) => {
            let warnings = native.warnings.clone();
            match imported_native_document(&native, spec, gpu, progress) {
                Some(opened) => (Some(opened), "ok".to_string(), warnings),
                None => (
                    None,
                    "error:the image could not be converted".to_string(),
                    Vec::new(),
                ),
            }
        }
    }
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
            Ok(rendered) => finish_import(rendered, &spec, gpu, progress),
            Err(error) => (None, error_status(&error), Vec::new()),
        }
    });
}

pub fn open_pdf_image_start(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    password: &QString,
    index: i32,
    target: &ImportTarget,
) {
    let path = path.to_string();
    let password = password.to_string();
    let spec = ImportSpec::from_target(target);
    let gpu = view.rust().gpu_compute;
    start(&mut view, move |progress| {
        progress(10, "Rendering image");
        match render_pdf_image(&path, &password, index, &spec) {
            Ok(rendered) => finish_import(rendered, &spec, gpu, progress),
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
            Ok(rendered) => finish_import(rendered, &spec, gpu, progress),
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
