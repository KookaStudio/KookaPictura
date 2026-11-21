use super::*;

use super::super::super::fade::FadeSnapshot;
use super::super::super::filter_map::filter_from_kind_params;
use super::super::super::helpers::{clamp_region, layer_has_effects};
use super::super::super::helpers_composite::premultiplied_display_image;
use super::super::super::impl_filters::{
    apply_op_active_region, cancel_filter_preview, pixel_filter_target, ActiveOp,
};
use super::super::super::presentation::{Dirty, Presentation, Rebuild};
use super::super::super::state::PictureViewRust;
use super::super::canvas_proxy::{pre_preview_sources, section_buffer};
use super::super::resolve_stack;
use crate::history::History;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QList, QString, QStringList};
use pictura_core::{Coverage, Document, Layer, PixelBuffer, PsdRect};
use pictura_error::{guard, Error, ErrorCode, OpStatus};
use pictura_filters::{Filter, FilterError};
use pictura_render::DocumentComposite;
use pictura_select::Selection;
use std::panic::AssertUnwindSafe;
use std::thread::JoinHandle;

/// The background filter work a document has in flight.
#[derive(Default)]
pub(in crate::cxxqt_object) struct FilterJobs {
    preview: Option<ExactPreview>,
    commit: Option<CommitJob>,
    pub(in crate::cxxqt_object) commit_status: OpStatus,
    panic_for_test: i32,
    /// Test seam: hold a finished exact preview before handing it over.
    pub(in crate::cxxqt_object) exact_delay_for_test: u64,
    /// Test seam: hold a finished commit before it lands.
    pub(in crate::cxxqt_object) commit_delay_for_test: u64,
}

/// What an exact preview filters and shows.
#[derive(Clone, PartialEq)]
pub(super) struct Request {
    path: String,
    filters: Vec<Filter>,
    region: PsdRect,
    scale: u64,
}

/// A filtered full-resolution layer and its reduced picture.
pub(super) struct Exact {
    layer: Layer,
    picture: PixelBuffer,
}

/// The latest exact-preview request and the one worker serving requests:
/// a request that arrives while it runs waits, and only the latest is kept.
pub(super) struct ExactPreview {
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
pub(super) enum Source {
    Filtered(Box<Layer>),
    Pending(JoinHandle<Option<Exact>>),
    Filter,
}

pub(super) struct CommitJob {
    handle: JoinHandle<Result<Committed, Error>>,
    source_state: (u64, u64, u64, u64, Option<u64>),
    last_filter: Option<(String, Vec<f64>)>,
    label: String,
}

/// A worker's commit: the new document and history, and what to refresh.
pub(super) struct Committed {
    doc: Document,
    history: History,
    refresh: WorkerRefresh,
    /// The effect's pre- and post-effect layers, for `Edit > Fade`.
    fade: Option<FadeSnapshot>,
}

pub(super) enum WorkerRefresh {
    Region {
        x0: i32,
        y0: i32,
        clipped: PsdRect,
        srgb: PixelBuffer,
    },
    Everywhere,
    Nothing,
}

/// The layer at `path` of `doc` with `op` applied.
pub(super) fn apply_op(
    doc: &Document,
    path: &str,
    op: &ActiveOp,
    mask: Option<&Coverage>,
    gpu: bool,
) -> Result<Layer, Error> {
    let mut layer = pictura_render::resolve_path(doc, path)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArg, "the active layer no longer exists"))?
        .clone();
    match op {
        ActiveOp::Filter(filter) => {
            pictura_render::apply_filter(&mut layer, filter, mask, gpu).map_err(commit_error)?;
        }
        ActiveOp::Filters(filters) => {
            for filter in filters {
                pictura_render::apply_filter(&mut layer, filter, mask, gpu)
                    .map_err(commit_error)?;
            }
        }
        ActiveOp::Adjustment(adjustment) => {
            let rect = layer.rect;
            let mode = pictura_render::document_color_mode(doc).into();
            pictura_render::apply_adjustment_region_in_mode(
                &mut layer, adjustment, mask, rect, None, mode,
            )
            .map_err(commit_error)?;
        }
        ActiveOp::ApplyImage(params) => {
            let rect = layer.rect;
            let target = pictura_render::ApplyImageTarget {
                origin: (0, 0),
                gray: false,
                native: pictura_render::apply_image_native(doc, path).map_err(commit_error)?,
            };
            pictura_render::apply_image_region(&mut layer, params, mask, rect, target)
                .map_err(commit_error)?;
        }
    }
    Ok(layer)
}

fn commit_error(error: FilterError) -> Error {
    let code = match &error {
        FilterError::Unsupported(_) => ErrorCode::Capability,
        FilterError::InvalidParams(_) | FilterError::Locked => ErrorCode::InvalidArg,
    };
    Error::with_source(code, error.to_string(), error)
}

/// The layer at `path` of `doc` with `filters` applied in order (exact preview).
pub(super) fn filter_layer(
    doc: &Document,
    path: &str,
    filters: &[Filter],
    mask: Option<&Coverage>,
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

pub(super) fn exact(
    mut doc: Document,
    request: &Request,
    mask: Option<Coverage>,
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
pub(super) fn advance(rust: &mut PictureViewRust) {
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
    let delay = rust.filter_jobs.exact_delay_for_test;
    let handle = std::thread::spawn(move || {
        let result = exact(doc, &job, mask, gpu);
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        result
    });
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
    // A Smart Object previews its chain on the canvas instead.
    if crate::cxxqt_object::helpers::mask_editing(rust)
        || super::super::smart_apply::on_smart_object(rust)
    {
        return false;
    }
    let Some(path) = rust.layer_target.single().map(str::to_string) else {
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
    commit_op_start(rust, ActiveOp::Filters(filters), label, last_filter)
}

/// Start a commit of `op` on a worker; `false` with `filter_error` set when
/// refused.
pub(in crate::cxxqt_object) fn commit_op_start(
    rust: &mut PictureViewRust,
    op: ActiveOp,
    label: String,
    last_filter: Option<(String, Vec<f64>)>,
) -> bool {
    rust.filter_jobs.commit_status = ErrorCode::InvalidArg.status();
    rust.filter_error = None;
    if rust.modal_transform_blocks_edit() {
        rust.filter_error = Some("a transform is pending".to_string());
        return false;
    }
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
    if let (Some(preview), Some(live)) = (
        rust.session.live.take_filter_preview(),
        rust.session.doc.as_mut(),
    ) {
        if let Some(layer) = pictura_render::resolve_path_mut(live, &preview.layer_path) {
            preview.baseline.restore(layer);
        }
    }
    rust.canvas_proxy = None;
    let source = match &op {
        ActiveOp::Filters(filters) => match rust.filter_jobs.preview.take() {
            Some(ExactPreview {
                done: Some((request, exact)),
                ..
            }) if request.path == path && request.filters == *filters => {
                Source::Filtered(Box::new(exact.layer))
            }
            Some(ExactPreview {
                running: Some((request, handle)),
                ..
            }) if request.path == path && request.filters == *filters => Source::Pending(handle),
            _ => Source::Filter,
        },
        _ => Source::Filter,
    };
    let selection = rust.session.selection.clone();
    let target = rust.layer_target.clone();
    let mut history = rust.session.history.clone();
    let gpu = rust.gpu_compute;
    let delay = rust.filter_jobs.commit_delay_for_test;
    let job_label = label.clone();
    let panic_for_test = std::mem::take(&mut rust.filter_jobs.panic_for_test);
    history.panic_after_truncation_for_test = panic_for_test == 2;
    let handle = std::thread::Builder::new().spawn(move || {
        // The worker owns this document and a separate history index over shared tiles.
        let committed = guard(AssertUnwindSafe(|| {
            commit(
                doc,
                history,
                target,
                &path,
                op,
                source,
                mask,
                &selection,
                &label,
                gpu,
                panic_for_test == 1,
            )
        }));
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        committed
    });
    let Ok(handle) = handle else {
        rust.filter_jobs.commit_status = ErrorCode::Host.status();
        rust.filter_error = Some("could not start the filter worker".to_string());
        return false;
    };
    rust.filter_jobs.commit_status = ErrorCode::Ok.status();
    rust.filter_jobs.commit = Some(CommitJob {
        handle,
        last_filter,
        source_state: commit_source_state(rust),
        label: job_label,
    });
    true
}

fn commit_source_state(rust: &PictureViewRust) -> (u64, u64, u64, u64, Option<u64>) {
    (
        rust.session.document_generation,
        rust.session.content_revision,
        rust.non_preview_revision,
        rust.session.history.revision(),
        rust.session.history.current_id(),
    )
}

#[allow(clippy::too_many_arguments)]
fn commit(
    mut doc: Document,
    mut history: History,
    target: LayerTarget,
    path: &str,
    op: ActiveOp,
    source: Source,
    mask: Option<Coverage>,
    selection: &Option<Selection>,
    label: &str,
    gpu: bool,
    panic_in_engine_for_test: bool,
) -> Result<Committed, Error> {
    let original = pictura_render::resolve_path(&doc, path).cloned();
    let layer = match source {
        Source::Filtered(layer) => Ok(*layer),
        Source::Pending(handle) => match handle.join() {
            Ok(Some(exact)) => Ok(exact.layer),
            Ok(None) => apply_op(&doc, path, &op, mask.as_ref(), gpu),
            Err(payload) => {
                std::mem::forget(payload);
                Err(Error::new(
                    ErrorCode::Panic,
                    "the filter stopped unexpectedly",
                ))
            }
        },
        Source::Filter => apply_op(&doc, path, &op, mask.as_ref(), gpu),
    }?;
    if panic_in_engine_for_test {
        panic!("injected panic in adjustment engine work");
    }
    let (rect, effects) = (layer.rect, layer_has_effects(&layer));
    let slot = pictura_render::resolve_path_mut(&mut doc, path)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArg, "the active layer no longer exists"))?;
    *slot = layer;
    let fade = original
        .zip(pictura_render::resolve_path(&doc, path).cloned())
        .map(|(original, effected)| FadeSnapshot {
            path: path.to_string(),
            label: label.to_string(),
            original,
            effected,
        });
    // As `refresh_region` / `recomposite` do on the UI thread: a filter
    // changes only its layer's rect unless a layer effect spills past it.
    let refresh = if effects {
        DocumentComposite::new(&mut doc).refresh(pictura_render::CompositeScope::Full, gpu);
        WorkerRefresh::Everywhere
    } else {
        match clamp_region(rect, doc.width, doc.height) {
            Some((x0, y0, w, h)) => {
                let buffer = pictura_render::composite(pictura_render::CompositeRequest::region(
                    &doc, rect, gpu,
                ))
                .0;
                let srgb = pictura_codec::buffer_to_srgb(&doc, &buffer).into_owned();
                DocumentComposite::new(&mut doc).patch(&buffer, x0, y0);
                let clipped = PsdRect {
                    top: y0,
                    left: x0,
                    bottom: y0 + h as i32,
                    right: x0 + w as i32,
                };
                WorkerRefresh::Region {
                    x0,
                    y0,
                    clipped,
                    srgb,
                }
            }
            None => WorkerRefresh::Nothing,
        }
    };
    history.capture_live(&mut doc, selection, &target, label);
    Ok(Committed {
        doc,
        history,
        refresh,
        fade,
    })
}

pub(super) fn commit_filters(
    mut view: Pin<&mut PictureView>,
    filters: Vec<Filter>,
    label: String,
    last_filter: Option<(String, Vec<f64>)>,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    if !crate::cxxqt_object::helpers::mask_editing(view.rust()) {
        let mut rust = view.as_mut().rust_mut();
        return commit_start(&mut rust, filters, label, last_filter);
    }
    // A layer mask is a mask-sized plane; filter it here.
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_op_active_region(&mut rust, &ActiveOp::Filters(filters), true, None, false)
    };
    let Some(region) = region else {
        return false;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    if let Some((kind, params)) = &last_filter {
        view.as_mut().rust_mut().session.history_log_detail =
            crate::history_log::filter_detail(kind, params);
    }
    view.as_mut().record(&label);
    if last_filter.is_some() {
        view.as_mut().rust_mut().last_filter = last_filter;
    }
    true
}

pub(super) fn filter_commit_start(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    label: &QString,
) -> bool {
    let started = filter_commit_start_inner(view.as_mut(), kind, params, label);
    refuse_commit(view, started)
}

pub(super) fn refuse_commit(mut view: Pin<&mut PictureView>, started: bool) -> bool {
    if !started {
        {
            let mut rust = view.as_mut().rust_mut();
            if rust.filter_jobs.commit_status.is_ok() {
                rust.filter_jobs.commit_status = ErrorCode::InvalidArg.status();
            }
            let _ = cancel_filter_preview(&mut rust);
        }
        view.as_mut().recomposite();
    }
    started
}

pub(super) fn filter_commit_start_inner(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    label: &QString,
) -> bool {
    view.as_mut().rust_mut().filter_jobs.commit_status = ErrorCode::Ok.status();
    let kind = kind.to_string();
    if let Some(reason) = super::super::depth_refusal(&kind, view.rust().session.doc.as_ref()) {
        view.as_mut().rust_mut().filter_error = Some(reason);
        return false;
    }
    let params: Vec<f64> = params.into_iter().copied().collect();
    // On a Smart Object the filter becomes a Smart Filter, synchronously.
    if let Some(applied) =
        super::super::smart_apply::route_commit(view.as_mut(), &kind, &params, &label.to_string())
    {
        return applied;
    }
    let params = super::super::super::filter_map::commit_params(&kind, &params);
    let params = super::super::lens_blur_depths::commit_slots(&kind, view.rust(), &params);
    let Some(filter) = filter_from_kind_params(&kind, &params) else {
        view.as_mut().rust_mut().filter_error = Some(format!(
            "filter '{kind}' is not recognised or its parameters are wrong"
        ));
        return false;
    };
    commit_filters(view, vec![filter], label.to_string(), Some((kind, params)))
}

pub(super) fn filter_stack_commit_start(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    label: &QString,
) -> bool {
    let started = filter_stack_commit_start_inner(view.as_mut(), kinds, params, label);
    refuse_commit(view, started)
}

pub(super) fn filter_stack_commit_start_inner(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    label: &QString,
) -> bool {
    view.as_mut().rust_mut().filter_jobs.commit_status = ErrorCode::Ok.status();
    let params: Vec<f64> = params.into_iter().copied().collect();
    let kind_list: Vec<String> = QList::<QString>::from(kinds)
        .iter()
        .map(|k| k.to_string())
        .collect();
    if let Some(applied) = super::super::smart_apply::route_stack(
        view.as_mut(),
        &kind_list,
        &params,
        true,
        &label.to_string(),
    ) {
        return applied;
    }
    match resolve_stack(kinds, &params) {
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
    /// Drop every pending preview and commit. Detached workers finish unobserved.
    pub(in crate::cxxqt_object) fn cancel_all(&mut self) {
        self.preview = None;
        if self.commit.take().is_some() {
            self.commit_status = ErrorCode::Cancelled.status();
        }
    }

    /// Whether a started commit is still running on its worker.
    pub(in crate::cxxqt_object) fn commit_running(&self) -> bool {
        self.commit
            .as_ref()
            .is_some_and(|job| !job.handle.is_finished())
    }
}

pub(super) fn filter_commit_running(view: &PictureView) -> bool {
    view.rust().filter_jobs.commit_running()
}

/// Wait for the started commit and swap its document and history in, patching
/// level 0 for a bounded refresh. The presenter owns the rebuild, dirty,
/// revision, and notification. `Ok(None)` when no commit was running; `Err`
/// (with `filter_error` set) when the worker refused.
pub(in crate::cxxqt_object) fn commit_finish(
    rust: &mut PictureViewRust,
) -> Result<Option<Presentation>, ()> {
    let Some(job) = rust.filter_jobs.commit.take() else {
        return Ok(None);
    };
    if commit_source_state(rust) != job.source_state {
        rust.filter_jobs.commit_status = ErrorCode::Cancelled.status();
        return Err(());
    }
    let completed = job.handle.join().unwrap_or_else(|payload| {
        std::mem::forget(payload);
        Err(Error::new(
            ErrorCode::Panic,
            "the filter stopped unexpectedly",
        ))
    });
    let committed = match completed {
        Ok(committed) => committed,
        Err(error) => {
            rust.filter_jobs.commit_status = error.code().status();
            rust.filter_error = Some(error.detail().to_owned());
            return Err(());
        }
    };
    rust.filter_jobs.commit_status = ErrorCode::Ok.status();
    rust.session.history = committed.history;
    let refresh = committed.refresh;
    rust.session.doc.set(Some(committed.doc));
    let detail = job
        .last_filter
        .as_ref()
        .and_then(|(kind, params)| crate::history_log::filter_detail(kind, params));
    rust.session.history_log.edit(&job.label, detail.as_deref());
    if job.last_filter.is_some() {
        rust.last_filter = job.last_filter;
    }
    super::super::super::fade::clear(rust);
    if let Some(snapshot) = committed.fade {
        super::super::super::fade::record_snapshot(rust, snapshot);
    }
    Ok(Some(match refresh {
        WorkerRefresh::Region {
            x0,
            y0,
            clipped,
            srgb,
        } => {
            rust.apply_refreshed_region(x0, y0, clipped, srgb);
            Presentation {
                regions: vec![clipped],
                rebuild: Rebuild::None,
                notify: false,
                dirty: Dirty::Set(true),
                revision: true,
                declaration: None,
            }
        }
        WorkerRefresh::Everywhere => Presentation {
            rebuild: Rebuild::Sync,
            notify: true,
            dirty: Dirty::Set(true),
            revision: true,
            ..Presentation::none()
        },
        WorkerRefresh::Nothing => Presentation {
            notify: true,
            dirty: Dirty::Set(true),
            revision: true,
            ..Presentation::none()
        },
    }))
}

#[cfg(test)]
pub(in crate::cxxqt_object) fn test_commit_finish(
    rust: &mut PictureViewRust,
) -> Option<Presentation> {
    let doc = rust.session.doc.snapshot()?;
    let history = rust.session.history.clone();
    let handle = std::thread::spawn(move || {
        Ok(Committed {
            doc,
            history,
            refresh: WorkerRefresh::Nothing,
            fade: None,
        })
    });
    rust.filter_jobs.commit = Some(CommitJob {
        handle,
        last_filter: None,
        source_state: commit_source_state(rust),
        label: "Filter".into(),
    });
    commit_finish(rust).ok().flatten()
}

pub(super) fn filter_commit_finish(mut view: Pin<&mut PictureView>) -> bool {
    let landed = {
        let mut rust = view.as_mut().rust_mut();
        commit_finish(&mut rust)
    };
    match landed {
        Err(()) => {
            view.as_mut().recomposite();
            return false;
        }
        Ok(Some(presentation)) => view.as_mut().present(presentation),
        Ok(None) => {}
    }
    view.rust().filter_jobs.commit_status.is_ok()
}

pub(super) fn filter_commit_status(view: &PictureView) -> u32 {
    view.rust().filter_jobs.commit_status.raw()
}

pub(super) fn filter_set_commit_panic_for_test(mut view: Pin<&mut PictureView>, stage: i32) {
    view.as_mut().rust_mut().filter_jobs.panic_for_test = stage;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn filter_exact_preview_start(
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

pub(super) fn filter_exact_preview_poll(mut view: Pin<&mut PictureView>) -> QImage {
    let mut rust = view.as_mut().rust_mut();
    exact_preview_poll(&mut rust).map_or_else(QImage::default, |picture| {
        premultiplied_display_image(&picture)
    })
}

pub(super) fn filter_exact_preview_busy(mut view: Pin<&mut PictureView>) -> bool {
    let mut rust = view.as_mut().rust_mut();
    exact_preview_busy(&mut rust)
}

pub(super) fn filter_exact_preview_cancel(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().filter_jobs.preview = None;
}

pub(super) fn filter_exact_preview_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().filter_jobs.exact_delay_for_test = ms;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn filter_section_image(
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
