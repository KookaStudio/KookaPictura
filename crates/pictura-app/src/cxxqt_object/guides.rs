//! Guide commands: free functions over a [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow). Every committed edit
//! records one history state and emits `changed`; a live drag step records
//! nothing.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{guide_near as nearest_guide, Guide, GuideOrientation};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Number of guides; 0 without a document.
        fn guide_count(view: &PictureView) -> i32;

        /// Guide `index` as `[vertical (1 or 0), position]` in document pixels; empty when out of range.
        fn guide_at(view: &PictureView, index: i32) -> Vec<f64>;

        /// The guide nearest `(x, y)` within `radius` document pixels, or -1.
        fn guide_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32;

        /// Add a guide and record one "New Guide" state; its index, or -1 (no state) without a document.
        fn add_guide(view: Pin<&mut PictureView>, vertical: bool, position: f64) -> i32;

        /// Move guide `index`. With `commit` false this is a live drag step (no history, no `changed`); with `commit` true it records one "Move Guide" state. False for a bad index or no document.
        fn move_guide(view: Pin<&mut PictureView>, index: i32, position: f64, commit: bool)
            -> bool;

        /// Delete guide `index` and record one "Delete Guide" state; false for a bad index or no document.
        fn remove_guide(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Delete every guide and record one "Clear Guides" state; false (no state) when there were none.
        fn clear_guides(view: Pin<&mut PictureView>) -> bool;
    }
}

fn guides(view: &PictureView) -> &[Guide] {
    view.rust().doc.as_ref().map_or(&[], |doc| &doc.guides)
}

/// Apply `edit` to the document's guides; `None` without a document.
fn edit<T>(view: Pin<&mut PictureView>, edit: impl FnOnce(&mut Vec<Guide>) -> T) -> Option<T> {
    let mut rust = view.rust_mut();
    rust.doc.as_mut().map(|doc| edit(&mut doc.guides))
}

fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().record(label);
    view.as_mut().changed();
}

fn guide_count(view: &PictureView) -> i32 {
    guides(view).len() as i32
}

fn guide_at(view: &PictureView, index: i32) -> Vec<f64> {
    usize::try_from(index)
        .ok()
        .and_then(|i| guides(view).get(i))
        .map_or_else(Vec::new, |g| {
            let vertical = g.orientation == GuideOrientation::Vertical;
            vec![f64::from(u8::from(vertical)), g.position]
        })
}

fn guide_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32 {
    nearest_guide(guides(view), x, y, radius.max(0.0)).map_or(-1, |i| i as i32)
}

fn add_guide(mut view: Pin<&mut PictureView>, vertical: bool, position: f64) -> i32 {
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

fn move_guide(
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

fn remove_guide(mut view: Pin<&mut PictureView>, index: i32) -> bool {
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

fn clear_guides(mut view: Pin<&mut PictureView>) -> bool {
    let cleared = edit(view.as_mut(), |g| !std::mem::take(g).is_empty()) == Some(true);
    if cleared {
        commit(view, "Clear Guides");
    }
    cleared
}
