//! Non-destructive scale/translation commits for placed smart objects.
//!
//! A placed smart object renders its embedded source scaled into the layer
//! rect, so an axis-aligned positive scale and a translation can be committed
//! by changing the rect alone. Rotation, skew, warp, masks, linked sets, and
//! every other target keep the rasterizing path.

use super::super::document_session::RepeatTransform;
use super::super::edit::Edit;
use super::super::qobject;
use super::geometry::MapParams;
use super::group::transform_group_edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;

/// The same 1 Gpx refusal the raster transform applies to a resampled result
/// (`pictura-render`'s `MAX_RESULT_PIXELS`), kept here because the smart-rect
/// path allocates no plane but must not create a rect no later op can handle.
const MAX_RESULT_PIXELS: u64 = 1 << 30;

/// Integer bounding box of the session quad, or `None` for a non-finite,
/// outward-of-i32, or implausibly large result.
fn quad_bbox(quad: [(f64, f64); 4]) -> Option<PsdRect> {
    if quad.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return None;
    }
    let (mut x0, mut y0) = (f64::MAX, f64::MAX);
    let (mut x1, mut y1) = (f64::MIN, f64::MIN);
    for &(x, y) in &quad {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (left, top, right, bottom) = (
        x0.floor() as i32,
        y0.floor() as i32,
        x1.ceil() as i32,
        y1.ceil() as i32,
    );
    if x0 < i32::MIN as f64 || x1 > i32::MAX as f64 || y0 < i32::MIN as f64 || y1 > i32::MAX as f64
    {
        return None;
    }
    let width = (right as i64) - (left as i64);
    let height = (bottom as i64) - (top as i64);
    if width <= 0 || height <= 0 || (width as u64) * (height as u64) > MAX_RESULT_PIXELS {
        return None;
    }
    Some(PsdRect {
        left,
        top,
        right,
        bottom,
    })
}

/// The destination rect for a scale/translation on a placed smart object, or
/// `None` when the session must use the rasterizing path. The layer rect is the
/// renderer's transform, so an axis-aligned positive scale and translation can
/// be committed without resampling the embedded source.
#[allow(clippy::too_many_arguments)]
fn smart_object_commit_rect(
    doc: Option<&pictura_core::Document>,
    path: &str,
    members: &[(String, PsdRect)],
    params: MapParams,
    warp: Option<&pictura_core::warp::Warp>,
    projective: bool,
    quad: [(f64, f64); 4],
) -> Option<PsdRect> {
    let doc = doc?;
    if members.len() != 1
        || members[0].0.as_str() != path
        || warp.is_some()
        || projective
        || params.angle.abs() > 1e-9
        || params.skew_x != 0.0
        || params.skew_y != 0.0
        || params.scale_x <= 0.0
        || params.scale_y <= 0.0
        || !pictura_render::can_transform_smart_object_rect(doc, path)
    {
        return None;
    }
    let rect = quad_bbox(quad)?;
    (rect.width() > 0 && rect.height() > 0).then_some(rect)
}

impl qobject::PictureView {
    /// Commit a scale/translation session on a placed smart object by changing
    /// its rect. `None` when this is not the non-destructive case, so the caller
    /// runs the rasterizing commit; otherwise the session is cleared and the
    /// one `"Free Transform"` state is recorded when the rect changed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_smart_object_rect(
        mut self: Pin<&mut Self>,
        path: &str,
        members: &[(String, PsdRect)],
        params: MapParams,
        warp: Option<&pictura_core::warp::Warp>,
        projective: bool,
        quad: [(f64, f64); 4],
        history_label: &str,
        orig_rect: PsdRect,
        repeat: Option<RepeatTransform>,
    ) -> Option<bool> {
        let rect = smart_object_commit_rect(
            self.rust().session.doc.as_ref(),
            path,
            members,
            params,
            warp,
            projective,
            quad,
        )?;
        let _ = self.as_mut().rust_mut().session.live.take_transform();
        let changed = self
            .as_mut()
            .rust_mut()
            .session
            .doc
            .as_mut()
            .is_some_and(|doc| pictura_render::transform_smart_object_rect(doc, path, rect));
        if changed {
            self.as_mut().rust_mut().last_transform = repeat;
            let edit = self.rust().session.doc.as_ref().map_or(Edit::Full, |doc| {
                transform_group_edit(doc, members, orig_rect)
            });
            self.as_mut()
                .commit_transform_edit(history_label, true, |_| edit);
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
        }
        self.as_mut().clear_transform_preview();
        Some(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_bbox_refuses_oversized_and_out_of_range_quads() {
        let normal = quad_bbox([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])
            .expect("an ordinary quad bboxes");
        assert_eq!(
            (normal.left, normal.top, normal.right, normal.bottom),
            (0, 0, 10, 10)
        );

        let oversized = (0.0, 0.0);
        let big = 40_000.0;
        assert!(
            quad_bbox([oversized, (big, 0.0), (big, big), (0.0, big)]).is_none(),
            "a quad over the 1 Gpx ceiling is refused"
        );
        assert!(
            quad_bbox([
                (0.0, 0.0),
                (f64::from(i32::MAX) + 1.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
            ])
            .is_none(),
            "an out-of-i32 quad is refused"
        );
        assert!(
            quad_bbox([(0.0, 0.0), (f64::NAN, 0.0), (10.0, 10.0), (0.0, 10.0)]).is_none(),
            "a non-finite quad is refused"
        );
    }
}
