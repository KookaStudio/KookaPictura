use super::*;

use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{
    Annotations, Layer, MarkerKind, Ruler, COUNT_LABEL_SIZE_MAX, COUNT_LABEL_SIZE_MIN,
    COUNT_MARKER_SIZE_MAX, COUNT_MARKER_SIZE_MIN,
};

pub(super) fn annotations(view: &PictureView) -> Option<&Annotations> {
    view.rust().session.doc.as_ref().map(|doc| &doc.annotations)
}

/// Apply `edit` to the document's annotations; `None` without a document.
pub(super) fn edit<T>(
    view: Pin<&mut PictureView>,
    edit: impl FnOnce(&mut Annotations) -> T,
) -> Option<T> {
    let mut rust = view.rust_mut();
    rust.session
        .doc
        .as_mut()
        .map(|doc| edit(&mut doc.annotations))
}

pub(super) fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Meta);
}

pub(super) fn label(kind: MarkerKind, sampler: &'static str, note: &'static str) -> &'static str {
    match kind {
        MarkerKind::ColorSampler => sampler,
        MarkerKind::Note => note,
    }
}

pub(super) fn kind_and_index(kind: i32, index: i32) -> Option<(MarkerKind, usize)> {
    Some((MarkerKind::from_i32(kind)?, usize::try_from(index).ok()?))
}

pub(super) fn marker_count(view: &PictureView, kind: i32) -> i32 {
    match (annotations(view), MarkerKind::from_i32(kind)) {
        (Some(a), Some(kind)) => a.markers(kind).len() as i32,
        _ => 0,
    }
}

pub(super) fn marker_at(view: &PictureView, kind: i32, index: i32) -> Vec<i32> {
    kind_and_index(kind, index)
        .and_then(|(kind, index)| annotations(view)?.marker(kind, index))
        .map_or_else(Vec::new, |m| vec![m.x, m.y])
}

pub(super) fn marker_near(view: &PictureView, kind: i32, x: f64, y: f64, radius: f64) -> i32 {
    MarkerKind::from_i32(kind)
        .and_then(|kind| annotations(view)?.marker_at(kind, x, y, radius.max(0.0)))
        .map_or(-1, |i| i as i32)
}

pub(super) fn add_marker(mut view: Pin<&mut PictureView>, kind: i32, x: i32, y: i32) -> i32 {
    let Some(kind) = MarkerKind::from_i32(kind) else {
        return -1;
    };
    let Some(index) = edit(view.as_mut(), |a| a.add(kind, x, y)).flatten() else {
        return -1;
    };
    commit(view, label(kind, "Color Sampler", "New Note"));
    index as i32
}

pub(super) fn move_marker(
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

pub(super) fn remove_marker(mut view: Pin<&mut PictureView>, kind: i32, index: i32) -> bool {
    let Some((kind, index)) = kind_and_index(kind, index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |a| a.remove(kind, index)) == Some(true);
    if removed {
        commit(view, label(kind, "Delete Color Sampler", "Delete Note"));
    }
    removed
}

pub(super) fn clear_markers(mut view: Pin<&mut PictureView>, kind: i32) -> bool {
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

pub(super) fn note_text(view: &PictureView, index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.marker(MarkerKind::Note, i))
        .map_or_else(QString::default, |m| QString::from(m.text.as_str()))
}

pub(super) fn set_note_text(mut view: Pin<&mut PictureView>, index: i32, text: &QString) -> bool {
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

pub(super) fn set_ruler(
    view: Pin<&mut PictureView>,
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    constrain: bool,
) {
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

pub(super) fn clear_ruler(view: Pin<&mut PictureView>) {
    view.rust_mut().ruler = None;
}

pub(super) fn ruler_line(view: &PictureView) -> Vec<f64> {
    view.rust()
        .ruler
        .map_or_else(Vec::new, |r| vec![r.a.0, r.a.1, r.b.0, r.b.1])
}

pub(super) fn ruler_measurement(view: &PictureView) -> Vec<f64> {
    view.rust().ruler.map_or_else(Vec::new, |r| {
        let m = r.measure();
        vec![m.x, m.y, m.width, m.height, m.angle, m.distance]
    })
}

/// Owned bytes of a layer's channel planes, mask, and every descendant's.
pub(super) fn layer_plane_bytes(layer: &Layer) -> usize {
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
pub(super) fn document_size_bytes(view: &PictureView) -> Vec<f64> {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
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

pub(super) fn count_group_count(view: &PictureView) -> i32 {
    annotations(view).map_or(0, |a| a.count_groups().len() as i32)
}

pub(super) fn count_active_group(view: &PictureView) -> i32 {
    annotations(view).map_or(0, |a| a.active_count_group() as i32)
}

pub(super) fn count_set_active_group(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    // The overlay shows every group, so switching the active group changes no
    // pixels; the options bar refreshes itself.
    edit(view.as_mut(), |a| a.set_active_count_group(index)) == Some(true)
}

pub(super) fn count_group_name(view: &PictureView, index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or_else(QString::default, |g| QString::from(g.name.as_str()))
}

pub(super) fn count_group_visible(view: &PictureView, index: i32) -> bool {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .is_some_and(|g| g.visible)
}

pub(super) fn count_group_color(view: &PictureView, index: i32) -> u32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(0, |g| {
            (u32::from(g.color[0]) << 16) | (u32::from(g.color[1]) << 8) | u32::from(g.color[2])
        })
}

pub(super) fn count_group_marker_size(view: &PictureView, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(2, |g| i32::from(g.marker_size))
}

pub(super) fn count_group_label_size(view: &PictureView, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(12, |g| i32::from(g.label_size))
}

pub(super) fn count_group_total(view: &PictureView, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .map_or(0, |g| g.count() as i32)
}

pub(super) fn count_active_total(view: &PictureView) -> i32 {
    annotations(view).map_or(0, |a| a.active_count_total() as i32)
}

pub(super) fn count_group_mark_at(view: &PictureView, index: i32, mark: i32) -> Vec<i32> {
    usize::try_from(index)
        .ok()
        .and_then(|i| annotations(view)?.count_group(i))
        .and_then(|g| usize::try_from(mark).ok().and_then(|m| g.marker(m)))
        .map_or_else(Vec::new, |m| vec![m.x, m.y])
}

pub(super) fn count_add_group(mut view: Pin<&mut PictureView>, name: &QString) -> i32 {
    let name = name.to_string();
    let Some(index) = edit(view.as_mut(), |a| a.add_count_group(name)) else {
        return -1;
    };
    commit(view, "Add Count Group");
    index as i32
}

pub(super) fn count_remove_group(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |a| a.remove_count_group(index)) == Some(true);
    if removed {
        commit(view, "Delete Count Group");
    }
    removed
}

pub(super) fn count_set_visible(mut view: Pin<&mut PictureView>, index: i32, on: bool) -> bool {
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

pub(super) fn count_set_color(mut view: Pin<&mut PictureView>, index: i32, rgb: u32) -> bool {
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

pub(super) fn count_set_marker_size(
    mut view: Pin<&mut PictureView>,
    index: i32,
    size: i32,
) -> bool {
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

pub(super) fn count_set_label_size(mut view: Pin<&mut PictureView>, index: i32, size: i32) -> bool {
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

pub(super) fn count_add_mark(mut view: Pin<&mut PictureView>, x: i32, y: i32) -> i32 {
    let Some(index) = edit(view.as_mut(), |a| a.active_group_mut().add_mark(x, y)) else {
        return -1;
    };
    commit(view, "New Count");
    index as i32
}

pub(super) fn count_mark_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32 {
    annotations(view)
        .and_then(|a| a.count_group(a.active_count_group()))
        .and_then(|g| g.mark_at(x, y, radius.max(0.0)))
        .map_or(-1, |i| i as i32)
}

pub(super) fn count_move_mark(
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

pub(super) fn count_remove_mark(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |a| a.active_group_mut().remove_mark(index)) == Some(true);
    if removed {
        commit(view, "Delete Count");
    }
    removed
}

pub(super) fn count_clear(mut view: Pin<&mut PictureView>) -> bool {
    let cleared = edit(view.as_mut(), |a| a.active_group_mut().clear()) == Some(true);
    if cleared {
        commit(view, "Clear Counts");
    }
    cleared
}
