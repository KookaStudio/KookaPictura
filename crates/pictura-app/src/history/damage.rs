//! The picture change a restore reports: changed tiles placed in the
//! document, and the layer-metadata changes no tile shows (#1216).

use super::planes::{self, plane_kinds, tile_rect, PlaneDelta, PlaneKind};
use pictura_core::{Document, PsdRect};

/// The picture change a walk through the states accumulates.
pub(super) struct Damage {
    pub(super) kinds: Vec<PlaneKind>,
    pub(super) height: usize,
    /// The document keeps no stored composite (#1216): its layer tiles are
    /// the picture's change, grown by their reach.
    pub(super) layer_tiles: bool,
    pub(super) rect: Option<PsdRect>,
    pub(super) everywhere: bool,
    /// The walk left a segment, so its tiles no longer tell the whole change.
    pub(super) crossed: bool,
}

impl Damage {
    pub(super) fn new(doc: &Document) -> Self {
        Self {
            kinds: plane_kinds(doc),
            height: doc.height as usize,
            layer_tiles: !pictura_render::DocumentComposite::is_stored(doc),
            rect: None,
            everywhere: false,
            crossed: false,
        }
    }

    /// The document rows a composite tile covers. The composite stacks its
    /// channels in one plane, so a tile row is a document row modulo the
    /// height, and a tile straddling two channels covers the whole height.
    fn rows(&self, tile: &planes::TileDelta) -> (i32, i32) {
        let h = self.height.max(1);
        let (first, last) = (tile.y / h, (tile.y + tile.h - 1) / h);
        if first == last {
            ((tile.y % h) as i32, ((tile.y + tile.h - 1) % h + 1) as i32)
        } else {
            (0, h as i32)
        }
    }

    pub(super) fn union(&mut self, r: PsdRect) {
        self.rect = Some(self.rect.map_or(r, |u| u.union(r)));
    }

    pub(super) fn add(&mut self, deltas: &[PlaneDelta]) {
        for delta in deltas {
            match self.kinds.get(delta.index).copied() {
                Some(PlaneKind::Composite) => {
                    for tile in &delta.tiles {
                        let (top, bottom) = self.rows(tile);
                        self.union(PsdRect {
                            top,
                            bottom,
                            ..tile_rect(tile)
                        });
                    }
                }
                Some(PlaneKind::Mask(mask, reach)) => {
                    let grow = if self.layer_tiles { reach } else { 0 };
                    for tile in &delta.tiles {
                        self.union(placed(tile, mask, grow));
                    }
                }
                Some(PlaneKind::LayerChannel { rect, reach }) => {
                    if self.layer_tiles {
                        for tile in &delta.tiles {
                            self.union(placed(tile, rect, reach));
                        }
                    }
                }
                // No canvas view draws these planes directly.
                Some(PlaneKind::DocumentChannel) => {}
                _ => self.everywhere = true,
            }
        }
    }
}

/// A changed tile of a plane placed at `origin`, in document pixels, grown by
/// `reach`.
fn placed(tile: &planes::TileDelta, origin: PsdRect, reach: i32) -> PsdRect {
    let r = tile_rect(tile);
    PsdRect {
        top: r.top + origin.top - reach,
        left: r.left + origin.left - reach,
        bottom: r.bottom + origin.top + reach,
        right: r.right + origin.left + reach,
    }
}

/// Whether two states would display differently with the same composite.
pub(super) fn display_differs(a: &Document, b: &Document) -> bool {
    (a.width, a.height, a.mode, a.depth) != (b.width, b.height, b.mode, b.depth)
        || a.document_icc != b.document_icc
}

/// Where a change of layer metadata (opacity, blend, visibility, position,
/// effects, order, an inserted or removed layer) between two hollow states
/// shows, on a document without a stored composite (#1216): the changed
/// tiles say nothing about it, and a stored composite's own tiles used to
/// bound it. The layers are matched in tree order; the run that differs is
/// bounded by each of its layers' influence in the state it belongs to. A
/// layer with no pixels shows nowhere. `Some(None)` when nothing shows,
/// `None` when a layer's reach is unbounded.
pub(super) fn meta_damage(before: &Document, after: &Document) -> Option<Option<PsdRect>> {
    fn rows<'a>(
        layers: &'a [pictura_core::Layer],
        prefix: &str,
        out: &mut Vec<(String, &'a pictura_core::Layer)>,
    ) {
        for (index, layer) in layers.iter().enumerate() {
            let path = if prefix.is_empty() {
                index.to_string()
            } else {
                format!("{prefix}/{index}")
            };
            out.push((path.clone(), layer));
            rows(&layer.children, &path, out);
        }
    }
    fn same(a: &pictura_core::Layer, b: &pictura_core::Layer) -> bool {
        let (mut a, mut b) = (a.clone(), b.clone());
        a.children = Default::default();
        b.children = Default::default();
        a == b
    }
    fn shows_nothing(layer: &pictura_core::Layer) -> bool {
        !layer.is_group && layer.adjustment.is_none() && layer.rect.is_empty()
    }
    let (mut a, mut b) = (Vec::new(), Vec::new());
    rows(&before.layers, "", &mut a);
    rows(&after.layers, "", &mut b);
    let prefix = a
        .iter()
        .zip(&b)
        .take_while(|((_, x), (_, y))| same(x, y))
        .count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|((_, x), (_, y))| same(x, y))
        .count();
    let mut rect: Option<PsdRect> = None;
    for (doc, run) in [
        (before, &a[prefix..a.len() - suffix]),
        (after, &b[prefix..b.len() - suffix]),
    ] {
        for (path, layer) in run {
            if shows_nothing(layer) {
                continue;
            }
            let reach = pictura_render::damage::influence(doc, path)?;
            rect = Some(rect.map_or(reach, |r| r.union(reach)));
        }
    }
    Some(rect)
}
