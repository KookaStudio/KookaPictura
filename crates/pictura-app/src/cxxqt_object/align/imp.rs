use super::*;

use super::super::helpers::list_of_strings;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QStringList;
use pictura_core::PsdRect;
use pictura_render::AlignEdge;

/// The target box an align lines up against: `align_to` 1 is the canvas bounds,
/// else the selection's bounding box when `to_selection` and something is
/// selected, else `None` (align against each other).
pub(super) fn target(view: &PictureView, to_selection: bool, align_to: i32) -> Option<PsdRect> {
    let doc = view.rust().session.doc.as_ref()?;
    if align_to == 1 {
        return Some(PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        });
    }
    let selection = view
        .rust()
        .session
        .selection
        .as_ref()
        .filter(|_| to_selection)?;
    pictura_render::coverage_bounds(&selection.data, selection.width, selection.height)
}

pub(super) fn align_can(
    view: &PictureView,
    paths: &QStringList,
    to_selection: bool,
    align_to: i32,
) -> bool {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return false;
    };
    let owned = list_of_strings(paths);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    pictura_render::can_align(doc, &refs, target(view, to_selection, align_to))
}

pub(super) fn distribute_can(view: &PictureView, paths: &QStringList) -> bool {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return false;
    };
    let owned = list_of_strings(paths);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    pictura_render::can_distribute(doc, &refs)
}

pub(super) fn align_apply(
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

pub(super) fn distribute_apply(view: Pin<&mut PictureView>, paths: &QStringList, edge: i32) -> i32 {
    let Some(edge) = AlignEdge::from_index(edge) else {
        return 0;
    };
    view.batch_changed(paths, edge.distribute_name(), |doc, refs| {
        pictura_render::distribute_layers(doc, refs, edge)
    })
}
