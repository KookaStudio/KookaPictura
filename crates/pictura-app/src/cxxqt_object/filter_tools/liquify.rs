//! Filter ▸ Liquify (#348): the dialog's editing session and its finished
//! meshes. The dialog edits a mesh over the active layer here and asks for
//! its view rendered at screen resolution; OK keeps the mesh and the
//! `liquify` kind's one slot names it by id, so the commit, Fade and Last
//! Filter run the ordinary filter path, as Displace's map does. Free
//! functions (their own bridge); the dialog is modal, so one session at a
//! time.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat, QString};
use pictura_core::{PixelBuffer, PsdRect};
use pictura_filters::{LiquifyBrush, LiquifyMesh, LiquifySource, LiquifyTool};

use super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::helpers_composite::{layer_buffer, rgba_image};
use super::super::qobject::PictureView;
use super::hidden_layer_buffer;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Open a session over the active layer. False when there is no
        /// unmasked pixel layer with colour data.
        fn liquify_begin(view: &PictureView) -> bool;

        /// The session's layer size, `[width, height]`; empty with no
        /// session.
        fn liquify_geometry() -> Vec<i32>;

        /// A `w`×`h` premultiplied view of the distorted layer whose pixel
        /// `(i, j)` shows layer point `(x + (i + 0.5) / zoom, y + (j + 0.5) /
        /// zoom)`, clear off the layer; null with no session.
        fn liquify_render(x: f64, y: f64, zoom: f64, w: i32, h: i32) -> QImage;

        /// Show Mesh: the source grid `spacing` layer pixels apart as the
        /// distortion draws it, traced where it can show inside the layer
        /// rectangle `(x0, y0)–(x1, y1)` and sampled every `step`. Flattened
        /// `x, y` pairs; a NaN pair ends each line.
        fn liquify_grid_lines(
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
            spacing: f64,
            step: f64,
        ) -> Vec<f64>;

        /// Show Backdrop's picture in the active layer's frame (its size, laid
        /// over it): an empty `path` is All Layers (the document with the
        /// active layer hidden), otherwise the layer at `path`. Null when
        /// unavailable.
        fn liquify_backdrop(view: Pin<&mut PictureView>, path: &QString) -> QImage;

        /// The brush the following strokes use. `tool` indexes Forward Warp,
        /// Reconstruct, Twirl, Pucker, Bloat, Push Left, Mirror, Turbulence,
        /// Freeze Mask, Thaw Mask.
        #[allow(clippy::too_many_arguments)]
        fn liquify_set_brush(
            tool: i32,
            size: f64,
            density: f64,
            pressure: f64,
            rate: f64,
            jitter: f64,
            reverse: bool,
        );

        /// Start a stroke at layer point `(x, y)` with stylus `pressure`
        /// (0..1). Each stroke call returns the layer rectangle it changed,
        /// `[x, y, w, h]`, or empty when nothing changed.
        fn liquify_stroke_begin(x: f64, y: f64, pressure: f64) -> Vec<i32>;

        /// Continue the stroke to `(x, y)`.
        fn liquify_stroke_to(x: f64, y: f64, pressure: f64) -> Vec<i32>;

        /// One Brush Rate tick with the brush held at the stroke's last point.
        fn liquify_hold() -> Vec<i32>;

        /// The Reconstruct button: restore unfrozen areas by `amount` percent.
        fn liquify_reconstruct(amount: f64);

        /// Restore All: drop every distortion, frozen areas included.
        fn liquify_restore_all();

        /// Mask None (0), Mask All (1), Invert All (2).
        fn liquify_mask(op: i32);

        /// Tint frozen areas with `(r, g, b)` in the view, or not.
        fn liquify_set_mask_overlay(show: bool, r: i32, g: i32, b: i32);

        /// Whether the mesh moves anything.
        fn liquify_distorted() -> bool;

        /// The id of the mesh the last OK kept (Load Last Mesh), or 0 when
        /// none is kept any more.
        fn liquify_last_mesh_id() -> i32;

        /// Replace the session's distortion with kept mesh `id`, stretched to
        /// fit this layer; the session's freeze mask stays. False when the id
        /// is not kept.
        fn liquify_load_mesh(id: i32) -> bool;

        /// End the session. With `keep` and a mesh that moves something, keep
        /// it (and remember it for Load Last Mesh) and return its id for the
        /// `liquify` slot; otherwise 0.
        fn liquify_finish(keep: bool) -> i32;
    }
}

/// ponytail: the eight most recent meshes stay in memory for the process; a
/// Last Filter that names an older one is refused, and none survive a
/// restart. Saving meshes to files (CS6's `.msh`, a closed format) is not
/// offered.
const KEPT: usize = 8;

struct Store {
    next: u32,
    last: u32,
    meshes: VecDeque<(u32, Arc<LiquifyMesh>)>,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    next: 1,
    last: 0,
    meshes: VecDeque::new(),
});

fn store() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn register(mesh: LiquifyMesh) -> u32 {
    let mut store = store();
    let id = store.next;
    store.next = store.next.wrapping_add(1).max(1);
    if store.meshes.len() == KEPT {
        store.meshes.pop_front();
    }
    store.meshes.push_back((id, Arc::new(mesh)));
    store.last = id;
    id
}

/// The mesh `id` names. Id 0 — a dialog's default — is an identity mesh; an
/// id no longer kept is `None`.
pub(in crate::cxxqt_object) fn liquify_mesh(id: u32) -> Option<Arc<LiquifyMesh>> {
    if id == 0 {
        return Some(Arc::new(LiquifyMesh::new(1, 1)));
    }
    store()
        .meshes
        .iter()
        .find(|(kept, _)| *kept == id)
        .map(|(_, mesh)| mesh.clone())
}

struct Session {
    mesh: LiquifyMesh,
    source: LiquifySource,
    brush: LiquifyBrush,
    last: Option<[f64; 3]>,
    mask: Option<[u8; 3]>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn with_session<R>(default: R, f: impl FnOnce(&mut Session) -> R) -> R {
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map_or(default, f)
}

fn rect_vec(r: Option<PsdRect>) -> Vec<i32> {
    r.map_or_else(Vec::new, |r| vec![r.left, r.top, r.width(), r.height()])
}

const TOOLS: [LiquifyTool; 10] = [
    LiquifyTool::ForwardWarp,
    LiquifyTool::Reconstruct,
    LiquifyTool::Twirl,
    LiquifyTool::Pucker,
    LiquifyTool::Bloat,
    LiquifyTool::PushLeft,
    LiquifyTool::Mirror,
    LiquifyTool::Turbulence,
    LiquifyTool::Freeze,
    LiquifyTool::Thaw,
];

fn liquify_begin(view: &PictureView) -> bool {
    let rust = view.rust();
    if active_mask_target(rust).is_some() {
        return false;
    }
    let Some(doc) = rust.doc.as_ref() else {
        return false;
    };
    let Some(layer) = active_pixel_layer(doc, rust.active_layer.as_deref()) else {
        return false;
    };
    let (w, h) = (layer.rect.width(), layer.rect.height());
    if w <= 0 || h <= 0 {
        return false;
    }
    let plane = w as usize * h as usize;
    // Planes are shared, not copied: the modal dialog never edits the layer.
    let channel = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id && c.data.len() == plane)
            .map(|c| c.data.clone())
    };
    let Some(c0) = channel(0) else {
        return false;
    };
    // A Grayscale layer carries one colour plane.
    let color = [
        c0.clone(),
        channel(1).unwrap_or_else(|| c0.clone()),
        channel(2).unwrap_or(c0),
    ];
    let session = Session {
        mesh: LiquifyMesh::new(w as u32, h as u32),
        source: LiquifySource::new(w as u32, h as u32, color, channel(-1)),
        brush: LiquifyBrush::new(LiquifyTool::ForwardWarp, 100.0),
        last: None,
        mask: None,
    };
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);
    true
}

fn liquify_geometry() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        vec![s.mesh.width() as i32, s.mesh.height() as i32]
    })
}

fn liquify_render(x: f64, y: f64, zoom: f64, w: i32, h: i32) -> QImage {
    if w <= 0 || h <= 0 {
        return QImage::default();
    }
    with_session(QImage::default(), |s| {
        let (wu, hu) = (w as usize, h as usize);
        let mut bytes = vec![0u8; wu * hu * 4];
        s.source
            .render(&s.mesh, [x, y], zoom, (wu, hu), &mut bytes, s.mask);
        // SAFETY: `bytes` is exactly w*h premultiplied RGBA8888 samples.
        unsafe { QImage::from_raw_bytes(bytes, w, h, QImageFormat::Format_RGBA8888_Premultiplied) }
    })
}

fn liquify_grid_lines(x0: f64, y0: f64, x1: f64, y1: f64, spacing: f64, step: f64) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        s.mesh
            .grid_lines([x0, y0, x1, y1], spacing, step)
            .into_iter()
            .flatten()
            .collect()
    })
}

fn liquify_backdrop(mut view: Pin<&mut PictureView>, path: &QString) -> QImage {
    let mut rust = view.as_mut().rust_mut();
    let gpu_compute = rust.gpu_compute;
    let Some(active) = rust.active_layer.clone() else {
        return QImage::default();
    };
    let Some(doc) = rust.doc.as_mut() else {
        return QImage::default();
    };
    let Some(frame) = active_pixel_layer(doc, Some(&active)).map(|l| l.rect) else {
        return QImage::default();
    };
    let path = path.to_string();
    let picture = if path.is_empty() {
        hidden_layer_buffer(doc, &active, gpu_compute)
            .map(|base| (pictura_codec::buffer_to_srgb(doc, &base).into_owned(), 0, 0))
    } else {
        active_pixel_layer(doc, Some(&path))
            .and_then(|layer| Some((layer_buffer(layer)?, layer.rect.left, layer.rect.top)))
    };
    let Some((buffer, left, top)) = picture else {
        return QImage::default();
    };
    let rgba = in_frame(&buffer, left - frame.left, top - frame.top, frame);
    rgba_image(rgba, frame.width(), frame.height())
}

/// `buffer` (gray, gray+alpha, RGB or RGBA planes) placed at `(dx, dy)` in a
/// clear straight-alpha RGBA8 image the size of `frame`.
fn in_frame(buffer: &PixelBuffer, dx: i32, dy: i32, frame: PsdRect) -> Vec<u8> {
    let (fw, fh) = (
        frame.width().max(0) as usize,
        frame.height().max(0) as usize,
    );
    let (bw, bh) = (buffer.width as usize, buffer.height as usize);
    let n = bw * bh;
    let channels = buffer.channels as usize;
    let mut out = vec![0u8; fw * fh * 4];
    for y in 0..fh {
        let sy = y as i64 - i64::from(dy);
        if sy < 0 || sy >= bh as i64 {
            continue;
        }
        for x in 0..fw {
            let sx = x as i64 - i64::from(dx);
            if sx < 0 || sx >= bw as i64 {
                continue;
            }
            let i = sy as usize * bw + sx as usize;
            let plane = |c: usize| buffer.data[c * n + i];
            let px = match channels {
                1 => [plane(0), plane(0), plane(0), 255],
                2 => [plane(0), plane(0), plane(0), plane(1)],
                3 => [plane(0), plane(1), plane(2), 255],
                _ => [plane(0), plane(1), plane(2), plane(3)],
            };
            out[(y * fw + x) * 4..(y * fw + x) * 4 + 4].copy_from_slice(&px);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn liquify_set_brush(
    tool: i32,
    size: f64,
    density: f64,
    pressure: f64,
    rate: f64,
    jitter: f64,
    reverse: bool,
) {
    with_session((), |s| {
        s.brush = LiquifyBrush {
            tool: TOOLS[(tool.max(0) as usize).min(TOOLS.len() - 1)],
            size,
            density,
            pressure,
            rate,
            jitter,
            reverse,
        };
    });
}

fn liquify_stroke_begin(x: f64, y: f64, pressure: f64) -> Vec<i32> {
    with_session(Vec::new(), |s| {
        let p = [x, y, pressure];
        s.last = Some(p);
        rect_vec(s.mesh.stroke_begin(&s.brush, p))
    })
}

fn liquify_stroke_to(x: f64, y: f64, pressure: f64) -> Vec<i32> {
    with_session(Vec::new(), |s| {
        let b = [x, y, pressure];
        let dirty = s.last.and_then(|a| s.mesh.stroke_to(&s.brush, a, b));
        s.last = Some(b);
        rect_vec(dirty)
    })
}

fn liquify_hold() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        rect_vec(s.last.and_then(|p| s.mesh.hold(&s.brush, p)))
    })
}

fn liquify_reconstruct(amount: f64) {
    with_session((), |s| s.mesh.reconstruct(amount));
}

fn liquify_restore_all() {
    with_session((), |s| s.mesh.restore_all());
}

fn liquify_mask(op: i32) {
    with_session((), |s| match op {
        0 => s.mesh.thaw_all(),
        1 => s.mesh.freeze_all(),
        _ => s.mesh.invert_freeze(),
    });
}

fn liquify_set_mask_overlay(show: bool, r: i32, g: i32, b: i32) {
    let byte = |v: i32| v.clamp(0, 255) as u8;
    with_session((), |s| s.mask = show.then(|| [byte(r), byte(g), byte(b)]));
}

fn liquify_distorted() -> bool {
    with_session(false, |s| !s.mesh.is_identity())
}

fn liquify_last_mesh_id() -> i32 {
    let id = store().last;
    if id != 0 && liquify_mesh(id).is_some() {
        id as i32
    } else {
        0
    }
}

fn liquify_load_mesh(id: i32) -> bool {
    let kept = match u32::try_from(id) {
        Ok(id) if id != 0 => liquify_mesh(id),
        _ => None,
    };
    let Some(kept) = kept else {
        return false;
    };
    with_session(false, |s| {
        let mut mesh = kept.scaled_to(s.mesh.width(), s.mesh.height());
        mesh.copy_freeze_from(&s.mesh);
        s.mesh = mesh;
        true
    })
}

fn liquify_finish(keep: bool) -> i32 {
    let session = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take();
    match session {
        Some(s) if keep && !s.mesh.is_identity() => register(s.mesh) as i32,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backdrop_sits_at_its_offset_in_the_active_layers_frame() {
        let mut buffer = PixelBuffer::new(2, 1, 3);
        buffer.data.copy_from_slice(&[10, 20, 30, 40, 50, 60]);
        let frame = PsdRect {
            left: 0,
            top: 0,
            right: 3,
            bottom: 2,
        };
        let rgba = in_frame(&buffer, 1, 1, frame);
        // Row 0 and column 0 are clear; the two pixels land at (1,1), (2,1).
        assert!(rgba[..12].iter().all(|&v| v == 0));
        assert_eq!(&rgba[12..16], &[0, 0, 0, 0]);
        assert_eq!(&rgba[16..24], &[10, 30, 50, 255, 20, 40, 60, 255]);
    }

    #[test]
    fn kept_meshes_are_found_by_id_and_zero_is_identity() {
        assert!(liquify_mesh(0).unwrap().is_identity());
        let mut mesh = LiquifyMesh::new(10, 10);
        mesh.stroke_to(
            &LiquifyBrush::new(LiquifyTool::ForwardWarp, 8.0),
            [3.0, 5.0, 1.0],
            [7.0, 5.0, 1.0],
        );
        let id = register(mesh.clone());
        assert_eq!(*liquify_mesh(id).unwrap(), mesh);
        assert_eq!(liquify_last_mesh_id(), id as i32);
        for _ in 0..KEPT {
            register(LiquifyMesh::new(2, 2));
        }
        assert!(liquify_mesh(id).is_none(), "the oldest mesh is dropped");
    }
}
