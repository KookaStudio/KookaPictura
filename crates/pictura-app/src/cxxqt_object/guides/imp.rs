use super::*;

use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{guide_near as nearest_guide, Guide, GuideOrientation};

pub(super) fn guides(view: &PictureView) -> &[Guide] {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or(&[], |doc| &doc.guides)
}

/// Apply `edit` to the document's guides; `None` without a document.
pub(super) fn edit<T>(
    view: Pin<&mut PictureView>,
    edit: impl FnOnce(&mut Vec<Guide>) -> T,
) -> Option<T> {
    let mut rust = view.rust_mut();
    rust.session.doc.as_mut().map(|doc| edit(&mut doc.guides))
}

pub(super) fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Meta);
}

pub(super) fn guide_count(view: &PictureView) -> i32 {
    guides(view).len() as i32
}

pub(super) fn guide_at(view: &PictureView, index: i32) -> Vec<f64> {
    usize::try_from(index)
        .ok()
        .and_then(|i| guides(view).get(i))
        .map_or_else(Vec::new, |g| {
            let vertical = g.orientation == GuideOrientation::Vertical;
            vec![f64::from(u8::from(vertical)), g.position]
        })
}

pub(super) fn guide_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32 {
    nearest_guide(guides(view), x, y, radius.max(0.0)).map_or(-1, |i| i as i32)
}

pub(super) fn add_guide(mut view: Pin<&mut PictureView>, vertical: bool, position: f64) -> i32 {
    let orientation = if vertical {
        GuideOrientation::Vertical
    } else {
        GuideOrientation::Horizontal
    };
    let Some(index) = edit(view.as_mut(), |g| {
        g.push(Guide {
            orientation,
            position,
        });
        g.len() - 1
    }) else {
        return -1;
    };
    commit(view, "New Guide");
    index as i32
}

pub(super) fn move_guide(
    mut view: Pin<&mut PictureView>,
    index: i32,
    position: f64,
    commit_move: bool,
) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let moved = edit(view.as_mut(), |g| {
        g.get_mut(index)
            .map(|guide| guide.position = position)
            .is_some()
    }) == Some(true);
    if moved && commit_move {
        commit(view, "Move Guide");
    }
    moved
}

pub(super) fn remove_guide(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = edit(view.as_mut(), |g| {
        (index < g.len()).then(|| g.remove(index)).is_some()
    }) == Some(true);
    if removed {
        commit(view, "Delete Guide");
    }
    removed
}

pub(super) fn clear_guides(mut view: Pin<&mut PictureView>) -> bool {
    let cleared = edit(view.as_mut(), |g| !std::mem::take(g).is_empty()) == Some(true);
    if cleared {
        commit(view, "Clear Guides");
    }
    cleared
}
