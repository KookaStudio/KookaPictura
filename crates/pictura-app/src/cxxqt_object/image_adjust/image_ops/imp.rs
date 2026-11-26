use super::*;

use super::super::super::picture::{coverage_bounds, picture_rgba};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::impl_core::{JobBar, Outcome};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Document;

pub(super) fn duplicate_into(
    source: &PictureView,
    target: Pin<&mut PictureView>,
    name: &QString,
    merged: bool,
) -> bool {
    let rust = source.rust();
    let Some(src) = rust.session.doc.as_ref() else {
        return false;
    };
    let doc = if merged {
        let rgba = picture_rgba(src, rust, false);
        Document::from_rgba(&name.to_string(), src.width, src.height, &rgba)
    } else {
        src.clone()
    };
    finish(target, doc, "Duplicate")
}

/// Install `doc` on `view` as a fresh document with a single history anchor.
pub(super) fn finish(mut view: Pin<&mut PictureView>, doc: Document, label: &str) -> bool {
    {
        let mut rust = view.as_mut().rust_mut();
        rust.replace_document(Some(doc), Default::default(), Default::default(), None);
        rust.select_only_layer("0");
    }
    view.as_mut().edit(label, |_| Edit::Full);
    view.as_mut().session_cancelled();
    true
}

pub(super) fn trim_image(mut view: Pin<&mut PictureView>) -> bool {
    {
        let rust = view.rust();
        if rust.has_active_session() || rust.session.pending_previews.any() {
            return false;
        }
    }
    let Some((width, height, (left, top, right, bottom))) = view
        .rust()
        .session
        .doc
        .as_ref()
        .map(|doc| (doc.width, doc.height, coverage_bounds(doc, view.rust())))
    else {
        return false;
    };
    if right <= left || bottom <= top {
        return false;
    }
    if left == 0 && top == 0 && right == width as i32 && bottom == height as i32 {
        return false;
    }
    let top_path = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let mut target = rust.layer_target.clone();
        target.retain_resolving(doc);
        target
            .is_empty()
            .then(|| doc.layers.len().saturating_sub(1).to_string())
    };
    if let Some(top_path) = top_path {
        view.as_mut().rust_mut().select_only_layer(&top_path);
    }
    let edited = view.as_mut().edit("Trim", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        if !pictura_render::crop_document(
            doc,
            left,
            top,
            (right - left) as u32,
            (bottom - top) as u32,
        ) {
            return Edit::None;
        }
        rust.session.clear_geometry_selection();
        Edit::Full
    });
    !matches!(edited, Edit::None)
}

pub(super) fn reveal_all(mut view: Pin<&mut PictureView>) -> bool {
    let edited = view.as_mut().edit("Reveal All", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        if !pictura_render::reveal_all(doc) {
            return Edit::None;
        }
        rust.session.clear_geometry_selection();
        Edit::Full
    });
    !matches!(edited, Edit::None)
}

pub(super) fn rotate_document_arbitrary(view: Pin<&mut PictureView>, angle_deg: f64) -> bool {
    rotate_document_arbitrary_with(view, angle_deg, &QString::from("bilinear"))
}

pub(super) fn rotate_document_arbitrary_with(
    mut view: Pin<&mut PictureView>,
    angle_deg: f64,
    interpolation_id: &QString,
) -> bool {
    crate::cxxqt_object::helpers::memory_gate::clear();
    if view.rust().modal_transform_blocks_edit() {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Capability);
    }
    let id = interpolation_id.to_string();
    let Some(interpolation) = pictura_render::Interpolation::from_id(&id) else {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    };
    if !angle_deg.is_finite() || angle_deg == 0.0 || !(-359.99..=359.99).contains(&angle_deg) {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    }
    let admitted = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| crate::cxxqt_object::helpers::rotate_admits(doc, angle_deg));
    if !admitted {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Host);
    }
    // A large document's rotation runs as a cancellable job (#1294).
    view.as_mut().document_op(
        "Rotate",
        JobBar::Counted,
        move |doc| {
            let pivot = (doc.width as f64 / 2.0, doc.height as f64 / 2.0);
            pictura_render::rotate_document_in_with(doc, angle_deg, pivot, interpolation)
                .then(|| Some(Outcome::default()))
                .ok_or_else(|| {
                    pictura_error::Error::new(
                        pictura_error::ErrorCode::Capability,
                        "the document cannot be rotated by this angle",
                    )
                })
        },
        |rust, _| {
            rust.session.clear_geometry_selection();
            Edit::Full
        },
    )
}

pub(super) fn duplicate_name(view: &PictureView) -> QString {
    let base = view.rust().path.as_deref().map_or_else(
        || "Untitled".to_string(),
        |p| {
            std::path::Path::new(p)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        },
    );
    QString::from(format!("{base} copy"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::helpers_composite::buffer_to_rgba_bytes;
    use crate::cxxqt_object::picture::readers_fixture::{full_picture, layered, view_of};

    /// Trim's coverage bounds and Duplicate (merged)'s pixels, read band by
    /// band, equal the whole composite's (#1274).
    #[test]
    fn trim_and_duplicate_merged_stream() {
        let view = view_of(layered(613, 541, false));
        let doc = view.session.doc.as_ref().expect("document");
        let rgba = buffer_to_rgba_bytes(&full_picture(&view));
        let (w, h) = (doc.width as i32, doc.height as i32);
        let (mut left, mut top, mut right, mut bottom) = (w, h, -1, -1);
        for y in 0..h {
            for x in 0..w {
                if rgba[((y * w + x) * 4 + 3) as usize] > 0 {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x + 1);
                    bottom = bottom.max(y + 1);
                }
            }
        }
        assert_eq!(coverage_bounds(doc, &view), (left, top, right, bottom));
        assert!((left, top) != (0, 0), "the fixture has transparent margins");
        assert!(picture_rgba(doc, &view, false) == rgba);
    }
}
