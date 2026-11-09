use super::*;

use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::{active_pixel_layer, layer_has_effects, mask_editing};
use crate::cxxqt_object::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{BlendMode, ColorMode, Layer, PsdRect};
use pictura_filters::fade::{fade_layer, Fade};

/// The last destructive effect's pixels and the current Fade values.
#[derive(Clone)]
pub(in crate::cxxqt_object) struct FadeState {
    pub(super) path: String,
    pub(super) label: String,
    pub(super) original: Layer,
    pub(super) effected: Layer,
    pub(super) current: Layer,
    pub(super) opacity: u8,
    pub(super) mode: BlendMode,
}

/// The pre-effect layer of an effect that has not committed yet.
pub(in crate::cxxqt_object) struct FadeCapture {
    pub(in crate::cxxqt_object) path: String,
    pub(in crate::cxxqt_object) original: Layer,
}

/// The two layers a worker commit produced, ready to become a record.
pub(in crate::cxxqt_object) struct FadeSnapshot {
    pub(in crate::cxxqt_object) path: String,
    pub(in crate::cxxqt_object) label: String,
    pub(in crate::cxxqt_object) original: Layer,
    pub(in crate::cxxqt_object) effected: Layer,
}

/// A computed Fade commit: the history label, the refreshed region, and the
/// record to install after the edit transaction clears the old one.
pub(in crate::cxxqt_object) struct AppliedFade {
    pub(in crate::cxxqt_object) label: String,
    pub(in crate::cxxqt_object) region: PsdRect,
    pub(in crate::cxxqt_object) full: bool,
    pub(in crate::cxxqt_object) state: FadeState,
}

/// Capture the active pixel layer before an effect changes it. `None` for a
/// mask/Quick Mask target, a Smart Object, or no single pixel layer.
pub(in crate::cxxqt_object) fn capture(rust: &PictureViewRust) -> Option<FadeCapture> {
    if rust.session.quick_mask.active || mask_editing(rust) {
        return None;
    }
    let doc = rust.session.doc.as_ref()?;
    let path = rust.layer_target.single()?.to_string();
    let layer = active_pixel_layer(doc, Some(&path))?;
    if layer.smart_object.is_some() || layer.is_group || layer.adjustment.is_some() {
        return None;
    }
    Some(FadeCapture {
        path,
        original: layer.clone(),
    })
}

/// Record a committed effect's pre- and post-effect pixels.
pub(in crate::cxxqt_object) fn record(
    rust: &mut PictureViewRust,
    capture: FadeCapture,
    label: &str,
) {
    let Some(doc) = rust.session.doc.as_ref() else {
        return;
    };
    let Some(effected) = pictura_render::resolve_path(doc, &capture.path).cloned() else {
        return;
    };
    record_layers(rust, capture.path, label, capture.original, effected);
}

/// Install a worker commit's record.
pub(in crate::cxxqt_object) fn record_snapshot(rust: &mut PictureViewRust, snapshot: FadeSnapshot) {
    record_layers(
        rust,
        snapshot.path,
        &snapshot.label,
        snapshot.original,
        snapshot.effected,
    );
}

fn record_layers(
    rust: &mut PictureViewRust,
    path: String,
    label: &str,
    original: Layer,
    effected: Layer,
) {
    if effected.smart_object.is_some() || effected.is_group || effected.adjustment.is_some() {
        return;
    }
    if original == effected {
        return;
    }
    rust.fade = Some(FadeState {
        path,
        label: label.to_string(),
        original,
        effected: effected.clone(),
        current: effected,
        opacity: 100,
        mode: BlendMode::Normal,
    });
}

pub(in crate::cxxqt_object) fn clear(rust: &mut PictureViewRust) {
    rust.fade = None;
}

pub(in crate::cxxqt_object) fn available(rust: &PictureViewRust) -> bool {
    rust.fade.is_some() && !rust.has_active_session() && !rust.modal_transform_blocks_edit()
}

pub(in crate::cxxqt_object) fn name(rust: &PictureViewRust) -> String {
    rust.fade
        .as_ref()
        .map_or_else(String::new, |state| state.label.clone())
}

pub(in crate::cxxqt_object) fn opacity(rust: &PictureViewRust) -> u8 {
    rust.fade.as_ref().map_or(100, |state| state.opacity)
}

pub(in crate::cxxqt_object) fn mode(rust: &PictureViewRust) -> BlendMode {
    rust.fade
        .as_ref()
        .map_or(BlendMode::Normal, |state| state.mode)
}

pub(in crate::cxxqt_object) fn mode_allowed(rust: &PictureViewRust, mode: BlendMode) -> bool {
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    if pictura_render::document_color_mode(doc) != ColorMode::Lab {
        return true;
    }
    !matches!(
        mode,
        BlendMode::ColorDodge
            | BlendMode::ColorBurn
            | BlendMode::Lighten
            | BlendMode::Darken
            | BlendMode::Difference
            | BlendMode::Exclusion
    )
}

pub(in crate::cxxqt_object) fn preview(
    rust: &mut PictureViewRust,
    opacity: u8,
    mode: BlendMode,
) -> Option<(PsdRect, bool)> {
    if !mode_allowed(rust, mode) {
        return None;
    }
    let (path, faded, full) = {
        let state = rust.fade.as_ref()?;
        let faded = fade_layer(&state.original, &state.effected, &Fade { opacity, mode });
        let full = layer_has_effects(&faded);
        (state.path.clone(), faded, full)
    };
    install(rust, &path, faded).map(|rect| (rect, full))
}

pub(in crate::cxxqt_object) fn cancel(rust: &mut PictureViewRust) -> Option<(PsdRect, bool)> {
    let (path, current, full) = {
        let state = rust.fade.as_ref()?;
        let current = state.current.clone();
        let full = layer_has_effects(&current);
        (state.path.clone(), current, full)
    };
    install(rust, &path, current).map(|rect| (rect, full))
}

pub(in crate::cxxqt_object) fn apply(
    rust: &mut PictureViewRust,
    opacity: u8,
    mode: BlendMode,
) -> Option<AppliedFade> {
    if !mode_allowed(rust, mode) {
        return None;
    }
    let (path, faded, mut next) = {
        let state = rust.fade.as_ref()?;
        (
            state.path.clone(),
            fade_layer(&state.original, &state.effected, &Fade { opacity, mode }),
            state.clone(),
        )
    };
    let full = layer_has_effects(&faded);
    let region = install(rust, &path, faded.clone())?;
    next.current = faded;
    next.opacity = opacity;
    next.mode = mode;
    Some(AppliedFade {
        label: format!("Fade {}", next.label),
        region,
        full,
        state: next,
    })
}

fn install(rust: &mut PictureViewRust, path: &str, layer: Layer) -> Option<PsdRect> {
    let doc = rust.session.doc.as_mut()?;
    let slot = pictura_render::resolve_path_mut(doc, path)?;
    let rect = layer.rect;
    *slot = layer;
    Some(rect)
}

fn parse_mode(key: &str) -> Option<BlendMode> {
    let bytes: [u8; 4] = key.as_bytes().try_into().ok()?;
    BlendMode::from_psd_key(bytes)
}

fn mode_key(mode: BlendMode) -> String {
    String::from_utf8_lossy(&mode.to_psd_key()).into_owned()
}

pub(super) fn fade_available(view: &PictureView) -> bool {
    available(view.rust())
}

pub(super) fn fade_name(view: &PictureView) -> QString {
    QString::from(name(view.rust()))
}

pub(super) fn fade_opacity(view: &PictureView) -> i32 {
    i32::from(opacity(view.rust()))
}

pub(super) fn fade_mode(view: &PictureView) -> QString {
    QString::from(mode_key(mode(view.rust())))
}

pub(super) fn fade_mode_allowed(view: &PictureView, key: &QString) -> bool {
    parse_mode(&key.to_string()).is_some_and(|mode| mode_allowed(view.rust(), mode))
}

pub(super) fn fade_preview(mut view: Pin<&mut PictureView>, opacity: i32, mode: &QString) -> bool {
    let Some(mode) = parse_mode(&mode.to_string()) else {
        return false;
    };
    let opacity = opacity.clamp(0, 100) as u8;
    let region = {
        let mut rust = view.as_mut().rust_mut();
        preview(&mut rust, opacity, mode)
    };
    let Some((rect, full)) = region else {
        return false;
    };
    view.preview(|_| {
        if full {
            Edit::Full
        } else {
            Edit::regions(vec![rect], false)
        }
    });
    true
}

pub(super) fn fade_apply(mut view: Pin<&mut PictureView>, opacity: i32, mode: &QString) -> bool {
    let Some(mode) = parse_mode(&mode.to_string()) else {
        return false;
    };
    let opacity = opacity.clamp(0, 100) as u8;
    let Some(AppliedFade {
        label,
        region,
        full,
        state,
    }) = ({
        let mut rust = view.as_mut().rust_mut();
        apply(&mut rust, opacity, mode)
    })
    else {
        return false;
    };
    view.as_mut().edit(&label, |_| {
        if full {
            Edit::Full
        } else {
            Edit::regions(vec![region], false)
        }
    });
    view.as_mut().rust_mut().fade = Some(state);
    true
}

pub(super) fn fade_cancel(mut view: Pin<&mut PictureView>) -> bool {
    let region = {
        let mut rust = view.as_mut().rust_mut();
        cancel(&mut rust)
    };
    let Some((rect, full)) = region else {
        return false;
    };
    view.preview(|_| {
        if full {
            Edit::Full
        } else {
            Edit::regions(vec![rect], false)
        }
    });
    true
}

#[cfg(test)]
mod tests;
