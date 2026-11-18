use crate::cxxqt_object::state::PictureViewRust;

/// The selection's coverage when it is document-sized and not empty: Free
/// Transform then lifts the selected pixels instead of moving the whole layer.
pub(super) fn lift_coverage(rust: &PictureViewRust) -> Option<pictura_core::Coverage> {
    let doc = rust.session.doc.as_ref()?;
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .filter(|s| pictura_render::coverage_bounds(&s.data, doc.width, doc.height).is_some())
        .map(pictura_select::Selection::coverage)
}

/// Whether a selection on `path` lifts its pixels for the transform (CS6
/// transforms the selected pixels, a Background's included; a hidden layer's
/// does not).
pub(super) fn lifts_selection(rust: &PictureViewRust, path: &str) -> bool {
    rust.session.doc.as_ref().is_some_and(|doc| {
        pictura_render::resolve_path(doc, path)
            .is_some_and(|layer| layer.visible && pictura_render::can_lift_selection(layer))
    }) && selection_covers(rust)
}

/// Whether [`lift_coverage`] would find a selection, without copying it.
fn selection_covers(rust: &PictureViewRust) -> bool {
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .is_some_and(|s| pictura_render::plane_has_coverage(&s.data, doc.width, doc.height))
}
