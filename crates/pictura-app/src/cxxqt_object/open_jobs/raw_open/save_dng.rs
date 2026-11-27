//! Save Image (#402, `document/raw-save-image`): the raw open's Pictura Raw
//! dialog writes a DNG on a worker while the dialog stays open.
//!
//! The pending source keeps only demosaiced camera RGB, so the worker reads
//! the raw file again (refusing it when its content hash no longer matches
//! the open's), takes the mosaic plane or, for a linear DNG, demosaics it
//! once more, and develops the previews with the dialog's settings.

use super::super::imp::open_budget;
use super::imp::{develop_pixels, reason, to8, PendingRaw};
use super::store::{replace_file, with_white_balance_label};
use super::PictureView;
use crate::cxxqt_object::helpers::safe_io;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::{
    CameraRgb, DngCompatibility, DngPixels, DngRgb8, DngWrite, ImageBudget, RawLook, WhiteBalance,
};
use pictura_core::PicturaRawSettings;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Save Options' JPEG Preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) enum PreviewSize {
    None,
    /// The pending ≤ 1024 px preview source, developed.
    Medium,
    Full,
}

impl PreviewSize {
    /// The menu index the shell passes: 0 None, 2 Full Size, else Medium.
    pub(super) fn from_index(index: i32) -> Self {
        match index {
            0 => Self::None,
            2 => Self::Full,
            _ => Self::Medium,
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::cxxqt_object) struct DngOptions {
    pub(super) target: PathBuf,
    pub(super) compatibility: DngCompatibility,
    pub(super) preview: PreviewSize,
    pub(super) embed_original: bool,
    pub(super) linear: bool,
}

/// What the worker needs from the pending raw, owned so it can leave the
/// GUI thread.
pub(super) struct DngRequest {
    pub(super) source: PathBuf,
    pub(super) key: String,
    pub(super) record: Option<Vec<u8>>,
    pub(super) preview: CameraRgb,
    pub(super) preview_look: RawLook,
    pub(super) as_shot: WhiteBalance,
    pub(super) settings: PicturaRawSettings,
    pub(super) options: DngOptions,
}

impl DngRequest {
    pub(super) fn new(
        pending: &PendingRaw,
        settings: PicturaRawSettings,
        options: DngOptions,
    ) -> Self {
        Self {
            source: PathBuf::from(&pending.path),
            key: pending.key.clone(),
            record: pending.record.clone(),
            preview: pending.preview.clone(),
            preview_look: pending.preview_look.clone(),
            as_shot: pending.source.as_shot,
            settings,
            options,
        }
    }
}

/// How a save ended: written, or cancelled before the file was placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) enum DngOutcome {
    Written,
    Cancelled,
}

/// A save running on a worker; progress is 0–1000.
pub(in crate::cxxqt_object) struct DngJob {
    pub(super) handle: JoinHandle<Result<DngOutcome, String>>,
    pub(super) progress: Arc<AtomicI32>,
    pub(super) cancel: Arc<AtomicBool>,
}

impl DngJob {
    pub(super) fn start(request: DngRequest, budget: ImageBudget) -> Self {
        let progress = Arc::new(AtomicI32::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (p, c) = (progress.clone(), cancel.clone());
        let handle = std::thread::spawn(move || {
            save_dng(
                &request,
                &budget,
                &|v| p.store(v, Ordering::Relaxed),
                &|| c.load(Ordering::Relaxed),
            )
        });
        Self {
            handle,
            progress,
            cancel,
        }
    }
}

/// `<stem>.dng` beside `source`, or `<stem>-<n>.dng` with the smallest free
/// `n`, so the default never names an existing file (the source included).
pub(super) fn default_dng_path(source: &Path) -> PathBuf {
    let folder = source.parent().unwrap_or(Path::new("."));
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string());
    let mut candidate = folder.join(format!("{stem}.dng"));
    let mut n = 1;
    while candidate.exists() {
        candidate = folder.join(format!("{stem}-{n}.dng"));
        n += 1;
    }
    candidate
}

/// Whether `target` names the same file as `source`, resolving links and
/// relative folders (the target need not exist).
fn same_file(source: &Path, target: &Path) -> bool {
    let resolve = |p: &Path| -> Option<PathBuf> {
        let parent = p.parent().filter(|p| !p.as_os_str().is_empty());
        let folder = std::fs::canonicalize(parent.unwrap_or(Path::new("."))).ok()?;
        let joined = folder.join(p.file_name()?);
        Some(std::fs::canonicalize(&joined).unwrap_or(joined))
    };
    match (resolve(source), resolve(target)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn rgb8(width: usize, height: usize, rgb: &[u16]) -> (usize, usize, Vec<u8>) {
    (width, height, rgb.iter().map(|&v| to8(v)).collect())
}

/// The full-resolution pixels, the look, and the tags, from a fresh read.
struct Image {
    pixels: Vec<u16>,
    cfa: Option<pictura_codec::CfaPattern>,
    width: usize,
    height: usize,
    look: RawLook,
    make: Option<String>,
    model: Option<String>,
}

fn read_image(bytes: &[u8], linear: bool, budget: &ImageBudget) -> Result<Image, String> {
    if linear {
        let source = pictura_codec::decode_raw_source(bytes, budget).map_err(reason)?;
        return Ok(Image {
            width: source.camera.width,
            height: source.camera.height,
            pixels: source.camera.rgb,
            cfa: None,
            look: source.look,
            make: source.make,
            model: source.model,
        });
    }
    let image = pictura_codec::read_raw(bytes, budget).map_err(reason)?;
    // A mosaic DNG cannot carry a SuperCCD's turned lattice (#1621).
    if image.fuji_width.is_some() {
        let camera = pictura_codec::demosaic(&image, pictura_codec::DemosaicMethod::Ahd, budget)
            .map_err(reason)?;
        return Ok(Image {
            width: camera.width,
            height: camera.height,
            pixels: camera.rgb,
            cfa: None,
            look: RawLook {
                color: image.color,
                crop: image.default_crop,
                orientation: image.orientation,
            },
            make: image.make,
            model: image.model,
        });
    }
    Ok(Image {
        width: image.width,
        height: image.height,
        pixels: image.samples,
        cfa: Some(image.cfa),
        look: RawLook {
            color: image.color,
            crop: image.default_crop,
            orientation: image.orientation,
        },
        make: image.make,
        model: image.model,
    })
}

/// The Full Size preview: the full image developed with the settings.
fn full_preview(
    image: &Image,
    request: &DngRequest,
    budget: &ImageBudget,
) -> Result<(usize, usize, Vec<u8>), String> {
    let camera = match &image.cfa {
        None => CameraRgb {
            width: image.width,
            height: image.height,
            rgb: image.pixels.clone(),
        },
        Some(cfa) => {
            let mosaic = pictura_codec::DngImage {
                width: image.width,
                height: image.height,
                samples: image.pixels.clone(),
                cfa: cfa.clone(),
                fuji_width: None,
                default_crop: image.look.crop,
                orientation: image.look.orientation,
                make: None,
                model: None,
                color: image.look.color.clone(),
            };
            pictura_codec::demosaic(&mosaic, pictura_codec::DemosaicMethod::Ahd, budget)
                .map_err(reason)?
        }
    };
    let developed = develop_pixels(camera, &image.look, request.as_shot, &request.settings)?;
    Ok(rgb8(developed.width, developed.height, &developed.rgb))
}

/// Writes the DNG `request` describes; progress is 0–1000.
pub(super) fn save_dng(
    request: &DngRequest,
    budget: &ImageBudget,
    progress: &(dyn Fn(i32) + Sync),
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<DngOutcome, String> {
    let options = &request.options;
    let name = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    if same_file(&request.source, &options.target) {
        return Err(format!(
            "{} is the raw file being edited and cannot be replaced",
            name(&request.source)
        ));
    }
    progress(20);
    let bytes = safe_io::read_capped(&request.source, safe_io::import_file_limit())
        .ok_or_else(|| "the raw file could not be read".to_string())?;
    if pictura_codec::raw_settings_key(&bytes) != request.key {
        return Err("the raw file changed since it was opened".to_string());
    }
    if cancelled() {
        return Ok(DngOutcome::Cancelled);
    }
    progress(100);
    let image = read_image(&bytes, options.linear, budget)?;
    if cancelled() {
        return Ok(DngOutcome::Cancelled);
    }
    progress(300);

    let developed = develop_pixels(
        request.preview.clone(),
        &request.preview_look,
        request.as_shot,
        &request.settings,
    )?;
    let look = rgb8(developed.width, developed.height, &developed.rgb);
    let full = match options.preview {
        PreviewSize::Full => Some(full_preview(&image, request, budget)?),
        _ => None,
    };
    let preview = match options.preview {
        PreviewSize::None => None,
        PreviewSize::Medium => Some(&look),
        PreviewSize::Full => full.as_ref(),
    };
    if cancelled() {
        return Ok(DngOutcome::Cancelled);
    }
    progress(400);

    let labelled = with_white_balance_label(&request.settings, request.as_shot);
    let xmp = pictura_codec::pictura_raw_to_xmp(request.record.as_deref(), &labelled)
        .or_else(|| pictura_codec::pictura_raw_to_xmp(None, &labelled));
    let original_name = name(&request.source);
    let pixels = match &image.cfa {
        Some(cfa) => DngPixels::Mosaic {
            samples: &image.pixels,
            cfa,
        },
        None => DngPixels::Linear { rgb: &image.pixels },
    };
    fn as_rgb8((width, height, rgb): &(usize, usize, Vec<u8>)) -> DngRgb8<'_> {
        DngRgb8 {
            width: *width,
            height: *height,
            rgb,
        }
    }
    let write = DngWrite {
        width: image.width,
        height: image.height,
        pixels,
        color: &image.look.color,
        crop: image.look.crop,
        orientation: image.look.orientation,
        make: image.make.as_deref(),
        model: image.model.as_deref(),
        look: as_rgb8(&look),
        preview: preview.map(as_rgb8),
        xmp: xmp.as_deref(),
        original: options
            .embed_original
            .then_some((original_name.as_str(), bytes.as_slice())),
        compatibility: options.compatibility,
    };
    let Some(dng) =
        pictura_codec::write_dng(&write, &|f| progress(400 + (f * 500.0) as i32), cancelled)
            .map_err(reason)?
    else {
        return Ok(DngOutcome::Cancelled);
    };
    if cancelled() {
        return Ok(DngOutcome::Cancelled);
    }
    progress(950);
    replace_file(&options.target, &dng)
        .map_err(|error| format!("{}: {error}", options.target.display()))?;
    progress(1000);
    Ok(DngOutcome::Written)
}

// The bridge (`open_jobs.rs`): the job lives on the pending raw, which the
// modal Pictura Raw dialog keeps until it closes.

pub fn raw_dng_default_path(view: &PictureView) -> QString {
    view.rust()
        .pending_raw
        .as_ref()
        .map(|pending| {
            QString::from(
                default_dng_path(Path::new(&pending.path))
                    .to_string_lossy()
                    .as_ref(),
            )
        })
        .unwrap_or_default()
}

pub fn raw_save_dng_start(
    mut view: Pin<&mut PictureView>,
    target: &QString,
    settings: &QString,
    compatibility: i32,
    preview: i32,
    embed_original: bool,
    linear: bool,
) -> bool {
    let settings = pictura_codec::pictura_raw_from_text(&settings.to_string());
    let options = DngOptions {
        target: PathBuf::from(target.to_string()),
        compatibility: DngCompatibility::from_index(compatibility),
        preview: PreviewSize::from_index(preview),
        embed_original,
        linear,
    };
    let (budget, _) = open_budget();
    let mut rust = view.as_mut().rust_mut();
    let Some(pending) = rust.pending_raw.as_mut() else {
        return false;
    };
    if pending.dng_job.is_some() {
        return false;
    }
    let request = DngRequest::new(pending, settings, options);
    pending.dng_job = Some(DngJob::start(request, budget));
    true
}

fn job(view: &PictureView) -> Option<&DngJob> {
    view.rust().pending_raw.as_ref()?.dng_job.as_ref()
}

pub fn raw_save_dng_running(view: &PictureView) -> bool {
    job(view).is_some_and(|job| !job.handle.is_finished())
}

pub fn raw_save_dng_progress(view: &PictureView) -> i32 {
    job(view).map_or(-1, |job| job.progress.load(Ordering::Relaxed))
}

pub fn raw_save_dng_cancel(view: &PictureView) {
    if let Some(job) = job(view) {
        job.cancel.store(true, Ordering::Relaxed);
    }
}

/// `"ok"`, `"cancelled"` or `"error:<reason>"`; waits for the worker.
pub fn raw_save_dng_finish(mut view: Pin<&mut PictureView>) -> QString {
    let job = view
        .as_mut()
        .rust_mut()
        .pending_raw
        .as_mut()
        .and_then(|pending| pending.dng_job.take());
    let status = match job.map(|job| job.handle.join()) {
        None => "error:no save is running".to_string(),
        Some(Ok(Ok(DngOutcome::Written))) => "ok".to_string(),
        Some(Ok(Ok(DngOutcome::Cancelled))) => "cancelled".to_string(),
        Some(Ok(Err(reason))) => format!("error:{reason}"),
        Some(Err(_)) => "error:the save stopped unexpectedly".to_string(),
    };
    QString::from(status.as_str())
}
