//! Content-Aware Scale session and generation-checked worker results.

use super::super::document_session::TransformMode;
use super::super::edit::Edit;
use super::super::qobject::PictureView;
use super::geometry::session_quad;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{BitDepth, Document, PsdRect};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

static PREVIEW_WORKER_BUSY: AtomicBool = AtomicBool::new(false);

type PreviewResult = (u64, Option<(Document, Vec<u8>, PsdRect)>);

#[derive(Clone, Copy, PartialEq, Eq)]
struct Request {
    rect: PsdRect,
    amount: u8,
    protect: i32,
}

pub(in crate::cxxqt_object) struct ContentAwareState {
    source: Document,
    original: Document,
    running: bool,
    amount: u8,
    protect: i32,
    generation: u64,
    request: Option<Request>,
    cancellation: Arc<AtomicBool>,
    tx: mpsc::Sender<PreviewResult>,
    rx: mpsc::Receiver<PreviewResult>,
    result: Option<Document>,
    pending: bool,
}

impl ContentAwareState {
    pub(in crate::cxxqt_object) fn new(source: Document, original: Document) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            source,
            original,
            running: false,
            amount: 100,
            protect: -1,
            generation: 0,
            request: None,
            cancellation: Arc::new(AtomicBool::new(false)),
            tx,
            rx,
            result: None,
            pending: false,
        }
    }
}
impl ContentAwareState {
    pub(super) fn original(&self) -> &Document {
        &self.original
    }
}
impl Drop for ContentAwareState {
    fn drop(&mut self) {
        self.cancellation.store(true, Ordering::Relaxed);
    }
}

pub(super) fn content_aware_active(view: &PictureView) -> bool {
    view.rust()
        .session
        .live
        .transform()
        .is_some_and(|s| s.content_aware.is_some())
}

pub(super) fn content_aware_can_begin(view: &PictureView, path: &QString) -> bool {
    let rust = view.rust();
    if rust.session.live.transform().is_some()
        || rust.mask_edit_target.is_some()
        || !rust.slot_allows(super::super::document_session::SessionKind::Transform)
        || rust.layer_target.single().is_none()
    {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let Some(layer) = pictura_render::resolve_path(doc, &path.to_string()) else {
        return false;
    };
    let has_selection = rust
        .session
        .selection
        .as_ref()
        .is_some_and(|s| pictura_render::plane_has_coverage(&s.data, doc.width, doc.height));
    let depth = doc.document_bit_depth();
    matches!(
        depth,
        BitDepth::Eight | BitDepth::Sixteen | BitDepth::ThirtyTwo
    ) && layer.visible
        && !layer.background
        && !layer.is_group()
        && !layer.has_adjustment_content()
        && !layer.is_type()
        && !layer.is_video_or_3d()
        && !pictura_render::is_shape_layer(layer)
        && pictura_render::decode_layer_fill(layer).is_none()
        && layer.smart_object().is_none()
        && layer.mask.is_none()
        && layer.vector_mask.is_none()
        && !(doc.source.mode().is_some() && layer.source_channels.is_none())
        && layer.lock.bits() == 0
        && !(has_selection && depth != BitDepth::Eight)
        && layer.rect.width() > 0
        && layer.rect.height() > 0
}

pub(super) fn content_aware_begin(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    if !content_aware_can_begin(view.as_ref().get_ref(), path) {
        return false;
    }
    view.as_mut()
        .begin_transform_session(path, TransformMode::Scale, true)
}

fn target_rect(session: &super::super::document_session::TransformSession) -> Option<PsdRect> {
    let q = session_quad(session);
    if session.scale_x <= 0.0
        || session.scale_y <= 0.0
        || q.iter().any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return None;
    }
    let (left, top, right, bottom) = (
        q[0].0.round(),
        q[0].1.round(),
        q[2].0.round(),
        q[2].1.round(),
    );
    if [left, top, right, bottom]
        .iter()
        .any(|x| *x < i32::MIN as f64 || *x > i32::MAX as f64)
        || right <= left
        || bottom <= top
        || right - left > i32::MAX as f64
        || bottom - top > i32::MAX as f64
    {
        return None;
    }
    Some(PsdRect {
        left: left as i32,
        top: top as i32,
        right: right as i32,
        bottom: bottom as i32,
    })
}

fn protection(
    doc: &Document,
    rect: PsdRect,
    channel: i32,
    cancel: &AtomicBool,
) -> Option<Option<Vec<u8>>> {
    if channel < 0 {
        return Some(None);
    }
    let channel = doc.channels.get(channel as usize)?;
    let (w, h) = (
        usize::try_from(rect.width()).ok()?,
        usize::try_from(rect.height()).ok()?,
    );
    let len = w.checked_mul(h)?;
    if len > 256 * 1024 * 1024 {
        return None;
    }
    let mut out = pictura_core::memory::try_buffer(len, 0u8, "content-aware protection").ok()?;
    for y in 0..h {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let sy = i64::from(rect.top) + y as i64;
        if sy < 0 || sy >= i64::from(doc.height) {
            continue;
        }
        for x in 0..w {
            let sx = i64::from(rect.left) + x as i64;
            if sx >= 0 && sx < i64::from(doc.width) {
                out[y * w + x] = channel
                    .data
                    .sample(sy as usize * doc.width as usize + sx as usize)
                    .unwrap_or(0);
            }
        }
    }
    Some(Some(out))
}

pub(super) fn content_aware_request(mut view: Pin<&mut PictureView>) {
    let request = view.rust().session.live.transform().and_then(|s| {
        let state = s.content_aware.as_ref()?;
        Some((target_rect(s)?, state.amount, state.protect))
    });
    let Some((dest, amount, protect_index)) = request else {
        if let Some(state) = view
            .as_mut()
            .rust_mut()
            .session
            .live
            .transform_mut()
            .and_then(|s| s.content_aware.as_mut())
        {
            state.cancellation.store(true, Ordering::Relaxed);
            state.generation = state.generation.wrapping_add(1);
            state.request = None;
            state.result = None;
            state.pending = false;
        }
        return;
    };
    let mut rust = view.as_mut().rust_mut();
    let Some(state) = rust
        .session
        .live
        .transform_mut()
        .and_then(|s| s.content_aware.as_mut())
    else {
        return;
    };
    let key = Request {
        rect: dest,
        amount,
        protect: protect_index,
    };
    if state.request == Some(key) {
        return;
    }
    state.cancellation.store(true, Ordering::Relaxed);
    state.cancellation = Arc::new(AtomicBool::new(false));
    state.generation = state.generation.wrapping_add(1);
    state.request = Some(key);
    state.pending = true;
    state.result = None;
    content_aware_launch(view);
}

fn content_aware_launch(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return;
    };
    let path = session.path.clone();
    let source_rect = session.orig_rect;
    let lifted = session.lifted.is_some();
    let Some(state) = session.content_aware.as_mut() else {
        return;
    };
    if !state.pending
        || state.running
        || state.request.is_none()
        || PREVIEW_WORKER_BUSY
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
    {
        return;
    }
    let request = state.request.expect("pending request");
    let mut doc = state.source.clone();
    let cancel = state.cancellation.clone();
    let generation = state.generation;
    let tx = state.tx.clone();
    state.running = true;
    if std::thread::Builder::new()
        .spawn(move || {
            test_gate::started(generation);
            test_gate::wait(generation);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mask = protection(&doc, source_rect, request.protect, &cancel)?;
                if cancel.load(Ordering::Relaxed)
                    || !pictura_render::content_aware_scale(
                        &mut doc,
                        &path,
                        request.rect,
                        pictura_render::ContentAwareOptions {
                            amount: request.amount,
                            protect: mask.as_deref(),
                        },
                        &cancel,
                    )
                {
                    return None;
                }
                let rect = pictura_render::resolve_path(&doc, &path)?.rect;
                let pixels = super::session::moving_layer_pixels(&doc, &path, rect, 0, false)?;
                let rgba = super::super::helpers_composite::buffer_to_rgba_bytes(&pixels);
                if cancel.load(Ordering::Relaxed)
                    || (lifted && !pictura_render::merge_lifted(&mut doc, &path))
                {
                    return None;
                }
                Some((doc, rgba, rect))
            }))
            .ok()
            .flatten();
            let _ = tx.send((generation, result));
            PREVIEW_WORKER_BUSY.store(false, Ordering::Release);
            test_gate::finished(generation);
        })
        .is_err()
    {
        PREVIEW_WORKER_BUSY.store(false, Ordering::Release);
        state.running = false;
        state.pending = false;
    }
}

pub(super) fn content_aware_poll(mut view: Pin<&mut PictureView>) -> bool {
    let result = {
        let mut rust = view.as_mut().rust_mut();
        let Some(state) = rust
            .session
            .live
            .transform_mut()
            .and_then(|s| s.content_aware.as_mut())
        else {
            return false;
        };
        let mut current = None;
        while let Ok((generation, result)) = state.rx.try_recv() {
            state.running = false;
            if generation == state.generation {
                state.pending = false;
                current = Some(result);
            }
        }
        current
    };
    content_aware_launch(view.as_mut());
    let Some(result) = result else {
        return false;
    };
    if let Some((doc, rgba, rect)) = result {
        let image = super::super::helpers_composite::rgba_image(rgba, rect.width(), rect.height());
        let mut rust = view.as_mut().rust_mut();
        rust.move_layer = Some(image);
        rust.move_x = rect.left;
        rust.move_y = rect.top;
        rust.transform_preview_level = 0;
        if let Some(state) = rust
            .session
            .live
            .transform_mut()
            .and_then(|s| s.content_aware.as_mut())
        {
            state.result = Some(doc);
        }
    }
    true
}

pub(super) fn content_aware_pending(view: &PictureView) -> bool {
    view.rust()
        .session
        .live
        .transform()
        .and_then(|s| s.content_aware.as_ref())
        .is_some_and(|s| s.pending)
}
pub(super) fn content_aware_ready(view: &PictureView) -> bool {
    view.rust().session.live.transform().is_some_and(|session| {
        session.content_aware.as_ref().is_some_and(|state| {
            let current = target_rect(session).map(|rect| Request {
                rect,
                amount: state.amount,
                protect: state.protect,
            });
            !state.pending
                && state.result.is_some()
                && state.request == current
                && current.is_some()
        })
    })
}
pub(super) fn content_aware_amount(view: &PictureView) -> i32 {
    view.rust()
        .session
        .live
        .transform()
        .and_then(|s| s.content_aware.as_ref())
        .map_or(100, |s| i32::from(s.amount))
}
pub(super) fn content_aware_protect(view: &PictureView) -> i32 {
    view.rust()
        .session
        .live
        .transform()
        .and_then(|s| s.content_aware.as_ref())
        .map_or(-1, |s| s.protect)
}
pub(super) fn content_aware_set_options(
    mut view: Pin<&mut PictureView>,
    amount: i32,
    protect: i32,
) {
    if let Some(state) = view
        .as_mut()
        .rust_mut()
        .session
        .live
        .transform_mut()
        .and_then(|s| s.content_aware.as_mut())
    {
        state.amount = amount.clamp(0, 100) as u8;
        state.protect = protect;
    }
    content_aware_request(view);
}

pub(super) fn content_aware_commit(mut view: Pin<&mut PictureView>) -> bool {
    content_aware_request(view.as_mut());
    if !content_aware_ready(view.as_ref().get_ref()) {
        return false;
    }
    let identity = view.rust().session.live.transform().is_some_and(|s| {
        s.content_aware
            .as_ref()
            .and_then(|state| state.request)
            .is_some_and(|request| request.rect == s.orig_rect)
    });
    if identity {
        view.as_mut().cancel_transform();
        return true;
    }
    let Some(mut session) = view.as_mut().rust_mut().session.live.take_transform() else {
        return false;
    };
    let Some(result) = session.content_aware.as_mut().and_then(|s| s.result.take()) else {
        return false;
    };
    {
        let mut rust = view.as_mut().rust_mut();
        rust.session.doc.set(Some(result));
        if session.lifted.is_some() {
            rust.session.set_selection_in_place(None);
        }
        rust.session.last_transform = None;
    }
    view.as_mut()
        .commit_transform_edit("Content-Aware Scale", true, |_| Edit::Full);
    if !view.rust().move_composite_pending() {
        view.as_mut().move_committed();
    }
    view.as_mut().clear_transform_preview();
    true
}

mod test_gate;
pub(super) use test_gate::{
    content_aware_test_finished, content_aware_test_generation, content_aware_test_hold,
    content_aware_test_release, content_aware_test_started,
};
