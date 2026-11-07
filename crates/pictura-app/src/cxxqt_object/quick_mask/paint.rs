use super::*;
impl PictureView {
    pub(in crate::cxxqt_object) fn quick_mask_dab(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        pressure: f64,
        hold: bool,
    ) -> bool {
        let changed = self
            .as_mut()
            .rust_mut()
            .sample_quick_mask(x, y, pressure, hold);
        if changed {
            let present = {
                let mut rust = self.as_mut().rust_mut();
                let dirty = rust
                    .session
                    .live
                    .stroke_session_mut()
                    .and_then(|s| s.quick_mask.as_mut())
                    .and_then(|s| s.stroke.take_dirty());
                dirty.and_then(|r| rust.queue_present(r))
            };
            if let Some(rect) = present {
                self.as_mut().quick_mask_present(rect);
            }
        }
        changed
    }
    pub(in crate::cxxqt_object) fn quick_mask_present(mut self: Pin<&mut Self>, rect: PsdRect) {
        self.as_mut()
            .region_damaged(rect.left, rect.top, rect.width(), rect.height());
    }
    pub(in crate::cxxqt_object) fn end_quick_mask_paint(mut self: Pin<&mut Self>) -> bool {
        let label = self
            .rust()
            .session
            .live
            .stroke_session()
            .unwrap()
            .stroke_label
            .clone();
        let changed = self.as_mut().rust_mut().finish_quick_mask_paint();
        if changed {
            self.as_mut().edit(&label, |_| Edit::Meta);
        } else {
            self.as_mut().changed();
        }
        changed
    }
}
impl PictureViewRust {
    pub(in crate::cxxqt_object) fn cancel_quick_mask_stroke(
        &mut self,
        ctx: QuickMaskStroke,
    ) -> Option<PsdRect> {
        let rect = ctx.stroke.dirty();
        self.session.quick_mask.coverage = ctx.baseline;
        if let Some(values) = self.session.quick_mask.pending.take() {
            self.apply_quick_mask_options(values);
        }
        self.invalidate_quick_mask();
        rect
    }
}
