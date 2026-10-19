//! Layer > Align / Align Layers To Selection / Distribute, and the Move tool's
//! matching options-bar buttons. Each layer lines up by its content — the
//! box round its non-transparent pixels — not by its layer bounds. Ported
//! from photorust's `core/src/document.rs` (`align_layers` /
//! `distribute_layers`).
//!
//! ponytail: groups, adjustment layers, and fill / shape layers (whose
//! content is a fill under a vector mask) are skipped rather than aligned by
//! their descendants' content or their shape bounds.

use pictura_core::{layer_move_locked, Document, Layer, PsdRect};

use super::paths::{resolve_path, resolve_path_mut, unique};
use crate::document_ops::crop::offset_layer;

/// One of the six edges the Align and Distribute commands line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignEdge {
    Top,
    VerticalCenter,
    Bottom,
    Left,
    HorizontalCenter,
    Right,
}

impl AlignEdge {
    /// The menu order: 0 Top … 5 Right; `None` outside it.
    pub fn from_index(index: i32) -> Option<AlignEdge> {
        Some(match index {
            0 => AlignEdge::Top,
            1 => AlignEdge::VerticalCenter,
            2 => AlignEdge::Bottom,
            3 => AlignEdge::Left,
            4 => AlignEdge::HorizontalCenter,
            5 => AlignEdge::Right,
            _ => return None,
        })
    }

    /// The edges that run up and down, which move layers sideways.
    fn is_horizontal(self) -> bool {
        matches!(
            self,
            AlignEdge::Left | AlignEdge::HorizontalCenter | AlignEdge::Right
        )
    }

    /// Where this edge of `r` lies, doubled, so a centre on a half pixel is
    /// still a whole number.
    fn doubled(self, r: PsdRect) -> i32 {
        match self {
            AlignEdge::Top => 2 * r.top,
            AlignEdge::VerticalCenter => r.top + r.bottom,
            AlignEdge::Bottom => 2 * r.bottom,
            AlignEdge::Left => 2 * r.left,
            AlignEdge::HorizontalCenter => r.left + r.right,
            AlignEdge::Right => 2 * r.right,
        }
    }

    /// The History state an align along this edge records.
    pub fn align_name(self) -> &'static str {
        match self {
            AlignEdge::Top => "Align Top Edges",
            AlignEdge::VerticalCenter => "Align Vertical Centers",
            AlignEdge::Bottom => "Align Bottom Edges",
            AlignEdge::Left => "Align Left Edges",
            AlignEdge::HorizontalCenter => "Align Horizontal Centers",
            AlignEdge::Right => "Align Right Edges",
        }
    }

    /// The History state a distribute along this edge records.
    pub fn distribute_name(self) -> &'static str {
        match self {
            AlignEdge::Top => "Distribute Top Edges",
            AlignEdge::VerticalCenter => "Distribute Vertical Centers",
            AlignEdge::Bottom => "Distribute Bottom Edges",
            AlignEdge::Left => "Distribute Left Edges",
            AlignEdge::HorizontalCenter => "Distribute Horizontal Centers",
            AlignEdge::Right => "Distribute Right Edges",
        }
    }

    /// The `(dx, dy)` that moves this edge `by` pixels.
    fn delta(self, by: i32) -> (i32, i32) {
        if self.is_horizontal() {
            (by, 0)
        } else {
            (0, by)
        }
    }
}

/// The document-space box round `layer`'s non-transparent pixels; the whole
/// layer rect when it has no transparency channel. `None` for a group, an
/// adjustment, or a layer with nothing showing.
fn content_bounds(layer: &Layer) -> Option<PsdRect> {
    if layer.is_group || layer.adjustment.is_some() {
        return None;
    }
    let r = layer.rect;
    let (w, h) = (r.width().max(0), r.height().max(0));
    if w == 0 || h == 0 {
        return None;
    }
    let Some(alpha) = layer.channels.iter().find(|c| c.id == -1) else {
        return Some(r);
    };
    let local = super::coverage_bounds(&alpha.data, w as u32, h as u32)?;
    Some(PsdRect {
        top: r.top + local.top,
        left: r.left + local.left,
        bottom: r.top + local.bottom,
        right: r.left + local.right,
    })
}

/// The document-space ink box round the layer at `path`, as Align computes it;
/// `None` when the path is missing, a group, an adjustment, or empty.
pub fn layer_content_bounds(doc: &Document, path: &str) -> Option<PsdRect> {
    content_bounds(resolve_path(doc, path)?)
}

/// Each of `paths` (deduplicated) that has content, with its content box.
fn contents(doc: &Document, paths: &[&str]) -> Vec<(String, PsdRect)> {
    unique(paths)
        .into_iter()
        .filter_map(|p| Some((p.to_string(), content_bounds(resolve_path(doc, p)?)?)))
        .collect()
}

fn union(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.min(b.top),
        left: a.left.min(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.max(b.right),
    }
}

/// Move `path` by `(dx, dy)` unless it is position-locked or the Background.
fn shift(doc: &mut Document, path: &str, (dx, dy): (i32, i32)) -> bool {
    match resolve_path_mut(doc, path) {
        Some(layer) if !layer_move_locked(layer) && !layer.background => {
            offset_layer(layer, dx, dy);
            true
        }
        _ => false,
    }
}

/// Whether Align applies: two layers with content line up on each other; with
/// a `selection` box, one is enough (CS6's Align Layers To Selection, which
/// is also how one layer aligns to the canvas after Select All).
pub fn can_align(doc: &Document, paths: &[&str], selection: Option<PsdRect>) -> bool {
    let n = contents(doc, paths).len();
    n >= 2 || (n == 1 && selection.is_some())
}

/// Line the content of `paths` up along `edge`: against `selection` when
/// given, else against the box round all of them. A position-locked layer or
/// the Background holds still but still counts toward that box. Returns how
/// many layers moved.
pub fn align_layers(
    doc: &mut Document,
    paths: &[&str],
    edge: AlignEdge,
    selection: Option<PsdRect>,
) -> usize {
    if !can_align(doc, paths, selection) {
        return 0;
    }
    let rects = contents(doc, paths);
    let target = selection.unwrap_or_else(|| {
        rects[1..]
            .iter()
            .fold(rects[0].1, |all, (_, r)| union(all, *r))
    });
    let mut moved = 0;
    for (path, rect) in &rects {
        let by = (edge.doubled(target) - edge.doubled(*rect)).div_euclid(2);
        if by != 0 && shift(doc, path, edge.delta(by)) {
            moved += 1;
        }
    }
    moved
}

/// Whether Distribute applies: three layers with content (with two there is
/// nothing between them to space).
pub fn can_distribute(doc: &Document, paths: &[&str]) -> bool {
    contents(doc, paths).len() >= 3
}

/// Space the content of `paths` so `edge` of each falls at even steps between
/// the two outermost, which stay put. Returns how many layers moved.
pub fn distribute_layers(doc: &mut Document, paths: &[&str], edge: AlignEdge) -> usize {
    let mut rects = contents(doc, paths);
    if rects.len() < 3 {
        return 0;
    }
    rects.sort_by_key(|(_, r)| edge.doubled(*r));
    let first = edge.doubled(rects[0].1) as i64;
    let last = edge.doubled(rects[rects.len() - 1].1) as i64;
    let steps = (rects.len() - 1) as i64;
    let mut moved = 0;
    for (i, (path, rect)) in rects.iter().enumerate() {
        // In doubled units, so a centre needs no rounding until the end.
        let want = first + (last - first) * i as i64 / steps;
        let by = ((want - edge.doubled(*rect) as i64) as f64 / 2.0).round() as i32;
        if by != 0 && shift(doc, path, edge.delta(by)) {
            moved += 1;
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::super::tests::{doc_with, pixel_layer};
    use super::*;
    use pictura_core::{Channel, LockFlags};

    fn at(rect: &mut PsdRect, x: i32, y: i32) {
        let (w, h) = (rect.width(), rect.height());
        *rect = PsdRect {
            top: y,
            left: x,
            bottom: y + h,
            right: x + w,
        };
    }

    /// A document of opaque squares, each `(x, y, size)`, bottom first; their
    /// paths are "0", "1", ….
    fn squares(spec: &[(i32, i32, u32)]) -> Document {
        let layers = spec
            .iter()
            .map(|&(x, y, size)| {
                let mut layer = pixel_layer("Square", size, size, 0);
                at(&mut layer.rect, x, y);
                layer
            })
            .collect();
        doc_with(layers)
    }

    fn origin(doc: &Document, i: usize) -> (i32, i32) {
        (doc.layers[i].rect.left, doc.layers[i].rect.top)
    }

    const ALL3: [&str; 3] = ["0", "1", "2"];

    #[test]
    fn align_lines_edges_up_on_the_outermost() {
        let mut d = squares(&[(10, 40, 20), (60, 10, 30), (120, 70, 10)]);
        assert_eq!(align_layers(&mut d, &ALL3, AlignEdge::Top, None), 2);
        for i in 0..3 {
            assert_eq!(origin(&d, i).1, 10, "every top meets the highest");
        }
        align_layers(&mut d, &ALL3, AlignEdge::Right, None);
        for i in 0..3 {
            assert_eq!(
                d.layers[i].rect.right, 130,
                "every right edge meets the furthest"
            );
        }
    }

    #[test]
    fn align_centres_on_the_middle_of_them_all() {
        let mut d = squares(&[(0, 0, 20), (80, 100, 40)]);
        align_layers(&mut d, &["0", "1"], AlignEdge::HorizontalCenter, None);
        // The box round both runs 0..120, so its middle is 60.
        assert_eq!(origin(&d, 0), (50, 0));
        assert_eq!(origin(&d, 1), (40, 100), "sideways only");
    }

    #[test]
    fn align_against_a_selection_moves_even_one_layer() {
        let mut d = squares(&[(30, 30, 20)]);
        let selection = PsdRect {
            top: 100,
            left: 0,
            bottom: 200,
            right: 200,
        };
        assert!(!can_align(&d, &["0"], None));
        assert_eq!(
            align_layers(&mut d, &["0"], AlignEdge::Bottom, Some(selection)),
            1
        );
        assert_eq!(origin(&d, 0), (30, 180));
    }

    #[test]
    fn align_goes_by_the_content_not_the_layer() {
        // A 10x10 square 40 px into a 100x100 transparent layer.
        let mut big = pixel_layer("Big", 100, 100, 0);
        let alpha: Vec<u8> = (0..100 * 100)
            .map(|i| u8::from((40..50).contains(&(i % 100)) && (40..50).contains(&(i / 100))) * 255)
            .collect();
        big.channels.retain(|c| c.id != -1);
        big.channels.push(Channel {
            id: -1,
            data: alpha.into(),
        });
        let mut small = pixel_layer("Small", 10, 10, 0);
        at(&mut small.rect, 5, 150);
        let mut d = doc_with(vec![big, small]);
        align_layers(&mut d, &["0", "1"], AlignEdge::Left, None);
        assert_eq!(origin(&d, 0).0, -35, "the square's edge, 40 in, meets 5");
        assert_eq!(origin(&d, 1).0, 5);
    }

    #[test]
    fn align_leaves_a_position_locked_layer_where_it_is() {
        let mut d = squares(&[(10, 40, 20), (60, 10, 30)]);
        d.layers[1].lock = LockFlags::default().with(LockFlags::POSITION, true);
        align_layers(&mut d, &["0", "1"], AlignEdge::Top, None);
        assert_eq!(origin(&d, 0).1, 10, "the other still comes up to it");
        align_layers(&mut d, &["0", "1"], AlignEdge::Bottom, None);
        assert_eq!(origin(&d, 1), (60, 10));
    }

    #[test]
    fn align_skips_groups_and_dedupes_paths() {
        let mut d = squares(&[(10, 40, 20)]);
        d.layers.push(super::super::create::empty_group("Group"));
        assert!(!can_align(&d, &["0", "0", "1"], None));
    }

    #[test]
    fn distribute_spaces_the_middle_evenly_and_keeps_the_ends() {
        let mut d = squares(&[(0, 0, 10), (15, 50, 10), (100, 90, 10), (60, 20, 10)]);
        let all = ["0", "1", "2", "3"];
        assert_eq!(distribute_layers(&mut d, &all, AlignEdge::Left), 2);
        // Left edges from 0 to 100 in three even steps.
        assert_eq!(origin(&d, 0).0, 0);
        assert_eq!(origin(&d, 1).0, 33);
        assert_eq!(origin(&d, 3).0, 67);
        assert_eq!(origin(&d, 2).0, 100);
    }

    #[test]
    fn distribute_needs_three_layers() {
        let mut d = squares(&[(0, 0, 10), (50, 50, 10)]);
        assert!(!can_distribute(&d, &["0", "1"]));
        assert_eq!(distribute_layers(&mut d, &["0", "1"], AlignEdge::Top), 0);
    }

    #[test]
    fn edge_indices_follow_the_menu() {
        assert_eq!(AlignEdge::from_index(0), Some(AlignEdge::Top));
        assert_eq!(AlignEdge::from_index(5), Some(AlignEdge::Right));
        assert_eq!(AlignEdge::from_index(6), None);
        assert_eq!(
            AlignEdge::HorizontalCenter.distribute_name(),
            "Distribute Horizontal Centers"
        );
    }
}
