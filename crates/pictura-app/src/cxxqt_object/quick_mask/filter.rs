use super::*;
use crate::cxxqt_object::{
    document_session::{ActiveSession, FilterPreviewSession},
    helpers_composite::selection_to_mask,
    impl_filters::ActiveOp,
};

impl PictureViewRust {
    pub(in crate::cxxqt_object) fn apply_quick_mask_op(
        &mut self,
        op: &ActiveOp,
        commit: bool,
    ) -> Option<Option<PsdRect>> {
        if self.layer_target.is_empty() || !self.slot_allows(SessionKind::FilterPreview) {
            return None;
        }
        let baseline = match &self.session.live {
            ActiveSession::Idle => self.session.quick_mask.coverage.clone(),
            ActiveSession::FilterPreview(preview) => match &**preview {
                FilterPreviewSession::QuickMask { original } => original.clone(),
                _ => return None,
            },
            _ => return None,
        };
        let current = self.session.quick_mask.coverage.clone();
        self.session.quick_mask.coverage = baseline.clone();
        let document = self.quick_mask_document(true);
        self.session.quick_mask.coverage = current;
        let mut document = document?;
        let clip = self
            .session
            .selection
            .as_ref()
            .map(|s| selection_to_mask(s, &document));
        let layer = document.layers.first_mut()?;
        let rect = layer.rect;
        let result = match op {
            ActiveOp::Filter(filter) => {
                pictura_render::apply_filter(layer, filter, clip.as_ref(), false)
            }
            ActiveOp::Filters(filters) => filters
                .iter()
                .try_for_each(|f| pictura_render::apply_filter(layer, f, clip.as_ref(), false)),
            ActiveOp::Adjustment(adjustment) => {
                pictura_render::apply_adjustment_region(layer, adjustment, clip.as_ref(), rect)
            }
            ActiveOp::ApplyImage(_) => return None,
        };
        if let Err(error) = result {
            self.filter_error = Some(error.to_string());
            return None;
        }
        self.session.quick_mask.coverage = baseline.clone();
        let changed = self.adopt_quick_mask_document(document, true);
        if commit {
            self.session.live.take_filter_preview_session();
            if !changed {
                return None;
            }
        } else {
            self.session.live =
                ActiveSession::FilterPreview(Box::new(FilterPreviewSession::QuickMask {
                    original: baseline,
                }));
        }
        self.invalidate_quick_mask();
        Some(Some(rect))
    }
}
