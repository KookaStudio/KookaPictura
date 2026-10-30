use super::*;

use super::super::helpers::active_mask_target;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;
use pictura_paint::healing::{
    move_layer, patch_layer, red_eye_layer, Adaptation, HealMode, HealStroke, MoveOptions,
    PatchOptions, Transfer,
};

pub(super) fn healing_begin(mut view: Pin<&mut PictureView>, diameter: i32, hardness: i32) -> bool {
    view.as_mut().rust_mut().begin_heal(diameter, hardness)
}

impl super::super::state::PictureViewRust {
    /// Begin a healing gesture on the active layer. False without a document, a
    /// layer target, or when another session owns the slot.
    pub(in crate::cxxqt_object) fn begin_heal(&mut self, diameter: i32, hardness: i32) -> bool {
        if active_mask_target(self).is_some() {
            return false;
        }
        if !self.slot_allows(super::super::document_session::SessionKind::Healing) {
            return false;
        }
        let begun = {
            let rust = &*self;
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let Some(path) = rust.layer_target.single() else {
                return false;
            };
            HealStroke::begin(
                doc,
                path,
                diameter.max(1) as u32,
                hardness.clamp(0, 100) as u8,
            )
        };
        match begun {
            Ok(stroke) => {
                self.session.live.set_heal_stroke(stroke);
                true
            }
            Err(_) => false,
        }
    }
}

pub(super) fn healing_dab(mut view: Pin<&mut PictureView>, x: f64, y: f64) -> bool {
    let mut rust = view.as_mut().rust_mut();
    let Some(stroke) = rust.session.live.heal_stroke_mut() else {
        return false;
    };
    // A dab only grows the mask; the pixels change on commit.
    stroke.dab(x as f32, y as f32);
    true
}

pub(super) fn healing_commit(
    mut view: Pin<&mut PictureView>,
    mode: i32,
    use_source: bool,
    source_x: i32,
    source_y: i32,
    texture_only: bool,
) -> bool {
    let source = use_source.then_some((source_x, source_y));
    let transfer = if texture_only {
        Transfer::TextureOnly
    } else {
        Transfer::Full
    };
    let heal_mode = HealMode::from_i32(mode).unwrap_or_default();
    let outcome = {
        let stroke = view.as_mut().rust_mut().session.live.take_heal_stroke();
        let Some(stroke) = stroke else {
            return false;
        };
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        match stroke.commit(doc, heal_mode, source, transfer) {
            Ok(outcome) => outcome,
            Err(_) => return false,
        }
    };
    let Some(outcome) = outcome else {
        return false;
    };
    let label = if source.is_some() {
        "Healing Brush"
    } else {
        "Spot Healing Brush"
    };
    {
        let mut rust = view.as_mut().rust_mut();
        rust.session.doc = Some(outcome.document);
    }
    view.as_mut()
        .edit(label, |_| Edit::regions(vec![outcome.dirty], true));
    true
}

pub(super) fn healing_cancel(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().session.live.clear_heal_stroke();
}

pub(super) fn healing_active(view: &PictureView) -> bool {
    view.rust().session.live.heal_stroke().is_some()
}

pub(super) fn patch_selection(
    mut view: Pin<&mut PictureView>,
    dx: i32,
    dy: i32,
    content_aware: bool,
    destination: bool,
    transparent: bool,
) -> bool {
    let options = PatchOptions {
        dx,
        dy,
        content_aware,
        destination,
        transparent,
    };
    let dirty = {
        let mut rust = view.as_mut().rust_mut();
        let rust = &mut *rust;
        let (Some(doc), Some(selection), Some(path)) = (
            rust.session.doc.as_mut(),
            rust.session.selection.as_ref(),
            rust.layer_target.single(),
        ) else {
            return false;
        };
        match patch_layer(doc, path, &selection.data, options) {
            Ok(Some(dirty)) => dirty,
            _ => return false,
        }
    };
    view.as_mut()
        .edit("Patch Tool", |_| Edit::regions(vec![dirty], true));
    true
}

pub(super) fn content_aware_move(
    mut view: Pin<&mut PictureView>,
    dx: i32,
    dy: i32,
    extend: bool,
    adaptation: i32,
) -> bool {
    let options = MoveOptions {
        dx,
        dy,
        extend,
        adaptation: Adaptation::from_i32(adaptation).unwrap_or_default(),
    };
    let dirty = {
        let mut rust = view.as_mut().rust_mut();
        let rust = &mut *rust;
        let (Some(doc), Some(selection), Some(path)) = (
            rust.session.doc.as_mut(),
            rust.session.selection.as_mut(),
            rust.layer_target.single(),
        ) else {
            return false;
        };
        match move_layer(doc, path, &selection.data, options) {
            Ok(Some(dirty)) => {
                *selection = selection.translate(dx, dy);
                dirty
            }
            _ => return false,
        }
    };
    view.as_mut().rust_mut().session.touch_selection();
    view.as_mut()
        .edit("Content-Aware Move", |_| Edit::regions(vec![dirty], true));
    true
}

pub(super) fn red_eye(
    mut view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    pupil: i32,
    darken: i32,
) -> bool {
    let rect = PsdRect {
        top: y,
        left: x,
        bottom: y + height.max(0),
        right: x + width.max(0),
    };
    let dirty = {
        let mut rust = view.as_mut().rust_mut();
        let rust = &mut *rust;
        let (Some(doc), Some(path)) = (rust.session.doc.as_mut(), rust.layer_target.single())
        else {
            return false;
        };
        let (pupil, darken) = (pupil.clamp(0, 100) as u32, darken.clamp(0, 100) as u32);
        match red_eye_layer(doc, path, rect, pupil, darken) {
            Ok(Some(dirty)) => dirty,
            _ => return false,
        }
    };
    view.as_mut()
        .edit("Red Eye Tool", |_| Edit::regions(vec![dirty], true));
    true
}
