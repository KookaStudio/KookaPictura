use super::*;

use cxx_qt::CxxQtType;

pub(crate) fn frame_revision(view: &PictureView) -> u64 {
    view.rust().frame_revision
}

pub(crate) fn preview_present_level(view: &PictureView) -> i32 {
    let rust = view.rust();
    rust.session
        .live
        .preview()
        .map_or(0, |preview| preview.level as i32)
}
