use super::*;

impl qobject::PictureView {
    pub fn has_selection(&self) -> bool {
        self.rust().session.selection.is_some()
    }

    pub fn selection_count(&self) -> i32 {
        self.rust().selection_pixel_count()
    }

    /// Coverage byte at `(x, y)` (0 outside, 255 fully inside, intermediate on
    /// a feathered edge). 0 without a document/selection or out of bounds.
    pub fn selection_coverage(&self, x: i32, y: i32) -> i32 {
        i32::from(self.rust().selection_coverage_at(x, y))
    }
}
