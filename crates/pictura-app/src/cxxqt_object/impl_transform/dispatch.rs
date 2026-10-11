use super::super::helpers::*;
use super::super::qobject;
use super::super::state::TransformMode;
use super::geometry::{
    gesture_distort, gesture_perspective, gesture_rotate, gesture_scale, gesture_skew,
    gesture_translate, hit_test_quad, session_quad, MOVE_HANDLE, ROTATE_HANDLE,
};
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
                Some(session) => {
                    let quad = session_quad(session);
                    let hit = hit_test_quad(&quad, session.mode.is_projective(), x, y, zoom);
                    (hit, quad)
                }
                None => return -1,
            }
        };
        if hit < 0 {
            return -1;
        }
        let mut rust = self.as_mut().rust_mut();
        if let Some(session) = rust.transform_session.as_mut() {
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
        }
        hit
    }

    /// Hover hit-test for cursor selection; no session is mutated.
    pub fn transform_hit_test(&self, x: f64, y: f64, zoom: f64) -> i32 {
        match self.rust().transform_session.as_ref() {
            Some(session) => {
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
        _zoom: f64,
        shift: bool,
        _alt: bool,
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
        let mode = session.mode;
        let handle = session.handle;
        match mode {
            TransformMode::Distort => gesture_distort(session, handle, x, y),
            TransformMode::Perspective => gesture_perspective(session, handle, x, y),
            TransformMode::Skew => gesture_skew(session, handle, x, y, shift),
            TransformMode::Free => match handle {
                MOVE_HANDLE => {
                    gesture_translate(session, x, y);
                    true
                }
                ROTATE_HANDLE => {
                    gesture_rotate(session, x, y, shift);
                    true
                }
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
        true
    }

    /// The session quad as `"x,y x,y x,y x,y"` (four document-space corners):
    /// the live projective quad in a projective mode, else the similarity quad.
    pub fn transform_quad(&self) -> QString {
        let Some(session) = self.rust().transform_session.as_ref() else {
            return QString::default();
        };
        let quad = session_quad(session);
        let encoded: Vec<String> = quad.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
        QString::from(encoded.join(" "))
    }

    /// The layer at `path`'s rect as `"left top right bottom"`, or empty.
    pub fn layer_rect(&self, path: &QString) -> QString {
        let rect = self
            .rust()
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
            .map(|layer| layer.rect);
        rect.map_or_else(QString::default, |rect| {
            QString::from(format!(
                "{} {} {} {}",
                rect.left, rect.top, rect.right, rect.bottom
            ))
        })
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

    /// Apply a named warp preset to the layer at `path`: build the style's
    /// control net at `bend` percent, warp through the engine op with the X/Y
    /// distortion, recomposite, and record one `"Warp"` history state. Returns
    /// false without a document, for an unknown style or path, for `None`/
    /// `Custom` (no preset mesh), or when the engine refuses.
    pub fn apply_warp_preset(
        mut self: Pin<&mut Self>,
        path: &QString,
        style: &QString,
        bend: f64,
        distort_x: f64,
        distort_y: f64,
        rotate_vertical: bool,
    ) -> bool {
        let Some(style) = pictura_render::WarpStyle::from_id(&style.to_string()) else {
            return false;
        };
        let path = path.to_string();
        let applied = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            let Some(layer) = pictura_render::resolve_path(doc, &path) else {
                return false;
            };
            let rect = layer.rect;
            let Some(mesh) = pictura_render::style_mesh(
                style,
                bend,
                rotate_vertical,
                rect.width(),
                rect.height(),
            ) else {
                return false;
            };
            pictura_render::transform_layer_warp(
                doc,
                &path,
                &mesh,
                pictura_render::WarpParams {
                    distort_h: distort_x,
                    distort_v: distort_y,
                },
            )
        };
        if applied {
            self.as_mut().recomposite();
            self.as_mut().record("Warp");
        }
        applied
    }
}
