use super::super::helpers::*;
use super::super::qobject;
use super::super::state::TransformMode;
use super::distort::gesture_distort;
use super::geometry::{
    gesture_rotate, gesture_scale, gesture_translate, hit_test_quad, pivot_drag, pivot_hit,
    session_quad, MOVE_HANDLE, PIVOT_HANDLE, ROTATE_HANDLE,
};
use super::perspective::{gesture_perspective, gesture_scale_base};
use super::skew::gesture_skew;
use super::warp::{refresh_warp_preview, warp_hit_test, warp_move, warp_press};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

impl qobject::PictureView {
    /// Begin a drag at document-space `(x, y)`; returns the hit handle
    /// (0..=7 scale, 8 rotate, 9 move) or -1 when nothing was hit.
    pub fn transform_press(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        zoom: f64,
        _shift: bool,
        _alt: bool,
    ) -> i32 {
        let (hit, start_quad) = {
            let rust = self.rust();
            match rust.transform_session.as_ref() {
                Some(session) if session.warp.is_some() => {
                    let hit = warp_hit_test(session, x, y, zoom);
                    (hit, session_quad(session))
                }
                Some(session) => {
                    if pivot_hit(session, x, y, zoom) {
                        (PIVOT_HANDLE, session_quad(session))
                    } else {
                        let quad = session_quad(session);
                        let hit = hit_test_quad(&quad, session.mode.is_projective(), x, y, zoom);
                        (hit, quad)
                    }
                }
                None => return -1,
            }
        };
        if hit < 0 {
            return -1;
        }
        let mut rust = self.as_mut().rust_mut();
        if let Some(session) = rust.transform_session.as_mut() {
            if session.warp.is_some() && warp_press(session, x, y, zoom) < 0 {
                return -1;
            }
            session.handle = hit;
            session.dragging = true;
            session.press_x = x;
            session.press_y = y;
            session.start = [
                session.scale_x,
                session.scale_y,
                session.angle,
                session.dx,
                session.dy,
            ];
            session.start_quad = start_quad;
            session.start_pivot = session.pivot;
            session.free_perspective = false;
            session.perspective_side = None;
        }
        hit
    }

    /// Hover hit-test for cursor selection; no session is mutated.
    pub fn transform_hit_test(&self, x: f64, y: f64, zoom: f64) -> i32 {
        match self.rust().transform_session.as_ref() {
            Some(session) if session.warp.is_some() => warp_hit_test(session, x, y, zoom),
            Some(session) => {
                if pivot_hit(session, x, y, zoom) {
                    return PIVOT_HANDLE;
                }
                let quad = session_quad(session);
                hit_test_quad(&quad, session.mode.is_projective(), x, y, zoom)
            }
            None => -1,
        }
    }

    /// Update the active drag from document-space `(x, y)`. In a projective mode
    /// the matching gesture edits the live quad; in `Free`, Shift locks the
    /// aspect ratio on a corner and snaps rotation to 15°. Returns false without
    /// an active drag.
    pub fn transform_move(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        zoom: f64,
        shift: bool,
        alt: bool,
    ) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let mut rust = self.as_mut().rust_mut();
        let Some(session) = rust.transform_session.as_mut() else {
            return false;
        };
        if !session.dragging {
            return false;
        }
        if session.warp.is_some() {
            if !warp_move(session, x, y) {
                return false;
            }
            return refresh_warp_preview(self);
        }
        if session.handle == PIVOT_HANDLE {
            pivot_drag(session, x, y, zoom);
            return true;
        }
        let mode = session.mode;
        let handle = session.handle;
        match mode {
            TransformMode::Distort => gesture_distort(session, handle, x, y),
            TransformMode::Perspective => gesture_perspective(session, handle, x, y),
            TransformMode::Skew => gesture_skew(session, handle, x, y, alt),
            TransformMode::Free if session.free_perspective => {
                gesture_perspective(session, handle, x, y)
            }
            TransformMode::Free => match handle {
                MOVE_HANDLE => {
                    gesture_translate(session, x, y);
                    true
                }
                ROTATE_HANDLE => {
                    gesture_rotate(session, x, y, shift);
                    true
                }
                0..=7 if session.base_quad.is_some() => gesture_scale_base(session, x, y, shift),
                h @ 0..=7 => {
                    session.handle = h;
                    gesture_scale(session, x, y, shift)
                }
                _ => false,
            },
        }
    }

    /// End the active drag. The session stays open until commit/cancel.
    pub fn transform_release(mut self: Pin<&mut Self>) -> bool {
        let mut rust = self.as_mut().rust_mut();
        let Some(session) = rust.transform_session.as_mut() else {
            return false;
        };
        session.dragging = false;
        session.handle = -1;
        session.warp_drag = None;
        session.free_perspective = false;
        session.perspective_side = None;
        true
    }
}

impl qobject::PictureView {
    pub fn translate_layer(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        let Some(path) = self.as_ref().move_cache_target() else {
            return false;
        };
        let before = self
            .rust()
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path))
            .map(|layer| layer.rect);
        let moved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::translate_layer_path(doc, &path, dx, dy)
        };
        if moved {
            match (
                before,
                self.rust()
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::resolve_path(doc, &path))
                    .map(|l| (l.rect, layer_has_effects(l))),
            ) {
                (Some(b), Some((a, false))) => self.as_mut().refresh_region(union_rect(b, a)),
                _ => self.as_mut().recomposite(),
            }
            self.as_mut().record_move("Move Layer");
        }
        moved
    }

    pub fn move_preview(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }
        let Some(path) = self.as_ref().move_cache_target() else {
            return false;
        };
        let before = self
            .rust()
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path))
            .map(|layer| layer.rect);
        let moved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::translate_layer_path(doc, &path, dx, dy)
        };
        if moved {
            match (
                before,
                self.rust()
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::resolve_path(doc, &path))
                    .map(|l| (l.rect, layer_has_effects(l))),
            ) {
                (Some(b), Some((a, false))) => self.as_mut().refresh_region(union_rect(b, a)),
                _ => self.as_mut().recomposite(),
            }
        }
        moved
    }
}

impl qobject::PictureView {
    pub fn resize_image(mut self: Pin<&mut Self>, kind: &QString, width: i32, height: i32) -> bool {
        let Some(resample) = parse_resample(&kind.to_string()) else {
            return false;
        };
        if width < 1 || height < 1 {
            return false;
        }
        let resized = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::resize_document(doc, width as u32, height as u32, resample).is_ok()
        };
        if resized {
            self.as_mut().rust_mut().selection = None;
            self.as_mut().recomposite();
            self.as_mut().record("Image Size");
        }
        resized
    }

    pub fn resize_canvas(
        mut self: Pin<&mut Self>,
        anchor: &QString,
        width: i32,
        height: i32,
    ) -> bool {
        let Some(anchor) = parse_anchor(&anchor.to_string()) else {
            return false;
        };
        if width < 1 || height < 1 {
            return false;
        }
        let resized = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::resize_canvas_document(doc, width as u32, height as u32, anchor).is_ok()
        };
        if resized {
            self.as_mut().rust_mut().selection = None;
            self.as_mut().recomposite();
            self.as_mut().record("Canvas Size");
        }
        resized
    }

    pub fn rotate_doc(mut self: Pin<&mut Self>, quarter_turns: i32) -> bool {
        if !(1..=3).contains(&quarter_turns) {
            return false;
        }
        let rotated = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::rotate_document(doc, quarter_turns as u8).is_ok()
        };
        if rotated {
            self.as_mut().rust_mut().selection = None;
            self.as_mut().recomposite();
            self.as_mut().record("Rotate");
        }
        rotated
    }

    pub fn flip_doc(mut self: Pin<&mut Self>, horizontal: bool) -> bool {
        {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::flip_document(doc, horizontal);
            rust.selection = None;
        }
        self.as_mut().recomposite();
        self.as_mut().record("Flip");
        true
    }

    /// Convert a 32-bit document to `bits` (16 or 8) through the CS6 HDR
    /// Conversion "Exposure & Gamma" method, clear the selection, recomposite,
    /// and record one "HDR Conversion" history state. Returns false without a
    /// document or when the engine refuses the conversion.
    pub fn convert_depth(
        mut self: Pin<&mut Self>,
        bits: i32,
        exposure_ev: f64,
        gamma: f64,
    ) -> bool {
        let out = match bits {
            16 => pictura_core::BitDepth::Sixteen,
            8 => pictura_core::BitDepth::Eight,
            _ => return false,
        };
        let converted = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::convert_depth_exposure_gamma(
                doc,
                out,
                pictura_render::ExposureGamma { exposure_ev, gamma },
            )
            .is_ok()
        };
        if converted {
            self.as_mut().rust_mut().selection = None;
            self.as_mut().recomposite();
            self.as_mut().record("HDR Conversion");
        }
        converted
    }
}
