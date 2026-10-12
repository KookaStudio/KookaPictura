//! Filter ▸ Liquify (#348): the dialog's editing session and its finished
//! meshes. The dialog edits a mesh over the active layer here, previewing on
//! a reduced copy; OK keeps the mesh and the `liquify` kind's one slot names
//! it by id, so the commit, Fade and Last Filter run the ordinary filter
//! path, as Displace's map does. Free functions (their own bridge); the
//! dialog is modal, so one session at a time.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat};
use pictura_core::PsdRect;
use pictura_filters::{LiquifyBrush, LiquifyMesh, LiquifyProxy, LiquifyTool};

use super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::qobject::PictureView;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Open a session over the active layer, previewed on a copy reduced
        /// by a whole factor to at most `max_side` on the long side. False
        /// when there is no unmasked pixel layer with colour data.
        fn liquify_begin(view: &PictureView, max_side: i32) -> bool;

        /// The session's layer size and preview scale (layer pixels per
        /// preview pixel): `[width, height, scale]`, empty with no session.
        fn liquify_geometry() -> Vec<i32>;

        /// The whole preview, premultiplied; null with no session.
        fn liquify_image() -> QImage;

        /// The preview pixels `(x, y, w, h)`, premultiplied.
        fn liquify_patch(x: i32, y: i32, w: i32, h: i32) -> QImage;

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
        /// (0..1). Each stroke call returns the preview rectangle it
        /// re-rendered, `[x, y, w, h]`, or empty when nothing changed.
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

        /// Tint frozen areas with `(r, g, b)` in the preview, or not.
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
    proxy: LiquifyProxy,
    /// The rendered preview, premultiplied RGBA8 at the proxy's size.
    out: Vec<u8>,
    brush: LiquifyBrush,
    last: Option<[f64; 3]>,
    mask: Option<[u8; 3]>,
}

impl Session {
    fn render(&mut self, layer: Option<PsdRect>) -> Vec<i32> {
        let Some(layer) = layer else {
            return Vec::new();
        };
        let r = self.proxy.proxy_rect(layer);
        self.proxy.render(&self.mesh, r, &mut self.out, self.mask);
        vec![r.left, r.top, r.width(), r.height()]
    }

    fn render_all(&mut self) {
        let all = PsdRect {
            left: 0,
            top: 0,
            right: self.proxy.width() as i32,
            bottom: self.proxy.height() as i32,
        };
        self.proxy.render(&self.mesh, all, &mut self.out, self.mask);
    }
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn with_session<R>(default: R, f: impl FnOnce(&mut Session) -> R) -> R {
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map_or(default, f)
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

fn liquify_begin(view: &PictureView, max_side: i32) -> bool {
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
    let channel = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id && c.data.len() == plane)
            .map(|c| &c.data[..])
    };
    let Some(c0) = channel(0) else {
        return false;
    };
    // A Grayscale layer carries one colour plane.
    let color = [c0, channel(1).unwrap_or(c0), channel(2).unwrap_or(c0)];
    let proxy = LiquifyProxy::new(
        w as u32,
        h as u32,
        color,
        channel(-1),
        max_side.max(1) as u32,
    );
    let mut session = Session {
        mesh: LiquifyMesh::new(w as u32, h as u32),
        out: vec![0; proxy.width() * proxy.height() * 4],
        proxy,
        brush: LiquifyBrush::new(LiquifyTool::ForwardWarp, 100.0),
        last: None,
        mask: None,
    };
    session.render_all();
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);
    true
}

fn liquify_geometry() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        vec![
            s.mesh.width() as i32,
            s.mesh.height() as i32,
            s.proxy.scale() as i32,
        ]
    })
}

fn liquify_image() -> QImage {
    with_session(QImage::default(), |s| {
        let (w, h) = (s.proxy.width(), s.proxy.height());
        image(s.out.clone(), w as i32, h as i32)
    })
}

fn liquify_patch(x: i32, y: i32, w: i32, h: i32) -> QImage {
    with_session(QImage::default(), |s| {
        let (pw, ph) = (s.proxy.width() as i32, s.proxy.height() as i32);
        let (x0, y0) = (x.clamp(0, pw), y.clamp(0, ph));
        let (x1, y1) = ((x + w).clamp(0, pw), (y + h).clamp(0, ph));
        if x0 >= x1 || y0 >= y1 {
            return QImage::default();
        }
        let stride = pw as usize * 4;
        let mut bytes = Vec::with_capacity((x1 - x0) as usize * (y1 - y0) as usize * 4);
        for row in y0..y1 {
            let start = row as usize * stride + x0 as usize * 4;
            bytes.extend_from_slice(&s.out[start..start + (x1 - x0) as usize * 4]);
        }
        image(bytes, x1 - x0, y1 - y0)
    })
}

fn image(bytes: Vec<u8>, w: i32, h: i32) -> QImage {
    // SAFETY: `bytes` is exactly w*h premultiplied RGBA8888 samples.
    unsafe { QImage::from_raw_bytes(bytes, w, h, QImageFormat::Format_RGBA8888_Premultiplied) }
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
        let dirty = s.mesh.stroke_begin(&s.brush, p);
        s.render(dirty)
    })
}

fn liquify_stroke_to(x: f64, y: f64, pressure: f64) -> Vec<i32> {
    with_session(Vec::new(), |s| {
        let b = [x, y, pressure];
        let dirty = s.last.and_then(|a| s.mesh.stroke_to(&s.brush, a, b));
        s.last = Some(b);
        s.render(dirty)
    })
}

fn liquify_hold() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        let dirty = s.last.and_then(|p| s.mesh.hold(&s.brush, p));
        s.render(dirty)
    })
}

fn liquify_reconstruct(amount: f64) {
    with_session((), |s| {
        s.mesh.reconstruct(amount);
        s.render_all();
    });
}

fn liquify_restore_all() {
    with_session((), |s| {
        s.mesh.restore_all();
        s.render_all();
    });
}

fn liquify_mask(op: i32) {
    with_session((), |s| {
        match op {
            0 => s.mesh.thaw_all(),
            1 => s.mesh.freeze_all(),
            _ => s.mesh.invert_freeze(),
        }
        s.render_all();
    });
}

fn liquify_set_mask_overlay(show: bool, r: i32, g: i32, b: i32) {
    let byte = |v: i32| v.clamp(0, 255) as u8;
    with_session((), |s| {
        s.mask = show.then(|| [byte(r), byte(g), byte(b)]);
        s.render_all();
    });
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
        s.render_all();
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
