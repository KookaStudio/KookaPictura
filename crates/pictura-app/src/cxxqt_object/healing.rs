//! The healing bridges: Spot Healing Brush, Healing Brush, Patch,
//! Content-Aware Move, and Red Eye. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). A gesture accumulates a coverage mask
//! ([`HealStroke`]); `healing_commit` runs the solve on the layer, recomposites,
//! and records one `"Spot Healing Brush"` / `"Healing Brush"` history state.
//! `patch_selection` heals through the selection instead of a stroke mask and
//! records one `"Patch Tool"` state; `content_aware_move` likewise records one
//! `"Content-Aware Move"` state, and `red_eye` one `"Red Eye Tool"` state.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;
use pictura_paint::healing::{
    move_layer, patch_layer, red_eye_layer, Adaptation, HealMode, HealStroke, MoveOptions,
    PatchOptions, Transfer,
};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start a healing gesture on the active pixel layer at `diameter` /
        /// `hardness`. False without a lone active raster layer or when its
        /// pixels are locked.
        fn healing_begin(view: Pin<&mut PictureView>, diameter: i32, hardness: i32) -> bool;

        /// Add a dab at document-space `(x, y)`. False while no gesture is
        /// active.
        fn healing_dab(view: Pin<&mut PictureView>, x: f64, y: f64) -> bool;

        /// Run the heal on release and commit it. `mode` is 0/1/2 for the Spot
        /// Healing types; for the Healing Brush pass the sample offset in
        /// `source_x`/`source_y` and `use_source` true. Records one history
        /// state; false when nothing was healed.
        fn healing_commit(
            view: Pin<&mut PictureView>,
            mode: i32,
            use_source: bool,
            source_x: i32,
            source_y: i32,
            texture_only: bool,
        ) -> bool;

        /// Drop an active gesture without committing (Escape, tool switch).
        fn healing_cancel(view: Pin<&mut PictureView>);

        /// Whether a gesture is active.
        fn healing_active(view: &PictureView) -> bool;

        /// Patch the active pixel layer through the selection after a drag of
        /// `(dx, dy)`: Source repairs the selection from the dragged-to area,
        /// Destination applies the selection there, Content-Aware ignores the
        /// drag and rebuilds the selection from its surroundings. Records one
        /// "Patch Tool" state; false (no state) without a selection, on a zero
        /// drag outside Content-Aware, or on a locked / non-raster layer.
        fn patch_selection(
            view: Pin<&mut PictureView>,
            dx: i32,
            dy: i32,
            content_aware: bool,
            destination: bool,
            transparent: bool,
        ) -> bool;

        /// Move the selection's pixels on the active pixel layer by `(dx, dy)`
        /// and rebuild the hole from its surroundings, or with `extend` copy
        /// them and keep the original. `adaptation` is 0 Very Strict … 4 Very
        /// Loose. The selection follows the pixels. Records one "Content-Aware
        /// Move" state; false (no state) without a selection, on a zero drag,
        /// or on a locked / non-raster layer.
        fn content_aware_move(
            view: Pin<&mut PictureView>,
            dx: i32,
            dy: i32,
            extend: bool,
            adaptation: i32,
        ) -> bool;

        /// Neutralise red-eye on the active pixel layer inside the document
        /// box `(x, y, width, height)`. `pupil` is Pupil Size and `darken`
        /// Darken Amount, both 0–100. Records one "Red Eye Tool" state; false
        /// (no state) when the box holds no red or the layer is locked or not
        /// a pixel layer.
        fn red_eye(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            pupil: i32,
            darken: i32,
        ) -> bool;
    }
}

fn healing_begin(mut view: Pin<&mut PictureView>, diameter: i32, hardness: i32) -> bool {
    view.as_mut().rust_mut().begin_heal(diameter, hardness)
}

impl super::state::PictureViewRust {
    /// Begin a healing gesture on the active layer. False without a document, a
    /// layer target, or when another session owns the slot.
    pub(super) fn begin_heal(&mut self, diameter: i32, hardness: i32) -> bool {
        if !self.slot_allows(super::document_session::SessionKind::Healing) {
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
                self.heal_stroke = Some(stroke);
                true
            }
            Err(_) => false,
        }
    }
}

fn healing_dab(mut view: Pin<&mut PictureView>, x: f64, y: f64) -> bool {
    let mut rust = view.as_mut().rust_mut();
    let Some(stroke) = rust.heal_stroke.as_mut() else {
        return false;
    };
    // A dab only grows the mask; the pixels change on commit.
    stroke.dab(x as f32, y as f32);
    true
}

fn healing_commit(
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
        let stroke = view.as_mut().rust_mut().heal_stroke.take();
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
    view.as_mut().edit(label, |_| Edit::Regions {
        rects: vec![outcome.dirty],
        notify: true,
    });
    true
}

fn healing_cancel(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().heal_stroke = None;
}

fn healing_active(view: &PictureView) -> bool {
    view.rust().heal_stroke.is_some()
}

fn patch_selection(
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
    view.as_mut().edit("Patch Tool", |_| Edit::Regions {
        rects: vec![dirty],
        notify: true,
    });
    true
}

fn content_aware_move(
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
    view.as_mut().edit("Content-Aware Move", |_| Edit::Regions {
        rects: vec![dirty],
        notify: true,
    });
    true
}

fn red_eye(
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
    view.as_mut().edit("Red Eye Tool", |_| Edit::Regions {
        rects: vec![dirty],
        notify: true,
    });
    true
}
