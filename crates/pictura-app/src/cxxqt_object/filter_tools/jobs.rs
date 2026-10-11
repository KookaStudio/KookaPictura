//! Filter work off the UI thread (#152). On a document the size of the
//! 16507×16196 world map a commit takes seconds and an artistic filter's exact
//! preview longer; running them here keeps the event loop free, so the dialog
//! and the busy indicator stay live. Free functions (their own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! A worker never touches the live document: it filters a clone (planes are
//! shared copy-on-write, so the clone is cheap and the first write copies only
//! what the filter changes). An exact preview hands back the filtered layer
//! and its reduced picture; a commit also composites the changed region and
//! captures history, which it holds until the UI thread swaps the results in.

use super::super::filter_map::filter_from_kind_params;
use super::super::helpers::{active_mask_target, clamp_region, layer_has_effects};
use super::super::helpers_composite::{
    current_buffer, patch_composite_region, premultiplied_display_image, store_composite,
};
use super::super::impl_filters::{apply_op_active_region, pixel_filter_target, ActiveOp};
use super::super::qobject::PictureView;
use super::super::state::PictureViewRust;
use super::canvas_proxy::{pre_preview_sources, section_buffer};
use super::resolve_stack;
use crate::history::History;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QList, QString, QStringList};
use pictura_core::{Document, Layer, LayerMask, PixelBuffer, PsdRect};
use pictura_filters::Filter;
use pictura_select::Selection;
use std::thread::JoinHandle;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;

        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start committing `kind` with `params` to the active layer on a worker
        /// thread, recorded as the history state `label`. An open preview is put
        /// back first; a finished exact preview of the same filter is reused.
        /// A layer-mask target is filtered at once instead. False when refused
        /// (see `filter_last_error`).
        fn filter_commit_start(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            label: &QString,
        ) -> bool;

        /// As [`filter_commit_start`] for a Filter Gallery stack: `kinds` in
        /// order, their parameter slots concatenated in `params`.
        fn filter_stack_commit_start(
            view: Pin<&mut PictureView>,
            kinds: &QStringList,
            params: &QList_f64,
            label: &QString,
        ) -> bool;

        /// Whether a started commit is still running.
        fn filter_commit_running(view: &PictureView) -> bool;

        /// Wait for the started commit and swap its document, history and
        /// refreshed region in. True when it applied (or there was nothing
        /// running); false when the worker refused it.
        fn filter_commit_finish(view: Pin<&mut PictureView>) -> bool;

        /// Ask for the exact preview of `kind`/`params`: the full-resolution
        /// layer filtered on a worker, shown reduced like
        /// `filter_canvas_proxy` over the document rect `(x, y, w, h)` at
        /// `scale`. A newer request supersedes an older one. False for a
        /// layer-mask target or an unknown filter.
        #[allow(clippy::too_many_arguments)]
        fn filter_exact_preview_start(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            scale: f64,
        ) -> bool;

        /// The latest exact preview's picture the first time it is ready, as
        /// a premultiplied display image; null otherwise.
        fn filter_exact_preview_poll(view: Pin<&mut PictureView>) -> QImage;

        /// Whether the latest exact preview is still being computed.
        fn filter_exact_preview_busy(view: Pin<&mut PictureView>) -> bool;

        /// Drop the exact preview; a running worker finishes unobserved.
        fn filter_exact_preview_cancel(view: Pin<&mut PictureView>);

        /// The document rect `(x, y, w, h)` with `kind`/`params` applied to the
        /// active layer, at full resolution, without changing the document:
        /// the dialog pane's exact section while the canvas shows a reduced
        /// copy. Null when refused or for a layer-mask target.
        #[allow(clippy::too_many_arguments)]
        fn filter_section_image(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> QImage;
    }
}

/// The background filter work a document has in flight.
#[derive(Default)]
pub(in crate::cxxqt_object) struct FilterJobs {
    preview: Option<ExactPreview>,
    commit: Option<CommitJob>,
}

/// What an exact preview filters and shows.
#[derive(Clone, PartialEq)]
struct Request {
    path: String,
    filters: Vec<Filter>,
    region: PsdRect,
    scale: u64,
}

/// A filtered full-resolution layer and its reduced picture.
struct Exact {
    layer: Layer,
    picture: PixelBuffer,
}

/// The latest exact-preview request and the one worker serving requests:
/// a request that arrives while it runs waits, and only the latest is kept.
struct ExactPreview {
    latest: Request,
    running: Option<(Request, JoinHandle<Option<Exact>>)>,
    done: Option<(Request, Exact)>,
    failed: Option<Request>,
    delivered: bool,
}

impl ExactPreview {
    fn settled(&self) -> bool {
        self.done.as_ref().is_some_and(|(r, _)| *r == self.latest)
            || self.failed.as_ref() == Some(&self.latest)
    }
}

/// Where a commit's filtered layer comes from.
enum Source {
    Filtered(Box<Layer>),
    Pending(JoinHandle<Option<Exact>>),
    Filter,
}

struct CommitJob {
    handle: JoinHandle<Committed>,
    last_filter: Option<(String, Vec<f64>)>,
}

/// A worker's commit: the new document and history, and what to refresh.
struct Committed {
    doc: Document,
    history: History,
    outcome: Result<Refresh, String>,
}

enum Refresh {
    Region {
        x0: i32,
        y0: i32,
        clipped: PsdRect,
        srgb: PixelBuffer,
    },
    Everywhere,
    Nothing,
}

/// The layer at `path` of `doc` with `filters` applied in order.
fn filter_layer(
    doc: &Document,
    path: &str,
    filters: &[Filter],
    mask: Option<&LayerMask>,
    gpu: bool,
) -> Result<Layer, String> {
    let mut layer = pictura_render::resolve_path(doc, path)
        .ok_or("the active layer no longer exists")?
        .clone();
    for filter in filters {
        pictura_render::apply_filter(&mut layer, filter, mask, gpu).map_err(|e| e.to_string())?;
    }
    Ok(layer)
}

fn exact(
    mut doc: Document,
    request: &Request,
    mask: Option<LayerMask>,
    gpu: bool,
) -> Option<Exact> {
    let layer = filter_layer(&doc, &request.path, &request.filters, mask.as_ref(), gpu).ok()?;
    *pictura_render::resolve_path_mut(&mut doc, &request.path)? = layer.clone();
    let scale = f64::from_bits(request.scale);
    let proxy = pictura_render::FilterProxy::new(&doc, None, request.region, scale)?;
    let rendered = proxy.render(&request.path, &[], false).ok()?;
    let picture = pictura_codec::buffer_to_srgb(&doc, &rendered).into_owned();
    Some(Exact { layer, picture })
}

/// Collect a finished worker and start the latest request when none runs.
fn advance(rust: &mut PictureViewRust) {
    let gpu = rust.gpu_compute;
    let Some(preview) = rust.filter_jobs.preview.as_mut() else {
        return;
    };
    if preview
        .running
        .as_ref()
        .is_some_and(|(_, handle)| handle.is_finished())
    {
        let (request, handle) = preview.running.take().expect("checked above");
        match handle.join().ok().flatten() {
            Some(exact) if request == preview.latest => preview.done = Some((request, exact)),
            Some(_) => {}
            None => preview.failed = Some(request),
        }
    }
    if preview.running.is_some() || preview.settled() {
        return;
    }
    let request = preview.latest.clone();
    let Some((doc, mask)) = pre_preview_sources(rust) else {
        return;
    };
    let job = request.clone();
    let handle = std::thread::spawn(move || exact(doc, &job, mask, gpu));
    if let Some(preview) = rust.filter_jobs.preview.as_mut() {
        preview.running = Some((request, handle));
    }
}

pub(in crate::cxxqt_object) fn exact_preview_start(
    rust: &mut PictureViewRust,
    filters: Vec<Filter>,
    region: PsdRect,
    scale: f64,
) -> bool {
    if active_mask_target(rust).is_some() {
        return false;
    }
    let Some(path) = rust.active_layer.clone() else {
        return false;
    };
    let request = Request {
        path,
        filters,
        region,
        scale: scale.to_bits(),
    };
    match rust.filter_jobs.preview.as_mut() {
        Some(preview) if preview.latest == request => {}
        Some(preview) => {
            preview.latest = request;
            preview.delivered = false;
        }
        None => {
            rust.filter_jobs.preview = Some(ExactPreview {
                latest: request,
                running: None,
                done: None,
                failed: None,
                delivered: false,
            })
        }
    }
    advance(rust);
    true
}

pub(in crate::cxxqt_object) fn exact_preview_poll(
    rust: &mut PictureViewRust,
) -> Option<PixelBuffer> {
    advance(rust);
    let preview = rust.filter_jobs.preview.as_mut()?;
    if preview.delivered {
        return None;
    }
    let (request, exact) = preview.done.as_ref()?;
    if *request != preview.latest {
        return None;
    }
    preview.delivered = true;
    Some(exact.picture.clone())
}

pub(in crate::cxxqt_object) fn exact_preview_busy(rust: &mut PictureViewRust) -> bool {
    advance(rust);
    rust.filter_jobs
        .preview
        .as_ref()
        .is_some_and(|preview| !preview.settled())
}

/// Start a commit of `filters` on a worker; `false` with `filter_error` set
/// when refused.
pub(in crate::cxxqt_object) fn commit_start(
    rust: &mut PictureViewRust,
    filters: Vec<Filter>,
    label: String,
    last_filter: Option<(String, Vec<f64>)>,
) -> bool {
    rust.filter_error = None;
    if rust.filter_jobs.commit.is_some() {
        rust.filter_error = Some("a filter is already being applied".to_string());
        return false;
    }
    let Some(path) = pixel_filter_target(rust) else {
        return false;
    };
    let Some((doc, mask)) = pre_preview_sources(rust) else {
        return false;
    };
    // The live layer goes back to its pre-preview pixels; the worker filters
    // the same pixels and its document replaces this one on finish.
    if let (Some(preview), Some(live)) = (rust.filter_preview.take(), rust.doc.as_mut()) {
        if let Some(layer) = pictura_render::resolve_path_mut(live, &preview.layer_path) {
            preview.baseline.restore(layer);
        }
    }
    rust.canvas_proxy = None;
    let matches = |request: &Request| request.path == path && request.filters == filters;
    let source = match rust.filter_jobs.preview.take() {
        Some(ExactPreview {
            done: Some((request, exact)),
            ..
        }) if matches(&request) => Source::Filtered(Box::new(exact.layer)),
        Some(ExactPreview {
            running: Some((request, handle)),
            ..
        }) if matches(&request) => Source::Pending(handle),
        _ => Source::Filter,
    };
    let selection = rust.selection.clone();
    let history = std::mem::take(&mut rust.history);
    let gpu = rust.gpu_compute;
    let handle = std::thread::spawn(move || {
        commit(
            doc, history, &path, &filters, source, mask, &selection, &label, gpu,
        )
    });
    rust.filter_jobs.commit = Some(CommitJob {
        handle,
        last_filter,
    });
    true
}

#[allow(clippy::too_many_arguments)]
fn commit(
    mut doc: Document,
    mut history: History,
    path: &str,
    filters: &[Filter],
    source: Source,
    mask: Option<LayerMask>,
    selection: &Option<Selection>,
    label: &str,
    gpu: bool,
) -> Committed {
    let filtered = match source {
        Source::Filtered(layer) => Ok(*layer),
        Source::Pending(handle) => match handle.join().ok().flatten() {
            Some(exact) => Ok(exact.layer),
            None => filter_layer(&doc, path, filters, mask.as_ref(), gpu),
        },
        Source::Filter => filter_layer(&doc, path, filters, mask.as_ref(), gpu),
    };
    let layer = match filtered {
        Ok(layer) => layer,
        Err(error) => {
            return Committed {
                doc,
                history,
                outcome: Err(error),
            }
        }
    };
    let (rect, effects) = (layer.rect, layer_has_effects(&layer));
    match pictura_render::resolve_path_mut(&mut doc, path) {
        Some(slot) => *slot = layer,
        None => {
            return Committed {
                doc,
                history,
                outcome: Err("the active layer no longer exists".to_string()),
            }
        }
    }
    // As `refresh_region` / `recomposite` do on the UI thread: a filter
    // changes only its layer's rect unless a layer effect spills past it.
    let refresh = if effects {
        let rendered = current_buffer(&doc, gpu);
        store_composite(&mut doc, &rendered);
        Refresh::Everywhere
    } else {
        match clamp_region(rect, doc.width, doc.height) {
            Some((x0, y0, w, h)) => {
                let buffer = pictura_render::composite_region_active(&doc, rect, gpu).0;
                let srgb = pictura_codec::buffer_to_srgb(&doc, &buffer).into_owned();
                patch_composite_region(&mut doc, &buffer, x0, y0);
                let clipped = PsdRect {
                    top: y0,
                    left: x0,
                    bottom: y0 + h as i32,
                    right: x0 + w as i32,
                };
                Refresh::Region {
                    x0,
                    y0,
                    clipped,
                    srgb,
                }
            }
            None => Refresh::Nothing,
        }
    };
    history.capture_live(&mut doc, selection, label);
    Committed {
        doc,
        history,
        outcome: Ok(refresh),
    }
}

fn commit_filters(
    mut view: Pin<&mut PictureView>,
    filters: Vec<Filter>,
    label: String,
    last_filter: Option<(String, Vec<f64>)>,
) -> bool {
    if active_mask_target(view.rust()).is_none() {
        let mut rust = view.as_mut().rust_mut();
        return commit_start(&mut rust, filters, label, last_filter);
    }
    // A layer mask is a mask-sized plane; filter it here.
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_op_active_region(&mut rust, &ActiveOp::Filters(filters), true, None)
    };
    let Some(region) = region else {
        return false;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    view.as_mut().record(&label);
    if last_filter.is_some() {
        view.as_mut().rust_mut().last_filter = last_filter;
    }
    true
}

fn filter_commit_start(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    label: &QString,
) -> bool {
    let kind = kind.to_string();
    let params: Vec<f64> = params.into_iter().copied().collect();
    let Some(filter) = filter_from_kind_params(&kind, &params) else {
        view.as_mut().rust_mut().filter_error = Some(format!(
            "filter '{kind}' is not recognised or its parameters are wrong"
        ));
        return false;
    };
    commit_filters(view, vec![filter], label.to_string(), Some((kind, params)))
}

fn filter_stack_commit_start(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    label: &QString,
) -> bool {
    match resolve_stack(kinds, params) {
        Some(filters) if !filters.is_empty() => {
            commit_filters(view, filters, label.to_string(), None)
        }
        _ => {
            view.as_mut().rust_mut().filter_error =
                Some("the gallery stack has an unknown filter or wrong parameters".to_string());
            false
        }
    }
}

impl FilterJobs {
    /// Whether a started commit is still running on its worker.
    pub(in crate::cxxqt_object) fn commit_running(&self) -> bool {
        self.commit
            .as_ref()
            .is_some_and(|job| !job.handle.is_finished())
    }
}

fn filter_commit_running(view: &PictureView) -> bool {
    view.rust().filter_jobs.commit_running()
}

/// What the canvas needs after [`commit_finish`] landed a commit.
pub(in crate::cxxqt_object) enum Landed {
    /// Level 0 and the pyramid were patched over this document origin.
    Region {
        x0: i32,
        y0: i32,
    },
    /// The pyramid was rebuilt.
    Everywhere,
    Nothing,
}

/// Wait for the started commit and swap its document and history in, patching
/// level 0 and the pyramid: the part of a commit the UI thread still does.
/// `Ok(Landed::Nothing)` when no commit was running; `Err` (with
/// `filter_error` set) when the worker refused.
pub(in crate::cxxqt_object) fn commit_finish(rust: &mut PictureViewRust) -> Result<Landed, ()> {
    let Some(job) = rust.filter_jobs.commit.take() else {
        return Ok(Landed::Nothing);
    };
    let Ok(committed) = job.handle.join() else {
        rust.filter_error = Some("the filter stopped unexpectedly; history was reset".to_string());
        return Err(());
    };
    rust.history = committed.history;
    let refresh = match committed.outcome {
        Ok(refresh) => refresh,
        Err(error) => {
            rust.filter_error = Some(error);
            return Err(());
        }
    };
    rust.doc = Some(committed.doc);
    rust.dirty = true;
    rust.content_revision = rust.content_revision.wrapping_add(1);
    if job.last_filter.is_some() {
        rust.last_filter = job.last_filter;
    }
    Ok(match refresh {
        Refresh::Region {
            x0,
            y0,
            clipped,
            srgb,
        } => {
            rust.apply_refreshed_region(x0, y0, clipped, srgb);
            Landed::Region { x0, y0 }
        }
        Refresh::Everywhere => {
            rust.reset_pyramid();
            Landed::Everywhere
        }
        Refresh::Nothing => Landed::Nothing,
    })
}

fn filter_commit_finish(mut view: Pin<&mut PictureView>) -> bool {
    let landed = {
        let mut rust = view.as_mut().rust_mut();
        commit_finish(&mut rust)
    };
    match landed {
        Err(()) => return false,
        Ok(Landed::Region { x0, y0 }) => {
            // A document canvas re-crops the pyramid on any blit and never
            // reads the image, so a pixel stands for the region rather than a
            // second full-size copy of it.
            let marker = PixelBuffer::new(1, 1, 4);
            view.as_mut()
                .region_blitted(premultiplied_display_image(&marker), x0, y0);
        }
        Ok(Landed::Everywhere) => view.as_mut().changed(),
        Ok(Landed::Nothing) => {}
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn filter_exact_preview_start(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scale: f64,
) -> bool {
    let params: Vec<f64> = params.into_iter().copied().collect();
    let Some(filter) = filter_from_kind_params(&kind.to_string(), &params) else {
        return false;
    };
    let region = PsdRect {
        top: y,
        left: x,
        bottom: y + h.max(0),
        right: x + w.max(0),
    };
    let mut rust = view.as_mut().rust_mut();
    exact_preview_start(&mut rust, vec![filter], region, scale)
}

fn filter_exact_preview_poll(mut view: Pin<&mut PictureView>) -> QImage {
    let mut rust = view.as_mut().rust_mut();
    exact_preview_poll(&mut rust).map_or_else(QImage::default, |picture| {
        premultiplied_display_image(&picture)
    })
}

fn filter_exact_preview_busy(mut view: Pin<&mut PictureView>) -> bool {
    let mut rust = view.as_mut().rust_mut();
    exact_preview_busy(&mut rust)
}

fn filter_exact_preview_cancel(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().filter_jobs.preview = None;
}

#[allow(clippy::too_many_arguments)]
fn filter_section_image(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> QImage {
    let params: Vec<f64> = params.into_iter().copied().collect();
    let Some(filter) = filter_from_kind_params(&kind.to_string(), &params) else {
        return QImage::default();
    };
    let section = PsdRect {
        top: y,
        left: x,
        bottom: y + h.max(0),
        right: x + w.max(0),
    };
    let mut rust = view.as_mut().rust_mut();
    section_buffer(&mut rust, &filter, section).map_or_else(QImage::default, |buffer| {
        premultiplied_display_image(&buffer)
    })
}

#[cfg(test)]
mod tests;
