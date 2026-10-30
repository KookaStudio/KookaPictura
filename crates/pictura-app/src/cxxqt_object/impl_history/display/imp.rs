use super::*;

use cxx_qt::CxxQtType;

pub(super) fn frame_revision(view: &PictureView) -> u64 {
    view.rust().frame_revision
}
