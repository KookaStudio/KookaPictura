use super::*;

use super::super::helpers_composite::sample_argb_scoped as sample_scoped;
use cxx_qt::CxxQtType;

pub(super) fn sample_argb_scoped(view: &PictureView, x: i32, y: i32, size: i32, scope: i32) -> u32 {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return 0;
    };
    let path = rust.layer_target.single().unwrap_or_default();
    sample_scoped(doc, path, x, y, size, scope)
}
