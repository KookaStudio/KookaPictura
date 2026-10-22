//! `Type > Warp Text`: free functions over a [`PictureView`] for the Warp Text
//! dialog. A preview re-sets the type layer with the new warp and records
//! nothing; the first preview stashes the layer as it was, and Cancel puts
//! that copy back, so an opened layer's own pixels survive a cancelled
//! dialog. Commit records one "Warp Text" state over the pre-dialog state.
//!
//! [`PictureView`]: crate::cxxqt_object::qobject::PictureView

use super::{ffi::TypeSetting, layer_spec};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::warp::{TextWarp, WarpStyle};
use pictura_render::WarpRefusal;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether the layer at `path` is type carrying a `Type > Warp Text` warp (the Layers panel's warped row).
        fn type_layer_is_warped(view: &PictureView, path: &QString) -> bool;

        /// Whether Warp Text may warp the type layer at `path`: 0 yes, 1 type on a path, 2 Faux Bold, 3 a font without outlines, -1 not a type layer.
        fn type_warp_refusal(view: &PictureView, path: &QString) -> i32;

        /// Re-set the type layer at `path` with the warp (`style` a `WarpStyle` id, `none` or empty for none; percentages) for the dialog's live preview, recording no state. False (unchanged) for a missing or refused layer.
        fn type_warp_preview(
            view: Pin<&mut PictureView>,
            path: &QString,
            style: &QString,
            bend: f64,
            h: f64,
            v: f64,
            vertical: bool,
        ) -> bool;

        /// Put back the layer the dialog's first preview stashed, recording no state. False when nothing was previewed.
        fn type_warp_cancel(view: Pin<&mut PictureView>) -> bool;

        /// Re-set the type layer at `path` with the warp and record one "Warp Text" state. False (no state) for a missing or refused layer.
        fn type_warp_commit(
            view: Pin<&mut PictureView>,
            path: &QString,
            style: &QString,
            bend: f64,
            h: f64,
            v: f64,
            vertical: bool,
        ) -> bool;
    }
}

/// The warp a [`TypeSetting`] carries.
pub(super) fn text_warp(setting: &TypeSetting) -> TextWarp {
    warp_of(
        &setting.warp_style,
        setting.warp_bend,
        setting.warp_h,
        setting.warp_v,
        setting.warp_vertical,
    )
}

fn warp_of(style: &QString, bend: f64, h: f64, v: f64, vertical: bool) -> TextWarp {
    match WarpStyle::parse(&style.to_string()).filter(|s| s.is_preset()) {
        Some(style) => TextWarp {
            style,
            bend,
            h_distort: h,
            v_distort: v,
            vertical,
        },
        None => TextWarp::default(),
    }
}

fn type_layer_is_warped(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
        .and_then(|layer| layer.type_tool.as_ref())
        .is_some_and(|tool| !pictura_codec::decode_type_warp(&tool.warp_desc).is_identity())
}

fn type_warp_refusal(view: &PictureView, path: &QString) -> i32 {
    layer_spec(view, path).map_or(-1, |spec| match pictura_render::type_warp_refusal(&spec) {
        None => 0,
        Some(WarpRefusal::OnPath) => 1,
        Some(WarpRefusal::FauxBold) => 2,
        Some(WarpRefusal::NoOutlines) => 3,
    })
}

/// Re-set the layer at `path` with `warp`. A refused layer can only lose a
/// warp (Style None), never gain one.
fn set_warp(mut view: Pin<&mut PictureView>, path: &QString, warp: TextWarp) -> bool {
    let Some(mut spec) = layer_spec(view.as_ref().get_ref(), path) else {
        return false;
    };
    if !warp.is_identity() && pictura_render::type_warp_refusal(&spec).is_some() {
        return false;
    }
    spec.warp = warp;
    let path = path.to_string();
    let mut rust = view.as_mut().rust_mut();
    let Some(doc) = rust.session.doc.as_mut() else {
        return false;
    };
    let original = pictura_render::resolve_path(doc, &path).cloned();
    if !pictura_render::replace_type_layer(doc, &path, &spec) {
        return false;
    }
    if rust
        .warp_text_original
        .as_ref()
        .is_none_or(|(stashed, _)| *stashed != path)
    {
        rust.warp_text_original = original.map(|layer| (path, layer));
    }
    true
}

fn present(mut view: Pin<&mut PictureView>) {
    view.as_mut().clear_link_sets();
    view.as_mut().recomposite();
    view.as_mut().changed();
}

fn type_warp_preview(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    style: &QString,
    bend: f64,
    h: f64,
    v: f64,
    vertical: bool,
) -> bool {
    let changed = set_warp(view.as_mut(), path, warp_of(style, bend, h, v, vertical));
    if changed {
        present(view);
    }
    changed
}

fn type_warp_cancel(mut view: Pin<&mut PictureView>) -> bool {
    let restored = {
        let mut rust = view.as_mut().rust_mut();
        let Some((path, layer)) = rust.warp_text_original.take() else {
            return false;
        };
        rust.session
            .doc
            .as_mut()
            .and_then(|doc| pictura_render::resolve_path_mut(doc, &path))
            .map(|slot| *slot = layer)
            .is_some()
    };
    if restored {
        present(view);
    }
    restored
}

fn type_warp_commit(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    style: &QString,
    bend: f64,
    h: f64,
    v: f64,
    vertical: bool,
) -> bool {
    let committed = set_warp(view.as_mut(), path, warp_of(style, bend, h, v, vertical));
    view.as_mut().rust_mut().warp_text_original = None;
    if committed {
        view.as_mut().edit("Warp Text", |_| Edit::Structural);
    }
    committed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_of_reads_presets_only() {
        let arc = warp_of(&QString::from("arc"), 50.0, 1.0, 2.0, true);
        assert_eq!(arc.style, WarpStyle::Arc);
        assert_eq!((arc.bend, arc.h_distort, arc.v_distort), (50.0, 1.0, 2.0));
        assert!(arc.vertical);
        for none in ["", "none", "custom", "bogus"] {
            assert_eq!(
                warp_of(&QString::from(none), 50.0, 0.0, 0.0, false),
                TextWarp::default()
            );
        }
    }
}
