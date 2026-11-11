//! The pointer dab entry points: one sample of the active stroke, with or
//! without tablet pose.

use super::paint_timing;
use super::tool_recording;
use super::*;

impl qobject::PictureView {
    pub fn paint_dab(self: Pin<&mut Self>, x: f64, y: f64, pressure: f64) -> bool {
        self.paint_dab_pose(x, y, pressure, 0.0, 0.0, 0.0, 0.0)
    }

    pub fn paint_dab_pose(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        rotation: f64,
        wheel: f64,
    ) -> bool {
        if self.rust().quick_mask_stroke_active() {
            return self.quick_mask_dab(x, y, pressure, false);
        }
        let _dab = paint_timing::Scope::new("dab_total");
        tool_recording::sample(x, y, pressure);
        if self.rust().session.live.mask_stroke().is_some() {
            return self.as_mut().mask_dab(x, y, pressure);
        }
        // Grow the layer before a backend presents the dab, so the GPU patch and
        // the reduced preview both land on the grown rect (#1208).
        let ready = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            match (rust.session.live.stroke_mut(), rust.session.doc.as_mut()) {
                (Some(stroke), Some(doc)) => stroke.ensure_rect_for_sample(doc, x as f32, y as f32),
                _ => true,
            }
        };
        if !ready {
            return false;
        }
        if self.rust().session.live.gpu_stroke().is_some() {
            return self.as_mut().gpu_dab(x, y, pressure);
        }
        if self.rust().session.live.preview().is_some() {
            return self.as_mut().preview_dab(x, y, pressure);
        }
        let painted = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let t = Instant::now();
            let painted = match (rust.session.live.stroke_mut(), rust.session.doc.as_mut()) {
                (Some(stroke), Some(doc)) => stroke.sample_with(
                    doc,
                    StrokeSample {
                        x: x as f32,
                        y: y as f32,
                        pressure: pressure as f32,
                    },
                    pictura_paint::dynamics::DynamicsInput {
                        pressure: pressure as f32,
                        tilt_x: tilt_x as f32,
                        tilt_y: tilt_y as f32,
                        rotation: rotation as f32,
                        wheel: wheel as f32,
                    },
                ),
                _ => return false,
            };
            paint_timing::record("cpu_stroke_sample", t.elapsed());
            painted
        };
        painted && self.present_stroke_dirty()
    }
}
