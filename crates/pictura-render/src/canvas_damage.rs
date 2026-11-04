//! Outstanding display damage for the canvas: which document rectangle a partial
//! repaint must redraw.
//!
//! Any change nobody described is a change to the whole canvas. An undescribed
//! edit forces a full redraw, a described mark only ever adds to the damage, and
//! a mark can never mask an earlier undescribed edit; only `take` clears the
//! account. Forgetting to describe a change therefore costs speed, never a stale
//! picture.

use pictura_core::PsdRect;

/// The canvas's outstanding damage. See the module notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CanvasDamage {
    /// Union of every described mark since the last `take`; `None` = none.
    region: Option<PsdRect>,
    /// An edit nobody described: the whole canvas.
    everywhere: bool,
}

impl CanvasDamage {
    /// The document was changed in a way nobody described: the whole canvas must
    /// be redrawn.
    pub fn edited(&mut self) {
        self.everywhere = true;
    }

    /// The document changed within `rect`, in document pixels, and nowhere else.
    /// An empty rectangle shows nowhere and is ignored.
    pub fn mark(&mut self, rect: PsdRect) {
        if rect.is_empty() {
            return;
        }
        self.region = Some(match self.region {
            None => rect,
            Some(cur) => cur.union(rect),
        });
    }

    /// What the canvas must redraw to catch up, clipped to `canvas`, then start
    /// afresh. The whole canvas if anything went undescribed; an empty rectangle
    /// if nothing changed at all.
    pub fn take(&mut self, canvas: PsdRect) -> PsdRect {
        let damage = if self.everywhere {
            canvas
        } else {
            match self.region {
                None => PsdRect::default(),
                Some(region) => region.intersect(canvas).unwrap_or_default(),
            }
        };
        *self = Self::default();
        damage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANVAS: PsdRect = PsdRect {
        top: 0,
        left: 0,
        bottom: 80,
        right: 100,
    };

    fn rect(top: i32, left: i32, width: i32, height: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom: top + height,
            right: left + width,
        }
    }

    #[test]
    fn nothing_changed_means_nothing_to_redraw() {
        assert_eq!(CanvasDamage::default().take(CANVAS), PsdRect::default());
    }

    #[test]
    fn described_changes_add_up() {
        let mut damage = CanvasDamage::default();
        damage.mark(rect(10, 10, 5, 5));
        damage.mark(rect(20, 30, 5, 5));
        assert_eq!(damage.take(CANVAS), rect(10, 10, 25, 15));
    }

    #[test]
    fn an_undescribed_change_is_the_whole_canvas() {
        let mut damage = CanvasDamage::default();
        damage.mark(rect(10, 10, 5, 5));
        damage.edited();
        assert_eq!(damage.take(CANVAS), CANVAS);
    }

    #[test]
    fn a_mark_cannot_hide_an_earlier_undescribed_change() {
        let mut damage = CanvasDamage::default();
        damage.edited();
        damage.mark(rect(10, 10, 5, 5));
        assert_eq!(damage.take(CANVAS), CANVAS);
    }

    #[test]
    fn taking_starts_afresh() {
        let mut damage = CanvasDamage::default();
        damage.edited();
        damage.take(CANVAS);
        assert_eq!(damage.take(CANVAS), PsdRect::default());
    }

    #[test]
    fn damage_is_clipped_to_the_canvas() {
        let mut damage = CanvasDamage::default();
        damage.mark(rect(70, 90, 40, 40));
        assert_eq!(damage.take(CANVAS), rect(70, 90, 10, 10));
    }

    #[test]
    fn an_empty_mark_is_ignored() {
        let mut damage = CanvasDamage::default();
        damage.mark(rect(5, 5, 0, 0));
        assert_eq!(damage.take(CANVAS), PsdRect::default());
    }
}
