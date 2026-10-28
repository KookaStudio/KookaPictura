//! Image > Adjustments dialog previews and applies off the GUI thread.
//!
//! A preview worker adjusts a copy-on-write document clone and returns the
//! adjusted layer plus the composited changed region. The GUI installs the layer
//! and the region when the latest request lands; the live document is untouched
//! until then. A newer request supersedes an older one. OK uses the shared
//! filter commit path with an adjustment op.

use super::super::document_session::{FilterPreview, PreviewBaseline, SessionKind};
use super::super::filter_tools::jobs::commit_op_start;
use super::super::helpers::{active_mask_target, clamp_region, layer_has_effects};
use super::super::helpers_composite::{
    current_buffer, patch_composite_region, selection_to_mask, store_composite,
};
use super::super::impl_filters::{cancel_filter_preview, pixel_filter_target, ActiveOp};
use super::super::qobject::PictureView;
use super::super::state::PictureViewRust;
use super::decode;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{layer_pixel_locked, Document, Layer, PsdRect};
use pictura_render::Adjustment;
use pictura_select::Selection;
use std::thread::JoinHandle;
use std::time::Duration;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start previewing `block` on the active pixel layer. False for a
        /// layer-mask target, no document, or an unknown adjustment.
        fn image_adjust_preview_start(view: Pin<&mut PictureView>, block: &[u8]) -> bool;

        /// Install the latest landed preview's layer and composited region.
        /// True when one landed.
        fn image_adjust_preview_poll(view: Pin<&mut PictureView>) -> bool;

        /// Whether the latest preview is still being computed.
        fn image_adjust_preview_busy(view: Pin<&mut PictureView>) -> bool;

        /// Drop the preview and restore the pre-dialog pixels.
        fn image_adjust_preview_cancel(view: Pin<&mut PictureView>);

        /// Hold a finished preview for `ms` milliseconds (test seam).
        fn image_adjust_preview_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// Start applying `block` as one history state named `label` on a
        /// worker. False when refused.
        fn image_adjust_apply_start(
            view: Pin<&mut PictureView>,
            block: &[u8],
            label: &QString,
        ) -> bool;

        /// Hold a finished apply for `ms` milliseconds (test seam).
        fn image_adjust_apply_set_delay(view: Pin<&mut PictureView>, ms: u64);
    }
}

/// The adjustment dialog work a document has in flight.
#[derive(Default)]
pub(in crate::cxxqt_object) struct AdjustJobs {
    preview: Option<AdjustPreview>,
    /// Test seam: hold a finished preview before handing it over.
    pub(in crate::cxxqt_object) delay_for_test: u64,
}

impl AdjustJobs {
    /// Drop the pending preview; a running worker finishes unobserved.
    pub(in crate::cxxqt_object) fn cancel(&mut self) {
        self.preview = None;
    }
}

#[derive(Clone, PartialEq)]
struct AdjustRequest {
    path: String,
    adjustment: Adjustment,
}

struct AdjustPreview {
    latest: AdjustRequest,
    running: Option<(AdjustRequest, JoinHandle<Option<AdjustResult>>)>,
    done: Option<(AdjustRequest, AdjustResult)>,
    failed: Option<AdjustRequest>,
    delivered: bool,
}

impl AdjustPreview {
    fn settled(&self) -> bool {
        self.done
            .as_ref()
            .is_some_and(|(request, _)| *request == self.latest)
            || self.failed.as_ref() == Some(&self.latest)
    }
}

struct AdjustResult {
    layer: Layer,
    refresh: AdjustRefresh,
}

pub(in crate::cxxqt_object) enum AdjustRefresh {
    Region {
        x0: i32,
        y0: i32,
        rect: PsdRect,
        srgb: pictura_core::PixelBuffer,
        composite: pictura_core::PixelBuffer,
    },
    Everywhere {
        composite: pictura_core::PixelBuffer,
    },
    Nothing,
}

/// The pre-preview document and the selection, for a worker.
fn preview_sources(rust: &PictureViewRust) -> Option<(Document, Option<Selection>)> {
    let mut doc = rust.session.doc.clone()?;
    if let Some(preview) = rust.session.live.filter_preview() {
        if let Some(layer) = pictura_render::resolve_path_mut(&mut doc, &preview.layer_path) {
            *layer = preview.baseline.original(layer).into_owned();
        }
    }
    Some((doc, rust.session.selection.clone()))
}

/// Adjust one layer of a cloned document and composite its changed rect.
fn adjust_preview(
    mut doc: Document,
    request: AdjustRequest,
    selection: Option<Selection>,
    gpu: bool,
) -> Option<AdjustResult> {
    let mask = selection
        .as_ref()
        .map(|selection| selection_to_mask(selection, &doc));
    let layer = pictura_render::resolve_path_mut(&mut doc, &request.path)?;
    if layer.is_group || layer.adjustment.is_some() || layer_pixel_locked(layer) {
        return None;
    }
    let rect = layer.rect;
    pictura_render::apply_adjustment_region(layer, &request.adjustment, mask.as_ref(), rect)
        .ok()?;
    let layer = layer.clone();
    let refresh = if layer_has_effects(&layer) {
        let rendered = current_buffer(&doc, gpu);
        store_composite(&mut doc, &rendered);
        AdjustRefresh::Everywhere {
            composite: rendered,
        }
    } else {
        match clamp_region(rect, doc.width, doc.height) {
            Some((x0, y0, w, h)) => {
                let buffer = pictura_render::composite_region_active(&doc, rect, gpu).0;
                let srgb = pictura_codec::buffer_to_srgb(&doc, &buffer).into_owned();
                patch_composite_region(&mut doc, &buffer, x0, y0);
                AdjustRefresh::Region {
                    x0,
                    y0,
                    rect: PsdRect {
                        top: y0,
                        left: x0,
                        bottom: y0 + h as i32,
                        right: x0 + w as i32,
                    },
                    srgb,
                    composite: buffer,
                }
            }
            None => AdjustRefresh::Nothing,
        }
    };
    Some(AdjustResult { layer, refresh })
}

/// Collect a finished worker and start the latest request when none runs.
fn advance(rust: &mut PictureViewRust) {
    let gpu = rust.gpu_compute;
    let Some(preview) = rust.adjust_jobs.preview.as_mut() else {
        return;
    };
    if preview
        .running
        .as_ref()
        .is_some_and(|(_, handle)| handle.is_finished())
    {
        let (request, handle) = preview.running.take().expect("checked above");
        match handle.join().ok().flatten() {
            Some(result) if request == preview.latest => preview.done = Some((request, result)),
            Some(_) => {}
            None => preview.failed = Some(request),
        }
    }
    if preview.running.is_some() || preview.settled() {
        return;
    }
    let request = preview.latest.clone();
    let Some((doc, selection)) = preview_sources(rust) else {
        return;
    };
    let job = request.clone();
    let delay = rust.adjust_jobs.delay_for_test;
    let handle = std::thread::spawn(move || {
        let result = adjust_preview(doc, job, selection, gpu);
        if delay > 0 {
            std::thread::sleep(Duration::from_millis(delay));
        }
        result
    });
    if let Some(preview) = rust.adjust_jobs.preview.as_mut() {
        preview.running = Some((request, handle));
    }
}

/// Start or supersede a preview of `adjustment`.
pub(in crate::cxxqt_object) fn preview_start(
    rust: &mut PictureViewRust,
    adjustment: Adjustment,
) -> bool {
    rust.filter_error = None;
    if active_mask_target(rust).is_some() {
        return false;
    }
    if !rust.slot_allows(SessionKind::FilterPreview) {
        rust.filter_error = Some("another session is live".to_string());
        return false;
    }
    let Some(path) = pixel_filter_target(rust) else {
        return false;
    };
    let request = AdjustRequest { path, adjustment };
    match rust.adjust_jobs.preview.as_mut() {
        Some(preview) if preview.latest == request => {
            if preview.failed.is_some() {
                preview.failed = None;
                preview.delivered = false;
            }
        }
        Some(preview) => {
            preview.latest = request;
            preview.delivered = false;
            preview.done = None;
            preview.failed = None;
        }
        None => {
            rust.adjust_jobs.preview = Some(AdjustPreview {
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

/// Install the latest landed preview; `None` when nothing is ready.
pub(in crate::cxxqt_object) fn preview_poll(rust: &mut PictureViewRust) -> Option<AdjustRefresh> {
    advance(rust);
    let preview = rust.adjust_jobs.preview.as_mut()?;
    if preview.delivered {
        return None;
    }
    let (request, result) = preview.done.take()?;
    if request != preview.latest {
        return None;
    }
    preview.delivered = true;
    let path = request.path;
    let baseline = match rust.session.live.take_filter_preview() {
        Some(existing) if existing.layer_path == path => existing.baseline,
        _ => {
            let live = rust.session.doc.as_ref()?;
            let layer = pictura_render::resolve_path(live, &path)?;
            PreviewBaseline::save(layer, None)
        }
    };
    let doc = rust.session.doc.as_mut()?;
    *pictura_render::resolve_path_mut(doc, &path)? = result.layer;
    rust.session.live.set_filter_preview(FilterPreview {
        layer_path: path,
        baseline,
    });
    rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    Some(result.refresh)
}

pub(in crate::cxxqt_object) fn preview_busy(rust: &mut PictureViewRust) -> bool {
    advance(rust);
    rust.adjust_jobs
        .preview
        .as_ref()
        .is_some_and(|preview| !preview.settled())
}

pub(in crate::cxxqt_object) fn preview_cancel(
    rust: &mut PictureViewRust,
) -> Option<Option<PsdRect>> {
    rust.adjust_jobs.preview = None;
    cancel_filter_preview(rust)
}

fn image_adjust_preview_start(mut view: Pin<&mut PictureView>, block: &[u8]) -> bool {
    let Some(adjustment) = decode(block) else {
        return false;
    };
    {
        let mut rust = view.as_mut().rust_mut();
        preview_start(&mut rust, adjustment)
    }
}

fn image_adjust_preview_poll(mut view: Pin<&mut PictureView>) -> bool {
    let refresh = {
        let mut rust = view.as_mut().rust_mut();
        preview_poll(&mut rust)
    };
    let Some(refresh) = refresh else {
        return false;
    };
    match refresh {
        AdjustRefresh::Region {
            x0,
            y0,
            rect,
            srgb,
            composite,
        } => {
            {
                let mut rust = view.as_mut().rust_mut();
                if let Some(doc) = rust.session.doc.as_mut() {
                    patch_composite_region(doc, &composite, x0, y0);
                }
                rust.apply_refreshed_region(x0, y0, rect, srgb);
            }
            view.as_mut()
                .region_damaged(rect.left, rect.top, rect.width(), rect.height());
        }
        AdjustRefresh::Everywhere { composite } => {
            {
                let mut rust = view.as_mut().rust_mut();
                // reset_pyramid drops the live preview; keep its baseline so
                // Cancel can still restore the layer after the rebuild.
                let preview = rust.session.live.take_filter_preview();
                if let Some(doc) = rust.session.doc.as_mut() {
                    store_composite(doc, &composite);
                }
                rust.reset_pyramid();
                if let Some(preview) = preview {
                    rust.session.live.set_filter_preview(preview);
                }
            }
            view.as_mut().changed();
        }
        AdjustRefresh::Nothing => {}
    }
    true
}

fn image_adjust_preview_busy(mut view: Pin<&mut PictureView>) -> bool {
    preview_busy(&mut view.as_mut().rust_mut())
}

fn image_adjust_preview_cancel(mut view: Pin<&mut PictureView>) {
    let region = {
        let mut rust = view.as_mut().rust_mut();
        preview_cancel(&mut rust)
    };
    let Some(region) = region else {
        return;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
}

fn image_adjust_preview_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().adjust_jobs.delay_for_test = ms;
}

fn image_adjust_apply_start(
    mut view: Pin<&mut PictureView>,
    block: &[u8],
    label: &QString,
) -> bool {
    let Some(adjustment) = decode(block) else {
        return false;
    };
    if active_mask_target(view.rust()).is_some() {
        let label = label.to_string();
        return super::run(view, adjustment, None, Some(label.as_str()));
    }
    let mut rust = view.as_mut().rust_mut();
    commit_op_start(
        &mut rust,
        ActiveOp::Adjustment(adjustment),
        label.to_string(),
        None,
    )
}

fn image_adjust_apply_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().filter_jobs.commit_delay_for_test = ms;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, LayerStack};

    fn view() -> PictureViewRust {
        let mut doc = Document::new(32, 24, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![pixel_layer("px", 32, 24, (40, 80, 120))]);
        let mut rust = PictureViewRust::default();
        rust.session.doc = Some(doc);
        rust.layer_target = crate::layer_target::LayerTarget::only("0");
        rust.gpu_compute = false;
        rust
    }

    #[test]
    fn a_preview_lands_and_cancel_restores() {
        let mut rust = view();
        let before = rust.session.doc.as_ref().unwrap().layers[0].channels[0]
            .data
            .to_vec();
        assert!(preview_start(&mut rust, Adjustment::Invert));
        while preview_busy(&mut rust) {
            std::thread::yield_now();
        }
        assert!(preview_poll(&mut rust).is_some());
        assert_ne!(
            rust.session.doc.as_ref().unwrap().layers[0].channels[0].data,
            before
        );
        let _ = preview_cancel(&mut rust);
        assert_eq!(
            rust.session.doc.as_ref().unwrap().layers[0].channels[0].data,
            before
        );
        assert!(!rust.session.dirty);
    }

    #[test]
    fn a_newer_request_supersedes_the_older_one() {
        let mut rust = view();
        assert!(preview_start(&mut rust, Adjustment::Invert));
        assert!(preview_start(&mut rust, Adjustment::Desaturate));
        while preview_busy(&mut rust) {
            std::thread::yield_now();
        }
        assert!(preview_poll(&mut rust).is_some());
        let preview = rust.adjust_jobs.preview.as_ref().unwrap();
        assert!(preview.delivered);
    }
}
