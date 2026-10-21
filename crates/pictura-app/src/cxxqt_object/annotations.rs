//! The Eyedropper group's Color Sampler, Note, and Ruler commands. Free
//! functions over a [`PictureView`] (their own bridge, so the `PictureView`
//! declaration list does not grow). `kind` is a [`MarkerKind`] integer: 0 color
//! sampler, 1 note. Every committed sampler/note edit records one history state
//! and emits `changed`; a live drag step records nothing. The ruler is view
//! state: it is neither recorded nor saved.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{
    Annotations, Layer, MarkerKind, Ruler, COUNT_LABEL_SIZE_MAX, COUNT_LABEL_SIZE_MIN,
    COUNT_MARKER_SIZE_MAX, COUNT_MARKER_SIZE_MIN,
};

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
        /// Number of markers of `kind`; 0 without a document or for an unknown kind.
        fn marker_count(view: &PictureView, kind: i32) -> i32;

        /// Marker `index` of `kind` as `[x, y]` in document pixels; empty when out of range.
        fn marker_at(view: &PictureView, kind: i32, index: i32) -> Vec<i32>;

        /// The marker of `kind` nearest `(x, y)` within `radius` document pixels, or -1.
        fn marker_near(view: &PictureView, kind: i32, x: f64, y: f64, radius: f64) -> i32;

        /// Place a marker and record one "Color Sampler" / "New Note" state; its index, or -1 (no state) when the four samplers are placed or without a document.
        fn add_marker(view: Pin<&mut PictureView>, kind: i32, x: i32, y: i32) -> i32;

        /// Move marker `index`. With `commit` false this is a live drag step (no history, no `changed`); with `commit` true it records one "Move Color Sampler" / "Move Note" state. False for a bad index or no document.
        fn move_marker(
            view: Pin<&mut PictureView>,
            kind: i32,
            index: i32,
            x: i32,
            y: i32,
            commit: bool,
        ) -> bool;

        /// Delete marker `index` and record one "Delete Color Sampler" / "Delete Note" state; false for a bad index or no document.
        fn remove_marker(view: Pin<&mut PictureView>, kind: i32, index: i32) -> bool;

        /// Delete every marker of `kind` and record one "Clear Color Samplers" / "Delete All Notes" state; false (no state) when there were none.
        fn clear_markers(view: Pin<&mut PictureView>, kind: i32) -> bool;

        /// Note `index`'s text; empty when out of range.
        fn note_text(view: &PictureView, index: i32) -> QString;

        /// Replace note `index`'s text and record one "Edit Note" state; false (no state) for a bad index or unchanged text.
        fn set_note_text(view: Pin<&mut PictureView>, index: i32, text: &QString) -> bool;

        /// Set the Ruler's line from `(ax, ay)` to `(bx, by)` in document pixels, snapped to 45° when `constrain`. No history.
        fn set_ruler(
            view: Pin<&mut PictureView>,
            ax: f64,
            ay: f64,
            bx: f64,
            by: f64,
            constrain: bool,
        );

        /// Remove the Ruler's line.
        fn clear_ruler(view: Pin<&mut PictureView>);

        /// The Ruler's line as `[ax, ay, bx, by]`; empty when there is none.
        fn ruler_line(view: &PictureView) -> Vec<f64>;

        /// The Ruler readout `[X, Y, W, H, A (degrees), D1]`; empty when there is no line.
        fn ruler_measurement(view: &PictureView) -> Vec<f64>;

        /// The document's `[memory, disk]` byte footprint; empty without a document.
        fn document_size_bytes(view: &PictureView) -> Vec<f64>;

        /// The number of Count groups (always at least one).
        fn count_group_count(view: &PictureView) -> i32;

        /// The active Count group's index.
        fn count_active_group(view: &PictureView) -> i32;

        /// Select Count group `index`; false when out of range. No history.
        fn count_set_active_group(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Group `index`'s name; empty when out of range.
        fn count_group_name(view: &PictureView, index: i32) -> QString;

        /// Group `index`'s eye visibility.
        fn count_group_visible(view: &PictureView, index: i32) -> bool;

        /// Group `index`'s colour as 0xRRGGBB.
        fn count_group_color(view: &PictureView, index: i32) -> u32;

        /// Group `index`'s marker size (1–10).
        fn count_group_marker_size(view: &PictureView, index: i32) -> i32;

        /// Group `index`'s label size (8–72).
        fn count_group_label_size(view: &PictureView, index: i32) -> i32;

        /// Group `index`'s mark count.
        fn count_group_total(view: &PictureView, index: i32) -> i32;

        /// The active group's mark count (the options bar's Count field).
        fn count_active_total(view: &PictureView) -> i32;

        /// Group `index` mark `mark` as `[x, y]`; empty when out of range.
        fn count_group_mark_at(view: &PictureView, index: i32, mark: i32) -> Vec<i32>;

        /// Add a group named `name` and make it active; records one "Add Count Group" state. Returns its index.
        fn count_add_group(view: Pin<&mut PictureView>, name: &QString) -> i32;

        /// Delete group `index`; false (no state) for the only group or out of range. Records "Delete Count Group".
        fn count_remove_group(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Set group `index`'s eye visibility; false when out of range. No history.
        fn count_set_visible(view: Pin<&mut PictureView>, index: i32, on: bool) -> bool;

        /// Set group `index`'s colour (0xRRGGBB); false when out of range. No history.
        fn count_set_color(view: Pin<&mut PictureView>, index: i32, rgb: u32) -> bool;

        /// Set group `index`'s marker size (clamped 1–10); false when out of range. No history.
        fn count_set_marker_size(view: Pin<&mut PictureView>, index: i32, size: i32) -> bool;

        /// Set group `index`'s label size (clamped 8–72); false when out of range. No history.
        fn count_set_label_size(view: Pin<&mut PictureView>, index: i32, size: i32) -> bool;

        /// Add a mark to the active group at `(x, y)`; records one "New Count" state. Returns its index.
        fn count_add_mark(view: Pin<&mut PictureView>, x: i32, y: i32) -> i32;

        /// The active group's mark nearest `(x, y)` within `radius`, or -1.
        fn count_mark_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32;

        /// Move active-group mark `index`. With `commit` false this is a live drag step; with `commit` true it records one "Move Count" state. False for a bad index.
        fn count_move_mark(
            view: Pin<&mut PictureView>,
            index: i32,
            x: i32,
            y: i32,
            commit: bool,
        ) -> bool;

        /// Delete active-group mark `index`; records one "Delete Count" state. False for a bad index.
        fn count_remove_mark(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Clear the active group's marks; records one "Clear Counts" state. False (no state) when there were none.
        fn count_clear(view: Pin<&mut PictureView>) -> bool;
    }
}

fn annotations(view: &PictureView) -> Option<&Annotations> {
    view.rust().doc.as_ref().map(|doc| &doc.annotations)
}

/// Apply `edit` to the document's annotations; `None` without a document.
fn edit<T>(view: Pin<&mut PictureView>, edit: impl FnOnce(&mut Annotations) -> T) -> Option<T> {
    let mut rust = view.rust_mut();
    rust.doc.as_mut().map(|doc| edit(&mut doc.annotations))
}

fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Meta);
}

fn label(kind: MarkerKind, sampler: &'static str, note: &'static str) -> &'static str {
    match kind {
        MarkerKind::ColorSampler => sampler,
        MarkerKind::Note => note,
    }
}

fn kind_and_index(kind: i32, index: i32) -> Option<(MarkerKind, usize)> {
    Some((MarkerKind::from_i32(kind)?, usize::try_from(index).ok()?))
}

fn marker_count(view: &PictureView, kind: i32) -> i32 {
    match (annotations(view), MarkerKind::from_i32(kind)) {
        (Some(a), Some(kind)) => a.markers(kind).len() as i32,
        _ => 0,
    }
}

fn marker_at(view: &PictureView, kind: i32, index: i32) -> Vec<i32> {
    kind_and_index(kind, index)
        .and_then(|(kind, index)| annotations(view)?.marker(kind, index))
        .map_or_else(Vec::new, |m| vec![m.x, m.y])
}

fn marker_near(view: &PictureView, kind: i32, x: f64, y: f64, radius: f64) -> i32 {
    MarkerKind::from_i32(kind)
        .and_then(|kind| annotations(view)?.marker_at(kind, x, y, radius.max(0.0)))
        .map_or(-1, |i| i as i32)
}

fn add_marker(mut view: Pin<&mut PictureView>, kind: i32, x: i32, y: i32) -> i32 {
    let Some(kind) = MarkerKind::from_i32(kind) else {
        return -1;
    };
    let Some(index) = edit(view.as_mut(), |a| a.add(kind, x, y)).flatten() else {
        return -1;
    };
    commit(view, label(kind, "Color Sampler", "New Note"));
    index as i32
}

fn move_marker(
    mut view: Pin<&mut PictureView>,
    kind: i32,
    index: i32,
    x: i32,
    y: i32,
    commit_move: bool,
) -> bool {
    let Some((kind, index)) = kind_and_index(kind, index) else {
        return false;
    };
    let moved = edit(view.as_mut(), |a| a.move_marker(kind, index, x, y)) == Some(true);
    if moved && commit_move {
        commit(view, label(kind, "Move Color Sampler", "Move Note"));
    }
    moved
}

fn remove_marker(mut view: Pin<&mut PictureView>, kind: i32, index: i32) -> bool {
    let Some((kind, index)) = kind_and_index(kind, index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |a| a.remove(kind, index)) == Some(true);
    if removed {
        commit(view, label(kind, "Delete Color Sampler", "Delete Note"));
    }
    removed
}

fn clear_markers(mut view: Pin<&mut PictureView>, kind: i32) -> bool {
    let Some(kind) = MarkerKind::from_i32(kind) else {
        return false;
    };
    let cleared = edit(view.as_mut(), |a| a.clear(kind)) == Some(true);
    if cleared {
        commit(
            view,
            label(kind, "Clear Color Samplers", "Delete All Notes"),
        );
    }
    cleared
}

fn note_text(view: &PictureView, index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.marker(MarkerKind::Note, i))
        .map_or_else(QString::default, |m| QString::from(m.text.as_str()))
}

fn set_note_text(mut view: Pin<&mut PictureView>, index: i32, text: &QString) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let text = text.to_string();
    let set = edit(view.as_mut(), |a| {
        let unchanged = a
            .marker(MarkerKind::Note, index)
            .is_some_and(|m| m.text == text);
        !unchanged && a.set_note_text(index, text)
    }) == Some(true);
    if set {
        commit(view, "Edit Note");
    }
    set
}

fn set_ruler(view: Pin<&mut PictureView>, ax: f64, ay: f64, bx: f64, by: f64, constrain: bool) {
    let ruler = if constrain {
        Ruler::constrained((ax, ay), (bx, by))
    } else {
        Ruler {
            a: (ax, ay),
            b: (bx, by),
        }
    };
    view.rust_mut().ruler = Some(ruler);
}

fn clear_ruler(view: Pin<&mut PictureView>) {
    view.rust_mut().ruler = None;
}

fn ruler_line(view: &PictureView) -> Vec<f64> {
    view.rust()
        .ruler
        .map_or_else(Vec::new, |r| vec![r.a.0, r.a.1, r.b.0, r.b.1])
}

fn ruler_measurement(view: &PictureView) -> Vec<f64> {
    view.rust().ruler.map_or_else(Vec::new, |r| {
        let m = r.measure();
        vec![m.x, m.y, m.width, m.height, m.angle, m.distance]
    })
}

/// Owned bytes of a layer's channel planes, mask, and every descendant's.
fn layer_plane_bytes(layer: &Layer) -> usize {
    let mut total: usize = layer
        .channels
        .iter()
        .map(|channel| channel.data.len())
        .sum();
    if let Some(data) = layer.mask.as_ref().and_then(|mask| mask.data.as_ref()) {
        total += data.len();
    }
    for child in &layer.children {
        total += layer_plane_bytes(child);
    }
    total
}

/// The document's pixel-plane memory footprint and on-disk file size in bytes as
/// `[memory, disk]`; empty without a document.
fn document_size_bytes(view: &PictureView) -> Vec<f64> {
    let rust = view.rust();
    let Some(doc) = rust.doc.as_ref() else {
        return Vec::new();
    };
    let mut memory = doc.composite.data.len();
    for layer in &doc.layers {
        memory += layer_plane_bytes(layer);
    }
    for channel in &doc.channels {
        memory += channel.data.len();
    }
    let disk = rust
        .path
        .as_deref()
        .and_then(|path| std::fs::metadata(path).ok())
        .map_or(0.0, |metadata| metadata.len() as f64);
    vec![memory as f64, disk]
}

// --- Count groups (Photoshop Extended) --------------------------------------

fn count_group_count(view: &PictureView) -> i32 {
    annotations(view).map_or(0, |a| a.count_groups().len() as i32)
}

fn count_active_group(view: &PictureView) -> i32 {
    annotations(view).map_or(0, |a| a.active_count_group() as i32)
}

fn count_set_active_group(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    // The overlay shows every group, so switching the active group changes no
    // pixels; the options bar refreshes itself.
    edit(view.as_mut(), |a| a.set_active_count_group(index)) == Some(true)
}

fn count_group_name(view: &PictureView, index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or_else(QString::default, |g| QString::from(g.name.as_str()))
}

fn count_group_visible(view: &PictureView, index: i32) -> bool {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .is_some_and(|g| g.visible)
}

fn count_group_color(view: &PictureView, index: i32) -> u32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(0, |g| {
            (u32::from(g.color[0]) << 16) | (u32::from(g.color[1]) << 8) | u32::from(g.color[2])
        })
}

fn count_group_marker_size(view: &PictureView, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(2, |g| i32::from(g.marker_size))
}

fn count_group_label_size(view: &PictureView, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(12, |g| i32::from(g.label_size))
}

fn count_group_total(view: &PictureView, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(0, |g| g.count() as i32)
}

fn count_active_total(view: &PictureView) -> i32 {
    annotations(view).map_or(0, |a| a.active_count_total() as i32)
}

fn count_group_mark_at(view: &PictureView, index: i32, mark: i32) -> Vec<i32> {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .and_then(|g| usize::try_from(mark).ok().and_then(|m| g.marker(m)))
        .map_or_else(Vec::new, |m| vec![m.x, m.y])
}

fn count_add_group(mut view: Pin<&mut PictureView>, name: &QString) -> i32 {
    let name = name.to_string();
    let Some(index) = edit(view.as_mut(), |a| a.add_count_group(name)) else {
        return -1;
    };
    commit(view, "Add Count Group");
    index as i32
}

fn count_remove_group(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |a| a.remove_count_group(index)) == Some(true);
    if removed {
        commit(view, "Delete Count Group");
    }
    removed
}

fn count_set_visible(mut view: Pin<&mut PictureView>, index: i32, on: bool) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let changed = edit(view.as_mut(), |a| {
        a.count_group_mut(index).is_some_and(|g| {
            let diff = g.visible != on;
            g.visible = on;
            diff
        })
    }) == Some(true);
    if changed {
        view.as_mut().changed();
    }
    changed
}

fn count_set_color(mut view: Pin<&mut PictureView>, index: i32, rgb: u32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let color = [
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ];
    let changed = edit(view.as_mut(), |a| {
        a.count_group_mut(index).is_some_and(|g| {
            let diff = g.color != color;
            g.color = color;
            diff
        })
    }) == Some(true);
    if changed {
        view.as_mut().changed();
    }
    changed
}

fn count_set_marker_size(mut view: Pin<&mut PictureView>, index: i32, size: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let size = size.clamp(
        i32::from(COUNT_MARKER_SIZE_MIN),
        i32::from(COUNT_MARKER_SIZE_MAX),
    ) as u8;
    let changed = edit(view.as_mut(), |a| {
        a.count_group_mut(index).is_some_and(|g| {
            let diff = g.marker_size != size;
            g.marker_size = size;
            diff
        })
    }) == Some(true);
    if changed {
        view.as_mut().changed();
    }
    changed
}

fn count_set_label_size(mut view: Pin<&mut PictureView>, index: i32, size: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let size = size.clamp(
        i32::from(COUNT_LABEL_SIZE_MIN),
        i32::from(COUNT_LABEL_SIZE_MAX),
    ) as u8;
    let changed = edit(view.as_mut(), |a| {
        a.count_group_mut(index).is_some_and(|g| {
            let diff = g.label_size != size;
            g.label_size = size;
            diff
        })
    }) == Some(true);
    if changed {
        view.as_mut().changed();
    }
    changed
}

fn count_add_mark(mut view: Pin<&mut PictureView>, x: i32, y: i32) -> i32 {
    let Some(index) = edit(view.as_mut(), |a| a.active_group_mut().add_mark(x, y)) else {
        return -1;
    };
    commit(view, "New Count");
    index as i32
}

fn count_mark_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32 {
    annotations(view)
        .and_then(|a| a.count_group(a.active_count_group()))
        .and_then(|g| g.mark_at(x, y, radius.max(0.0)))
        .map_or(-1, |i| i as i32)
}

fn count_move_mark(
    mut view: Pin<&mut PictureView>,
    index: i32,
    x: i32,
    y: i32,
    commit_move: bool,
) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let moved = edit(view.as_mut(), |a| {
        a.active_group_mut().move_mark(index, x, y)
    }) == Some(true);
    if moved && commit_move {
        commit(view, "Move Count");
    } else if moved {
        view.as_mut().changed();
    }
    moved
}

fn count_remove_mark(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |a| a.active_group_mut().remove_mark(index)) == Some(true);
    if removed {
        commit(view, "Delete Count");
    }
    removed
}

fn count_clear(mut view: Pin<&mut PictureView>) -> bool {
    let cleared = edit(view.as_mut(), |a| a.active_group_mut().clear()) == Some(true);
    if cleared {
        commit(view, "Clear Counts");
    }
    cleared
}
