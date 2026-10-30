use super::*;

use super::super::super::helpers::{
    active_layer_visible, active_pixel_layer, active_pixel_layer_mut,
};
use super::super::super::helpers_composite::selection_to_mask;
use super::super::super::state::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
#[cfg(test)]
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{layer_pixel_locked, Layer};
use pictura_render::{MatteBackground, MattingError};

/// A matting edit and its parameter.
pub(super) enum MattingOp {
    Remove(MatteBackground),
    Defringe(u32),
}

/// Resolve the active pixel layer and run `op` on it, gated by the selection
/// mask. `None` when there is no document or no single editable active layer;
/// `Some(Err(_))` when the engine refuses the layer.
pub(super) fn apply_matte(
    rust: &mut PictureViewRust,
    op: MattingOp,
) -> Option<Result<(), MattingError>> {
    let active = rust.layer_target.single().map(str::to_string);
    let mask = {
        let doc = rust.session.doc.as_ref()?;
        rust.session
            .selection
            .as_ref()
            .map(|selection| selection_to_mask(selection, doc))
    };
    let doc = rust.session.doc.as_mut()?;
    if !active_layer_visible(doc, active.as_deref()) {
        return None;
    }
    let layer = active_pixel_layer_mut(doc, active.as_deref())?;
    let result = match op {
        MattingOp::Remove(background) => {
            pictura_render::remove_matte(layer, background, mask.as_ref())
        }
        MattingOp::Defringe(width) => pictura_render::defringe(layer, width, mask.as_ref()),
    };
    Some(result)
}

pub(super) fn run(mut view: Pin<&mut PictureView>, op: MattingOp, label: &str) -> bool {
    let edit = view
        .as_mut()
        .edit(label, |rust| match apply_matte(rust, op) {
            Some(Ok(())) => Edit::Full,
            _ => Edit::None,
        });
    matches!(edit, Edit::Full)
}

pub(super) fn layer_remove_black_matte(view: Pin<&mut PictureView>) -> bool {
    run(
        view,
        MattingOp::Remove(MatteBackground::Black),
        "Remove Black Matte",
    )
}

pub(super) fn layer_remove_white_matte(view: Pin<&mut PictureView>) -> bool {
    run(
        view,
        MattingOp::Remove(MatteBackground::White),
        "Remove White Matte",
    )
}

pub(super) fn layer_defringe(view: Pin<&mut PictureView>, width: i32) -> bool {
    run(view, MattingOp::Defringe(width.max(1) as u32), "Defringe")
}

pub(super) fn matting_target_ready(view: &PictureView) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let active = rust.layer_target.single();
    active_layer_visible(doc, active)
        && active_pixel_layer(doc, active)
            .is_some_and(|layer| !layer_pixel_locked(layer) && has_colour(layer))
}

/// Whether `layer` carries an editable colour plane (channel 0 full-length over
/// a non-empty rect) — the engine's own emptiness test.
pub(super) fn has_colour(layer: &Layer) -> bool {
    let (w, h) = (layer.rect.width(), layer.rect.height());
    if w <= 0 || h <= 0 {
        return false;
    }
    let n = w as usize * h as usize;
    layer
        .channels
        .iter()
        .any(|c| c.id == 0 && c.data.len() == n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, LockFlags, PsdRect};

    fn pixel_layer(w: i32, h: i32, colour: [u8; 3], alpha: Vec<u8>) -> Layer {
        let n = (w * h) as usize;
        let plane = |v: u8| vec![v; n];
        Layer {
            name: "px".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h,
                right: w,
            },
            channels: vec![
                pictura_core::Channel {
                    id: 0,
                    data: plane(colour[0]).into(),
                },
                pictura_core::Channel {
                    id: 1,
                    data: plane(colour[1]).into(),
                },
                pictura_core::Channel {
                    id: 2,
                    data: plane(colour[2]).into(),
                },
                pictura_core::Channel {
                    id: -1,
                    data: alpha.into(),
                },
            ],
            ..Default::default()
        }
    }

    fn state(layer: Layer) -> PictureViewRust {
        let mut doc = pictura_core::Document::new(4, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            layer_target: LayerTarget::only("0"),
            ..Default::default()
        }
    }

    fn red(rust: &PictureViewRust) -> Vec<u8> {
        rust.session.doc.as_ref().unwrap().layers[0]
            .channels
            .iter()
            .find(|c| c.id == 0)
            .unwrap()
            .data
            .to_vec()
    }

    #[test]
    fn remove_black_matte_applies_and_locked_is_refused() {
        let mut rust = state(pixel_layer(4, 1, [100, 100, 100], vec![128; 4]));
        assert!(matches!(
            apply_matte(&mut rust, MattingOp::Remove(MatteBackground::Black)),
            Some(Ok(()))
        ));
        assert!(red(&rust).iter().all(|&v| v > 100), "un-matted");

        let mut locked = state(pixel_layer(4, 1, [100, 100, 100], vec![128; 4]));
        locked.session.doc.as_mut().unwrap().layers[0].lock =
            LockFlags::default().with(LockFlags::PIXELS, true);
        let before = red(&locked);
        assert!(matches!(
            apply_matte(&mut locked, MattingOp::Remove(MatteBackground::White)),
            Some(Err(MattingError::Locked))
        ));
        assert_eq!(red(&locked), before, "refusal leaves pixels unchanged");
    }

    #[test]
    fn defringe_replaces_the_edge_over_a_semi_transparent_row() {
        // Transparent, semi, semi, transparent; interior colour is index 1.
        let mut rust = state(pixel_layer(4, 1, [10, 10, 10], vec![0, 255, 255, 0]));
        for c in rust.session.doc.as_mut().unwrap().layers[0]
            .channels
            .iter_mut()
            .filter(|c| c.id == 0)
        {
            c.data = vec![10, 10, 10, 10].into();
        }
        assert!(matches!(
            apply_matte(&mut rust, MattingOp::Defringe(1)),
            Some(Ok(()))
        ));
        assert_eq!(red(&rust)[1], 10, "index 1 is interior and unchanged");
        assert_eq!(red(&rust)[2], 10, "index 2 band takes the interior colour");
    }

    #[test]
    fn no_active_layer_is_refused() {
        let mut rust = state(pixel_layer(4, 1, [100, 100, 100], vec![128; 4]));
        rust.layer_target.clear();
        assert!(apply_matte(&mut rust, MattingOp::Remove(MatteBackground::Black)).is_none());
    }
}
