use super::*;

use super::super::type_paths;
use crate::cxxqt_object::edit::{structural_paths, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

pub(super) fn type_set_text_gamma(enabled: bool, gamma: f64) {
    pictura_render::set_text_gamma(enabled.then_some(gamma as f32));
}

pub(super) fn type_text_gamma_refresh(view: Pin<&mut PictureView>) {
    if view.as_ref().rust().session.doc.is_some() {
        view.recomposite();
    }
}

pub(super) fn type_layers_editable(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(pictura_render::type_layers_editable)
}

pub(super) fn type_replace_fonts(view: Pin<&mut PictureView>, missing: &QString) -> i32 {
    let missing = missing.to_string();
    let names: Vec<&str> = missing.lines().filter(|name| !name.is_empty()).collect();
    if names.is_empty() {
        return 0;
    }
    let mut replaced = 0;
    view.edit("Replace All Missing Fonts", |rust| {
        let before = rust.session.doc.as_ref().and_then(|doc| {
            let paths = type_paths(doc);
            let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
            pictura_render::damage::influence_paths(doc, &refs)
        });
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        replaced = pictura_render::replace_type_fonts(doc, &names);
        if replaced > 0 {
            let after = rust.session.doc.as_ref().and_then(|doc| {
                let paths = type_paths(doc);
                let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
                pictura_render::damage::influence_paths(doc, &refs)
            });
            structural_paths(before, after)
        } else {
            Edit::None
        }
    });
    i32::try_from(replaced).unwrap_or(i32::MAX)
}

pub(super) fn type_set_missing_glyph_protection(on: bool) {
    pictura_render::set_missing_glyph_protection(on);
}

pub(super) fn type_missing_glyph_protection() -> bool {
    pictura_render::missing_glyph_protection()
}

pub(super) fn type_missing_glyph_protection_refresh(mut view: Pin<&mut PictureView>) {
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return;
        };
        pictura_render::re_render_type_layers(doc)
    };
    view.as_mut().clear_link_sets();
    view.as_mut().recomposite();
    if changed > 0 {
        view.as_mut().changed();
    }
}
