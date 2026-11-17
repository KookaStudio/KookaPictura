//! Whole-selection exports to the shell: each fills one row-major output
//! buffer from the selection's tiles, leaving no cached contiguous copy of the
//! selection behind (#1268).

use super::super::picture::{stream_picture, whole};
use super::super::state::PictureViewRust;
use super::channel_row_selection_from;
use super::save_load::selection_from_target;

impl PictureViewRust {
    /// The live selection's coverage; empty without a selection.
    pub(in crate::cxxqt_object) fn selection_coverage_bytes(&self) -> Vec<u8> {
        self.session
            .selection
            .as_ref()
            .map(|selection| selection.coverage_bytes())
            .unwrap_or_default()
    }

    /// The coverage of `target` in this document; empty on failure.
    pub(in crate::cxxqt_object) fn channel_coverage(&self, target: &str) -> Vec<u8> {
        self.session
            .doc
            .as_ref()
            .and_then(|doc| selection_from_target(doc, target))
            .map(|selection| selection.coverage_bytes())
            .unwrap_or_default()
    }

    /// The coverage of colour row `row` (0 is the composite) under `mask`.
    pub(in crate::cxxqt_object) fn color_row_coverage(&self, row: i32, mask: u8) -> Vec<u8> {
        let Some(doc) = self.session.doc.as_ref() else {
            return Vec::new();
        };
        channel_row_selection_from(doc, |f| stream_picture(doc, self, whole(doc), f), row, mask)
            .map(|selection| selection.coverage_bytes())
            .unwrap_or_default()
    }
}
