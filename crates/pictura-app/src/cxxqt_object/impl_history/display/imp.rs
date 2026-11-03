use super::*;

use cxx_qt::CxxQtType;

pub(super) fn frame_revision(view: &PictureView) -> u64 {
    view.rust().frame_revision
}

pub(super) fn preview_present_level(view: &PictureView) -> i32 {
    let rust = view.rust();
    rust.session
        .live
        .preview()
        .map(|preview| preview.level as usize)
        .or(rust.deferred_pyramid.map(|deferred| deferred.level))
        .map_or(0, |level| level as i32)
}
