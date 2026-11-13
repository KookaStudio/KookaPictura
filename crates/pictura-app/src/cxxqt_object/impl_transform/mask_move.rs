use super::super::helpers::memory_gate;
use super::super::state::PictureViewRust;
use super::super::{
    helpers::active_mask_target,
    helpers_composite::{buffer_to_image, document_to_image, rgba_image},
    qobject,
};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;

pub(super) fn target(rust: &PictureViewRust) -> Option<String> {
    let path = active_mask_target(rust)?;
    let doc = rust.session.doc.as_ref()?;
    let layer = pictura_render::resolve_path(doc, &path)?;
    (layer.visible && !pictura_render::layer_mask_linked(doc, &path)).then_some(path)
}

impl qobject::PictureView {
    pub(super) fn begin_mask_move(mut self: Pin<&mut Self>, path: String) -> bool {
        self.as_mut().settle_pending_composite();
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return false;
        };
        let Some(layer) = pictura_render::resolve_path(doc, &path).cloned() else {
            return false;
        };
        self.as_mut().rust_mut().mask_move_baseline = Some((path, layer));
        if !self.as_mut().render_mask_move_preview(0, 0) {
            self.as_mut().rust_mut().mask_move_baseline = None;
            return false;
        }
        self.as_mut().rust_mut().move_drag_active = true;
        true
    }

    pub(super) fn render_mask_move_preview(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        let (Some(origin_x), Some(origin_y)) = (dx.checked_neg(), dy.checked_neg()) else {
            return false;
        };
        let rust = self.rust();
        let Some((path, original)) = rust.mask_move_baseline.as_ref() else {
            return false;
        };
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let Ok(_held) = pictura_core::memory::governor().reserve(
            u64::from(doc.width) * u64::from(doc.height) * 16,
            "mask move preview",
        ) else {
            return false;
        };
        let mut candidate = doc.clone();
        let Some(layer) = pictura_render::resolve_path_mut(&mut candidate, path) else {
            return false;
        };
        *layer = original.clone();
        if (dx != 0 || dy != 0)
            && !pictura_render::translate_layer_mask(&mut candidate, path, dx, dy)
        {
            return false;
        }
        let image = if rust.mask_solo.as_deref() == Some(path.as_str()) {
            let Some(mask) = pictura_render::resolve_path(&candidate, path)
                .and_then(|layer| layer.mask.as_ref())
            else {
                return false;
            };
            buffer_to_image(&super::super::impl_core::mask_display_plane(
                mask,
                candidate.width,
                candidate.height,
            ))
        } else {
            document_to_image(&candidate, false)
        };
        let base = rgba_image(
            vec![0; candidate.width as usize * candidate.height as usize * 4],
            candidate.width as i32,
            candidate.height as i32,
        );
        let mut state = self.as_mut().rust_mut();
        state.move_base = Some(base);
        state.move_base_origin = None;
        state.move_view = None;
        state.move_view_pixels = Default::default();
        state.move_layer = Some(image);
        state.move_x = origin_x;
        state.move_y = origin_y;
        state.move_opacity = 255;
        state.move_preview_cache_hit = false;
        true
    }

    pub(super) fn end_mask_move(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().mask_move_baseline = None;
    }

    pub(super) fn commit_mask_move(
        mut self: Pin<&mut Self>,
        path: String,
        dx: i32,
        dy: i32,
    ) -> bool {
        self.as_mut().end_mask_move();
        let outcome = self.as_mut().edit("Move Layer", |rust| {
            let Some(doc) = rust.session.doc.as_ref() else {
                return Edit::None;
            };
            let canvas = PsdRect {
                left: 0,
                top: 0,
                right: doc.width as i32,
                bottom: doc.height as i32,
            };
            if !memory_gate::admit(12 * memory_gate::area(canvas)) {
                return Edit::None;
            }
            let mut candidate = doc.clone();
            if !pictura_render::translate_layer_mask(&mut candidate, &path, dx, dy) {
                return Edit::None;
            }
            rust.session.doc = Some(candidate);
            Edit::regions(vec![canvas], false)
        });
        if matches!(outcome, Edit::None) {
            return false;
        }
        self.as_mut().move_committed();
        true
    }
}
