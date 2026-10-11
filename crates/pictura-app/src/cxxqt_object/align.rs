//! Layer > Align / Align Layers To Selection / Distribute and the Move tool's
//! align and distribute buttons. Free functions over a [`PictureView`] (their
//! own bridge, so the `PictureView` declaration list does not grow). `edge` is
//! the menu order, 0 Top … 5 Right. Each edit recomposites and records one
//! state named for the edge.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::list_of_strings;
use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QStringList;
use pictura_core::PsdRect;
use pictura_render::AlignEdge;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether Align applies to `paths`: two layers with content, or one when the target box exists (`to_selection` with a selection, or `align_to` Canvas).
        fn align_can(
            view: &PictureView,
            paths: &QStringList,
            to_selection: bool,
            align_to: i32,
        ) -> bool;

        /// Whether Distribute applies to `paths` (three layers with content).
        fn distribute_can(view: &PictureView, paths: &QStringList) -> bool;

        /// Align `paths` along `edge`, against `align_to` 1 Canvas's bounds, the selection's bounds when `to_selection` and one exists, else against each other. Returns how many layers moved.
        fn align_apply(
            view: Pin<&mut PictureView>,
            paths: &QStringList,
            edge: i32,
            to_selection: bool,
            align_to: i32,
        ) -> i32;

        /// Distribute `paths` along `edge`. Returns how many layers moved.
        fn distribute_apply(view: Pin<&mut PictureView>, paths: &QStringList, edge: i32) -> i32;
    }
}

/// The target box an align lines up against: `align_to` 1 is the canvas bounds,
/// else the selection's bounding box when `to_selection` and something is
/// selected, else `None` (align against each other).
fn target(view: &PictureView, to_selection: bool, align_to: i32) -> Option<PsdRect> {
    let doc = view.rust().doc.as_ref()?;
    if align_to == 1 {
        return Some(PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        });
    }
    let selection = view.rust().selection.as_ref().filter(|_| to_selection)?;
    pictura_render::coverage_bounds(&selection.data, selection.width, selection.height)
}

fn align_can(view: &PictureView, paths: &QStringList, to_selection: bool, align_to: i32) -> bool {
    let Some(doc) = view.rust().doc.as_ref() else {
        return false;
    };
    let owned = list_of_strings(paths);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    pictura_render::can_align(doc, &refs, target(view, to_selection, align_to))
}

fn distribute_can(view: &PictureView, paths: &QStringList) -> bool {
    let Some(doc) = view.rust().doc.as_ref() else {
        return false;
    };
    let owned = list_of_strings(paths);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    pictura_render::can_distribute(doc, &refs)
}

fn align_apply(
    view: Pin<&mut PictureView>,
    paths: &QStringList,
    edge: i32,
    to_selection: bool,
    align_to: i32,
) -> i32 {
    let Some(edge) = AlignEdge::from_index(edge) else {
        return 0;
    };
    let selection = target(&view, to_selection, align_to);
    view.batch_changed(paths, edge.align_name(), |doc, refs| {
        pictura_render::align_layers(doc, refs, edge, selection)
    })
}

fn distribute_apply(view: Pin<&mut PictureView>, paths: &QStringList, edge: i32) -> i32 {
    let Some(edge) = AlignEdge::from_index(edge) else {
        return 0;
    };
    view.batch_changed(paths, edge.distribute_name(), |doc, refs| {
        pictura_render::distribute_layers(doc, refs, edge)
    })
}
