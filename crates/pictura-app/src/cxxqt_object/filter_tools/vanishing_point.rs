//! Filter ▸ Vanishing Point (#349): the dialog's plane-editing session and
//! its kept edits. The dialog edits planes over the document here; OK keeps
//! the planes on the document and, with Render Grids on, keeps the edit the
//! `vanishing-point` kind's one slot names by id, so the commit, Fade and
//! Last Filter run the ordinary filter path, as Liquify's mesh does. Free
//! functions (their own bridge); the dialog is modal, so one session at a
//! time.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_core::PsdRect;
use pictura_filters::{VpDragMode, VpEdit, VpEditor, VpPart, VpPlanes, VP_DEFAULT_GRID};

use super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::helpers_composite::document_to_image;
use super::super::qobject::PictureView;
use super::document_supported;

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
        /// Open a session over the active layer, on the planes the document
        /// kept. False when the document is not 8-bit RGB or Grayscale or
        /// there is no pixel layer to edit.
        fn vp_begin(view: &PictureView) -> bool;

        /// Replace the session's planes with those kept edit `id` was made
        /// from (Last Filter Settings). False when it is no longer kept or
        /// was made on a document of another size.
        fn vp_load(id: i32) -> bool;

        /// The session's document size, `[width, height]`; empty with no
        /// session.
        fn vp_geometry() -> Vec<i32>;

        /// The document's composite, as the dialog shows it.
        fn vp_backdrop(view: &PictureView) -> QImage;

        fn vp_plane_count() -> i32;

        /// Plane `index`'s four corners, `x, y` pairs in document pixels.
        fn vp_plane_corners(index: i32) -> Vec<f64>;

        /// Plane `index`'s four edge-middle nodes, `x, y` pairs.
        fn vp_plane_edge_nodes(index: i32) -> Vec<f64>;

        /// 0 valid (blue), 1 no aspect ratio (red), 2 no camera (yellow).
        fn vp_plane_validity(index: i32) -> i32;

        /// Plane `index`'s grid lines, `x0, y0, x1, y1` per line.
        fn vp_plane_grid_lines(index: i32) -> Vec<f64>;

        /// How a plane on these four `x, y` corners would read (0–2).
        fn vp_quad_validity(points: &[f64]) -> i32;

        /// Add a plane on four `x, y` corners and make it active; its index,
        /// or -1 for a malformed list.
        fn vp_create(points: &[f64]) -> i32;

        /// The active plane, or -1.
        fn vp_active() -> i32;
        fn vp_set_active(index: i32);

        /// What document point `(x, y)` is over within `tolerance` px:
        /// `[plane, part, node]`, part 0 a corner, 1 an edge node, 2 the
        /// inside; empty over nothing.
        fn vp_hit(x: f64, y: f64, tolerance: f64) -> Vec<i32>;

        /// Ctrl-click: make the next plane under `(x, y)` active; its index
        /// or -1.
        fn vp_cycle(x: f64, y: f64) -> i32;

        /// Start a drag of `part`/`node` of `index` from `(x, y)`; `mode` 0
        /// plain, 1 tear off (Ctrl on an edge node), 2 turn (Alt on a far
        /// edge node). False when the gesture does not apply.
        fn vp_drag_begin(index: i32, part: i32, node: i32, mode: i32, x: f64, y: f64) -> bool;
        fn vp_drag_to(x: f64, y: f64);
        fn vp_drag_end();

        /// The active plane family's Grid Size (the default with none).
        fn vp_grid_size() -> f64;
        fn vp_set_grid_size(size: f64);

        /// The active plane's angle to its parent, or 0 for a root.
        fn vp_angle() -> f64;
        /// Whether the active plane may turn: a related plane with no
        /// related planes of its own.
        fn vp_angle_editable() -> bool;
        fn vp_set_angle(degrees: f64) -> bool;

        /// Delete the active plane and the planes hinged on it.
        fn vp_delete_active() -> bool;

        fn vp_undo() -> bool;
        fn vp_redo() -> bool;

        /// End the session. With `keep` the document keeps the planes, and
        /// with `render_grids` a grid edit is kept and its id returned for
        /// the `vanishing-point` slot; otherwise 0.
        fn vp_finish(view: Pin<&mut PictureView>, keep: bool, render_grids: bool) -> i32;
    }
}

/// ponytail: the eight most recent edits stay in memory for the process; a
/// Last Filter that names an older one is refused.
const KEPT: usize = 8;

struct Kept {
    id: u32,
    edit: Arc<VpEdit>,
    planes: VpPlanes,
}

struct Store {
    next: u32,
    kept: VecDeque<Kept>,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    next: 1,
    kept: VecDeque::new(),
});

fn store() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn register(edit: VpEdit, planes: VpPlanes) -> u32 {
    let mut store = store();
    let id = store.next;
    store.next = store.next.wrapping_add(1).max(1);
    if store.kept.len() == KEPT {
        store.kept.pop_front();
    }
    store.kept.push_back(Kept {
        id,
        edit: Arc::new(edit),
        planes,
    });
    id
}

/// The edit `id` names. Id 0 — the descriptor's default — draws nothing; an
/// id no longer kept is `None`.
pub(in crate::cxxqt_object) fn vp_edit(id: u32) -> Option<Arc<VpEdit>> {
    if id == 0 {
        return Some(Arc::new(VpEdit::default()));
    }
    store()
        .kept
        .iter()
        .find(|k| k.id == id)
        .map(|k| k.edit.clone())
}

struct Session {
    editor: VpEditor,
    /// The active layer's rect: the edit is kept in its pixels.
    frame: PsdRect,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn with_session<R>(default: R, f: impl FnOnce(&mut Session) -> R) -> R {
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map_or(default, f)
}

fn pairs(points: &[f64]) -> Option<[(f64, f64); 4]> {
    (points.len() == 8 && points.iter().all(|v| v.is_finite()))
        .then(|| std::array::from_fn(|k| (points[2 * k], points[2 * k + 1])))
}

fn flat(points: &[(f64, f64)]) -> Vec<f64> {
    points.iter().flat_map(|p| [p.0, p.1]).collect()
}

fn plane_index(editor: &VpEditor, index: i32) -> Option<usize> {
    usize::try_from(index)
        .ok()
        .filter(|&i| i < editor.planes().len())
}

fn vp_begin(view: &PictureView) -> bool {
    let rust = view.rust();
    if active_mask_target(rust).is_some() {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref().filter(|d| document_supported(d)) else {
        return false;
    };
    let Some(layer) = active_pixel_layer(doc, rust.layer_target.single()) else {
        return false;
    };
    if doc.width == 0 || doc.height == 0 {
        return false;
    }
    let planes = rust
        .session
        .vanishing_planes
        .clone()
        .filter(|p| p.size() == (doc.width, doc.height))
        .unwrap_or_else(|| VpPlanes::new(doc.width, doc.height));
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(Session {
        editor: VpEditor::new(planes),
        frame: layer.rect,
    });
    true
}

fn vp_load(id: i32) -> bool {
    let Some(planes) = u32::try_from(id).ok().filter(|&id| id != 0).and_then(|id| {
        store()
            .kept
            .iter()
            .find(|k| k.id == id)
            .map(|k| k.planes.clone())
    }) else {
        return false;
    };
    with_session(false, |s| {
        if planes.size() != s.editor.planes().size() {
            return false;
        }
        s.editor = VpEditor::new(planes);
        true
    })
}

fn vp_geometry() -> Vec<i32> {
    with_session(Vec::new(), |s| {
        let (w, h) = s.editor.planes().size();
        vec![w as i32, h as i32]
    })
}

fn vp_backdrop(view: &PictureView) -> QImage {
    let rust = view.rust();
    rust.session
        .doc
        .as_ref()
        .map_or_else(QImage::default, |doc| {
            document_to_image(doc, rust.gpu_compute)
        })
}

fn vp_plane_count() -> i32 {
    with_session(0, |s| s.editor.planes().len() as i32)
}

fn vp_plane_corners(index: i32) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        plane_index(&s.editor, index).map_or_else(Vec::new, |i| flat(&s.editor.planes().corners(i)))
    })
}

fn vp_plane_edge_nodes(index: i32) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        plane_index(&s.editor, index)
            .map_or_else(Vec::new, |i| flat(&s.editor.planes().edge_nodes(i)))
    })
}

fn vp_plane_validity(index: i32) -> i32 {
    with_session(1, |s| {
        plane_index(&s.editor, index).map_or(1, |i| s.editor.planes().validity(i).code())
    })
}

fn vp_plane_grid_lines(index: i32) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        plane_index(&s.editor, index).map_or_else(Vec::new, |i| {
            s.editor
                .planes()
                .grid_lines(i)
                .iter()
                .flat_map(|[a, b]| [a.0, a.1, b.0, b.1])
                .collect()
        })
    })
}

fn vp_quad_validity(points: &[f64]) -> i32 {
    let Some(q) = pairs(points) else {
        return 1;
    };
    with_session(1, |s| s.editor.planes().quad_validity(&q).code())
}

fn vp_create(points: &[f64]) -> i32 {
    let Some(q) = pairs(points) else {
        return -1;
    };
    with_session(-1, |s| s.editor.create(q) as i32)
}

fn vp_active() -> i32 {
    with_session(-1, |s| s.editor.active().map_or(-1, |i| i as i32))
}

fn vp_set_active(index: i32) {
    with_session((), |s| {
        let i = plane_index(&s.editor, index);
        s.editor.set_active(i);
    });
}

fn vp_hit(x: f64, y: f64, tolerance: f64) -> Vec<i32> {
    with_session(Vec::new(), |s| {
        s.editor
            .hit((x, y), tolerance)
            .map_or_else(Vec::new, |(i, part)| {
                let (code, node) = match part {
                    VpPart::Corner(k) => (0, k as i32),
                    VpPart::Edge(k) => (1, k as i32),
                    VpPart::Inside => (2, -1),
                };
                vec![i as i32, code, node]
            })
    })
}

fn vp_cycle(x: f64, y: f64) -> i32 {
    with_session(-1, |s| s.editor.cycle((x, y)).map_or(-1, |i| i as i32))
}

fn vp_drag_begin(index: i32, part: i32, node: i32, mode: i32, x: f64, y: f64) -> bool {
    let k = usize::try_from(node).unwrap_or(0) % 4;
    let part = match part {
        0 => VpPart::Corner(k),
        1 => VpPart::Edge(k),
        _ => VpPart::Inside,
    };
    let mode = match mode {
        1 => VpDragMode::TearOff,
        2 => VpDragMode::Angle,
        _ => VpDragMode::Normal,
    };
    with_session(false, |s| {
        plane_index(&s.editor, index).is_some_and(|i| s.editor.begin(i, part, mode, (x, y)))
    })
}

fn vp_drag_to(x: f64, y: f64) {
    with_session((), |s| s.editor.drag_to((x, y)));
}

fn vp_drag_end() {
    with_session((), |s| s.editor.end());
}

fn vp_grid_size() -> f64 {
    with_session(VP_DEFAULT_GRID, |s| {
        s.editor
            .active()
            .map_or(VP_DEFAULT_GRID, |i| s.editor.planes().grid(i))
    })
}

fn vp_set_grid_size(size: f64) {
    if size.is_finite() {
        with_session((), |s| s.editor.set_grid(size));
    }
}

fn vp_angle() -> f64 {
    with_session(0.0, |s| {
        s.editor
            .active()
            .and_then(|i| s.editor.planes().angle(i))
            .unwrap_or(0.0)
    })
}

fn vp_angle_editable() -> bool {
    with_session(false, |s| {
        s.editor
            .active()
            .is_some_and(|i| s.editor.planes().angle_editable(i))
    })
}

fn vp_set_angle(degrees: f64) -> bool {
    with_session(false, |s| s.editor.set_angle(degrees))
}

fn vp_delete_active() -> bool {
    with_session(false, |s| s.editor.delete_active())
}

fn vp_undo() -> bool {
    with_session(false, |s| s.editor.undo())
}

fn vp_redo() -> bool {
    with_session(false, |s| s.editor.redo())
}

fn vp_finish(mut view: Pin<&mut PictureView>, keep: bool, render_grids: bool) -> i32 {
    let session = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take();
    let Some(Session { editor, frame }) = session.filter(|_| keep) else {
        return 0;
    };
    let planes = editor.into_planes();
    let edit = VpEdit {
        lines: if render_grids {
            planes.rendered_grid(-f64::from(frame.left), -f64::from(frame.top))
        } else {
            Vec::new()
        },
    };
    view.as_mut().rust_mut().session.vanishing_planes = Some(planes.clone());
    if edit.is_empty() {
        0
    } else {
        register(edit, planes) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::super::eight_bit_refusal;
    use super::*;
    use pictura_core::{BitDepth, ColorMode, Document};

    #[test]
    fn kept_edits_are_found_by_id_and_zero_draws_nothing() {
        assert!(vp_edit(0).unwrap().is_empty());
        let mut planes = VpPlanes::new(100, 100);
        planes.add([(10.0, 10.0), (90.0, 10.0), (90.0, 90.0), (10.0, 90.0)]);
        let lines = planes.rendered_grid(0.0, 0.0);
        let id = register(
            VpEdit {
                lines: lines.clone(),
            },
            planes,
        );
        assert_eq!(vp_edit(id).unwrap().lines, lines);
        for _ in 0..KEPT {
            register(VpEdit::default(), VpPlanes::new(1, 1));
        }
        assert!(vp_edit(id).is_none(), "an evicted edit is refused");
    }

    #[test]
    fn commits_are_refused_off_eight_bit_rgb_and_gray() {
        let rgb8 = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        let gray8 = Document::new(4, 4, ColorMode::Grayscale, BitDepth::Eight);
        let rgb16 = Document::new(4, 4, ColorMode::Rgb, BitDepth::Sixteen);
        let cmyk8 = Document::new(4, 4, ColorMode::Cmyk, BitDepth::Eight);
        for kind in ["adaptive-wide-angle", "vanishing-point"] {
            assert!(eight_bit_refusal(kind, Some(&rgb8)).is_none());
            assert!(eight_bit_refusal(kind, Some(&gray8)).is_none());
            assert!(eight_bit_refusal(kind, Some(&rgb16)).is_some());
            assert!(eight_bit_refusal(kind, Some(&cmyk8)).is_some());
            assert!(eight_bit_refusal(kind, None).is_some());
        }
        assert!(eight_bit_refusal("liquify", Some(&rgb16)).is_none());
    }
}
