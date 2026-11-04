//! Slices: the document's web-export cut lines. User slices are drawn with the
//! Slice tool and stored in [`Document::slices`]; auto slices are derived to
//! cover everything the user slices do not, so they are recomputed on every
//! query rather than stored. Slices carry no pixels; export crops the composite.
//!
//! Ported from photorust's `core/src/slice.rs`
//! (<https://github.com/perfecto25/photorust>).

use pictura_core::{Document, PsdRect};

/// One resolved slice, ready to draw or export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slice {
    pub rect: PsdRect,
    /// CS6's badge number: 1-based, in reading order over user and auto slices.
    pub number: u32,
    /// Index into [`Document::slices`], or `None` for an auto slice.
    pub user_index: Option<usize>,
}

/// Add a user slice, returning its index; `None` (and no change) for an empty
/// rect, so a click without a drag leaves no invisible slice behind.
pub fn add_slice(doc: &mut Document, rect: PsdRect) -> Option<usize> {
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }
    doc.slices.push(rect);
    Some(doc.slices.len() - 1)
}

/// Move or resize user slice `index`; false (no change) for an empty rect or an
/// out-of-range index.
pub fn set_slice(doc: &mut Document, index: usize, rect: PsdRect) -> bool {
    if rect.width() <= 0 || rect.height() <= 0 || index >= doc.slices.len() {
        return false;
    }
    doc.slices[index] = rect;
    true
}

/// Delete user slice `index`; false for an out-of-range index.
pub fn remove_slice(doc: &mut Document, index: usize) -> bool {
    if index >= doc.slices.len() {
        return false;
    }
    doc.slices.remove(index);
    true
}

/// Every slice over the canvas: the user slices clipped to it, plus auto slices
/// tiling the rest, numbered left to right then top to bottom. An unsliced
/// document is one auto slice. A user slice entirely off-canvas is skipped but
/// keeps its index.
pub fn resolve_slices(doc: &Document) -> Vec<Slice> {
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    if canvas.width() <= 0 || canvas.height() <= 0 {
        return Vec::new();
    }
    let mut slices = Vec::new();
    let mut kept = Vec::new();
    for (index, rect) in doc.slices.iter().enumerate() {
        let Some(clipped) = rect.intersect(canvas) else {
            continue;
        };
        kept.push(clipped);
        slices.push(Slice {
            rect: clipped,
            number: 0,
            user_index: Some(index),
        });
    }
    slices.extend(auto_slices(&kept, canvas).into_iter().map(|rect| Slice {
        rect,
        number: 0,
        user_index: None,
    }));
    slices.sort_by_key(|s| (s.rect.top, s.rect.left));
    for (i, slice) in slices.iter_mut().enumerate() {
        slice.number = i as u32 + 1;
    }
    slices
}

/// The rects covering what `user` leaves of `canvas`: every user edge is
/// extended across the canvas into a grid, uncovered cells are the auto slices,
/// and runs of them along a row merge into one wide slice, as CS6 draws them.
fn auto_slices(user: &[PsdRect], canvas: PsdRect) -> Vec<PsdRect> {
    let mut xs = vec![canvas.left, canvas.right];
    let mut ys = vec![canvas.top, canvas.bottom];
    for rect in user {
        xs.extend([rect.left, rect.right]);
        ys.extend([rect.top, rect.bottom]);
    }
    for v in [&mut xs, &mut ys] {
        v.sort_unstable();
        v.dedup();
    }
    let covered = |x: i32, y: i32| {
        user.iter()
            .any(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom)
    };
    let mut out = Vec::new();
    for row in ys.windows(2) {
        let (top, bottom) = (row[0], row[1]);
        let mut run: Option<i32> = None;
        for col in xs.windows(2) {
            let left = col[0];
            if covered(left, top) {
                if let Some(start) = run.take() {
                    out.push(PsdRect {
                        top,
                        left: start,
                        bottom,
                        right: left,
                    });
                }
            } else if run.is_none() {
                run = Some(left);
            }
        }
        if let Some(start) = run {
            out.push(PsdRect {
                top,
                left: start,
                bottom,
                right: canvas.right,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode};

    fn doc() -> Document {
        Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight)
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> PsdRect {
        PsdRect {
            top: y,
            left: x,
            bottom: y + h,
            right: x + w,
        }
    }

    fn area(slices: &[Slice]) -> i64 {
        slices
            .iter()
            .map(|s| s.rect.width() as i64 * s.rect.height() as i64)
            .sum()
    }

    fn overlaps(a: PsdRect, b: PsdRect) -> bool {
        a.intersect(b).is_some()
    }

    #[test]
    fn an_unsliced_document_is_one_auto_slice() {
        let slices = resolve_slices(&doc());
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].user_index, None);
        assert_eq!(
            (slices[0].rect, slices[0].number),
            (rect(0, 0, 100, 100), 1)
        );
    }

    #[test]
    fn slices_tile_the_canvas_without_overlap() {
        let mut d = doc();
        add_slice(&mut d, rect(20, 20, 30, 30));
        add_slice(&mut d, rect(60, 10, 30, 20));
        let slices = resolve_slices(&d);

        assert_eq!(area(&slices), 100 * 100);
        for (i, a) in slices.iter().enumerate() {
            for b in &slices[i + 1..] {
                assert!(
                    !overlaps(a.rect, b.rect),
                    "{:?} overlaps {:?}",
                    a.rect,
                    b.rect
                );
            }
        }
    }

    #[test]
    fn a_user_slice_survives_as_itself_and_keeps_its_index() {
        let mut d = doc();
        assert_eq!(add_slice(&mut d, rect(20, 20, 30, 30)), Some(0));
        let user: Vec<_> = resolve_slices(&d)
            .into_iter()
            .filter(|s| s.user_index.is_some())
            .collect();
        assert_eq!(user.len(), 1);
        assert_eq!(
            (user[0].rect, user[0].user_index),
            (rect(20, 20, 30, 30), Some(0))
        );
    }

    #[test]
    fn numbering_runs_in_reading_order_without_gaps() {
        let mut d = doc();
        add_slice(&mut d, rect(0, 50, 50, 50));
        let slices = resolve_slices(&d);

        let numbers: Vec<u32> = slices.iter().map(|s| s.number).collect();
        assert_eq!(numbers, (1..=slices.len() as u32).collect::<Vec<_>>());
        for pair in slices.windows(2) {
            assert!((pair[0].rect.top, pair[0].rect.left) <= (pair[1].rect.top, pair[1].rect.left));
        }
    }

    #[test]
    fn an_empty_band_merges_into_one_wide_slice() {
        let mut d = doc();
        add_slice(&mut d, rect(40, 0, 20, 20));
        let slices = resolve_slices(&d);
        let band = slices
            .iter()
            .find(|s| s.rect.top == 20 && s.rect.height() == 80)
            .expect("no band beneath the slice");
        assert_eq!(band.rect.width(), 100);
    }

    #[test]
    fn slices_are_clipped_and_off_canvas_ones_skipped() {
        let mut d = doc();
        add_slice(&mut d, rect(80, 80, 60, 60));
        add_slice(&mut d, rect(200, 200, 20, 20));
        let slices = resolve_slices(&d);

        assert_eq!(area(&slices), 100 * 100);
        let user: Vec<_> = slices.iter().filter(|s| s.user_index.is_some()).collect();
        assert_eq!(user.len(), 1);
        assert_eq!(user[0].rect, rect(80, 80, 20, 20));
        assert_eq!(d.slices.len(), 2, "the off-canvas slice is still stored");
    }

    #[test]
    fn a_full_canvas_slice_leaves_no_auto_slices() {
        let mut d = doc();
        add_slice(&mut d, rect(0, 0, 100, 100));
        let slices = resolve_slices(&d);
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].user_index, Some(0));
    }

    #[test]
    fn empty_slices_are_refused() {
        let mut d = doc();
        assert_eq!(add_slice(&mut d, rect(10, 10, 0, 0)), None);
        assert_eq!(add_slice(&mut d, rect(10, 10, 5, 0)), None);
        assert!(d.slices.is_empty());
    }

    #[test]
    fn overlapping_user_slices_leave_no_auto_slice_underneath() {
        let mut d = doc();
        add_slice(&mut d, rect(10, 10, 40, 40));
        add_slice(&mut d, rect(30, 30, 40, 40));
        for auto in resolve_slices(&d).iter().filter(|s| s.user_index.is_none()) {
            for user in &d.slices {
                assert!(
                    !overlaps(auto.rect, *user),
                    "{:?} under {:?}",
                    auto.rect,
                    user
                );
            }
        }
    }

    #[test]
    fn set_and_remove_address_the_right_slice() {
        let mut d = doc();
        add_slice(&mut d, rect(0, 0, 10, 10));
        add_slice(&mut d, rect(20, 20, 10, 10));

        assert!(set_slice(&mut d, 1, rect(50, 50, 10, 10)));
        assert_eq!(d.slices[1], rect(50, 50, 10, 10));
        assert!(
            !set_slice(&mut d, 9, rect(0, 0, 5, 5)),
            "out-of-range index"
        );
        assert!(!set_slice(&mut d, 0, rect(0, 0, 5, 0)), "empty rect");
        assert_eq!(d.slices[0], rect(0, 0, 10, 10));

        assert!(remove_slice(&mut d, 0));
        assert_eq!(d.slices, vec![rect(50, 50, 10, 10)]);
        assert!(!remove_slice(&mut d, 9));
    }

    #[test]
    fn an_empty_canvas_has_no_slices() {
        assert!(resolve_slices(&Document::new(0, 0, ColorMode::Rgb, BitDepth::Eight)).is_empty());
    }
}
