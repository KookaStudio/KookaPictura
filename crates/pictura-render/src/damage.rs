//! The document-space reach of a layer and the damage of a structural edit.
//!
//! A layer's influence is the union of its content box, its effect reach, a
//! raster-mask extent that confines an adjustment or fill, a fill/shape
//! vector-mask box, and the contiguous clipped run directly above it. An
//! adjustment's vector mask does not confine it. The run matters because
//! [`pictura_core::LayerStack`] releases or re-bases it when a base is removed
//! or moved.
//!
//! `None` means the reach cannot be proven, so the caller must fall back to the
//! full recomposite. A path that the edit creates is omitted from the before
//! set and included in the after set; a path the edit removes is included only
//! before. A path listed on a side where it should resolve but does not is an
//! unexpected missing path and is unbounded.

use pictura_core::{Document, Layer, PsdRect};

use crate::document_ops::{parent_path, resolve_path};

/// The document pixels whose value can depend on `path`, clipped to the canvas.
///
/// `None` when the path does not resolve, when the reach cannot be proven, or
/// when the document contains a knockout whose base the layer could affect.
pub fn influence(doc: &Document, path: &str) -> Option<PsdRect> {
    let layer = resolve_path(doc, path)?;
    // A knockout base is the bottom root layer (or the group-entry backdrop).
    // The layer-local reach cannot prove that a base reassignment stays inside
    // the edited path, so a document with a knockout keeps the full path.
    if doc
        .layers
        .iter()
        .any(crate::composite_knockout::has_knockout)
    {
        return None;
    }
    let mut rect = influence_of(doc, layer)?;
    let index: usize = path.rsplit('/').next()?.parse().ok()?;
    let siblings: &[Layer] = match parent_path(path) {
        Some(parent) => &resolve_path(doc, parent)?.children,
        None => &doc.layers,
    };
    for above in siblings.iter().skip(index + 1) {
        if !above.clipping {
            break;
        }
        rect = union_rect(rect, influence_of(doc, above)?);
    }
    clip_to_canvas(doc, rect)
}

fn canvas_rect(doc: &Document) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height.min(i32::MAX as u32) as i32,
        right: doc.width.min(i32::MAX as u32) as i32,
    }
}

/// The union of [`influence`] over every path; `None` if any path is unbounded.
///
/// The paths are the ones expected to resolve on this side of the edit. The
/// returned rectangles are clipped and overlapping rectangles are merged.
pub fn influence_paths(doc: &Document, paths: &[&str]) -> Option<Vec<PsdRect>> {
    let mut rects = Vec::with_capacity(paths.len());
    for &path in paths {
        rects.push(influence(doc, path)?);
    }
    Some(merge_overlapping(rects))
}

/// The union of two optional damage bounds. `None` wins: an unbounded side
/// makes the whole edit unbounded. Omit a side that does not exist instead of
/// passing `None` for it.
pub fn union(a: Option<PsdRect>, b: Option<PsdRect>) -> Option<PsdRect> {
    Some(union_rect(a?, b?))
}

fn influence_of(doc: &Document, layer: &Layer) -> Option<PsdRect> {
    if layer.is_group {
        let mut rect = None;
        for child in &layer.children {
            let child = influence_of(doc, child)?;
            rect = Some(match rect {
                None => child,
                Some(current) => union_rect(current, child),
            });
        }
        if let Some(mask) = &layer.mask {
            if mask.data.is_some() {
                rect = Some(union_rect(rect?, mask.rect));
            }
        }
        return rect;
    }
    let rect = crate::layer_effects::paint_bounds(layer, doc)?;
    if crate::layer_effects::is_destructive_adjustment(layer) {
        // An adjustment whose reach is the canvas is unbounded; a mask that
        // confines it is what makes the influence reportable.
        if rect == canvas_rect(doc) {
            return None;
        }
        return Some(rect);
    }
    Some(rect)
}

fn clip_to_canvas(doc: &Document, rect: PsdRect) -> Option<PsdRect> {
    let right = rect.right.min(doc.width.min(i32::MAX as u32) as i32);
    let bottom = rect.bottom.min(doc.height.min(i32::MAX as u32) as i32);
    let left = rect.left.max(0);
    let top = rect.top.max(0);
    (right > left && bottom > top).then_some(PsdRect {
        top,
        left,
        bottom,
        right,
    })
}

fn union_rect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.min(b.top),
        left: a.left.min(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.max(b.right),
    }
}

fn intersects(a: PsdRect, b: PsdRect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

fn merge_overlapping(rects: Vec<PsdRect>) -> Vec<PsdRect> {
    let mut out: Vec<PsdRect> = Vec::new();
    for mut rect in rects {
        let mut at = 0;
        while at < out.len() {
            if intersects(out[at], rect) {
                rect = union_rect(out.remove(at), rect);
            } else {
                at += 1;
            }
        }
        out.push(rect);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorMode, Knockout, LayerMask, LayerStack, VectorMask,
        VectorSubpath,
    };

    const W: u32 = 32;
    const H: u32 = 24;

    fn rect(top: i32, left: i32, width: i32, height: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom: top + height,
            right: left + width,
        }
    }

    fn pixel_layer(name: &str, at: PsdRect, color: u8, alpha: u8) -> Layer {
        let n = (at.width().max(0) * at.height().max(0)) as usize;
        Layer {
            name: name.into(),
            rect: at,
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            visible: true,
            channels: [0i16, 1, 2, -1]
                .map(|id| Channel {
                    id,
                    data: vec![if id == -1 { alpha } else { color }; n].into(),
                })
                .to_vec(),
            ..Default::default()
        }
    }

    fn group(name: &str, children: Vec<Layer>) -> Layer {
        Layer {
            name: name.into(),
            is_group: true,
            children: LayerStack::from_layers(children),
            ..Default::default()
        }
    }

    fn masked(rect: PsdRect, feather: f64) -> LayerMask {
        LayerMask {
            rect,
            default_color: 0,
            disabled: false,
            data: Some(vec![255u8; (rect.width() * rect.height()) as usize].into()),
            density: 255,
            feather_bits: feather.to_bits(),
            ..Default::default()
        }
    }

    fn doc(layers: Vec<Layer>) -> Document {
        let mut d = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
        d.layers = LayerStack::from_layers(layers);
        d
    }

    fn one_path_rect(doc: &Document, path: &str) -> Option<Vec<PsdRect>> {
        influence_paths(doc, &[path])
    }

    fn total(doc: &Document, paths: &[&str]) -> Option<PsdRect> {
        influence_paths(doc, paths)
            .map(|rects| rects.into_iter().reduce(union_rect).expect("paths"))
    }

    #[test]
    fn structural_delete_uses_influence() {
        let d = doc(vec![
            pixel_layer("bottom", rect(0, 0, W as i32, H as i32), 10, 255),
            pixel_layer("small", rect(3, 4, 6, 5), 200, 255),
        ]);
        let damage = one_path_rect(&d, "1").expect("bounded");
        assert_eq!(damage, vec![rect(3, 4, 6, 5)]);
    }

    #[test]
    fn structural_reorder_uses_influence() {
        let mut d = doc(vec![
            pixel_layer("bottom", rect(0, 0, W as i32, H as i32), 10, 255),
            pixel_layer("small", rect(3, 4, 6, 5), 200, 255),
            pixel_layer("top", rect(20, 18, 4, 4), 30, 255),
        ]);
        let before = total(&d, &["1"]).expect("bounded before");
        assert!(crate::move_path(&mut d, "1", 1));
        let after = total(&d, &["2"]).expect("bounded after");
        let damage = union(Some(before), Some(after)).expect("bounded");
        assert!(damage.width() >= 6 && damage.height() >= 5);
    }

    #[test]
    fn clipping_run_is_influenced() {
        let d = doc(vec![
            pixel_layer("base", rect(0, 0, 10, 10), 10, 255),
            Layer {
                clipping: true,
                blend: BlendMode::Multiply,
                ..pixel_layer("clip", rect(5, 5, 20, 20), 200, 200)
            },
        ]);
        let base = influence(&d, "0").expect("bounded");
        assert!(base.left <= 5 && base.top <= 5 && base.right >= 25 && base.bottom >= H as i32);
    }

    #[test]
    fn group_and_ungroup_are_bounded() {
        let d = doc(vec![group(
            "g",
            vec![
                pixel_layer("a", rect(2, 3, 4, 5), 10, 255),
                pixel_layer("b", rect(10, 12, 6, 7), 20, 255),
            ],
        )]);
        assert_eq!(influence(&d, "0").expect("bounded"), rect(2, 3, 15, 15));

        let mut d = doc(vec![group(
            "g",
            vec![
                pixel_layer("a", rect(2, 3, 4, 5), 10, 255),
                pixel_layer("b", rect(10, 12, 6, 7), 20, 255),
            ],
        )]);
        assert_eq!(crate::ungroup_paths(&mut d, &["0"]), 1);
        let after = influence_paths(&d, &["0", "1"]).expect("bounded children");
        assert_eq!(after.len(), 2);
        assert!(after.contains(&rect(2, 3, 4, 5)));
        assert!(after.contains(&rect(10, 12, 6, 7)));
    }

    #[test]
    fn duplicate_and_insertion_are_bounded() {
        let mut d = doc(vec![
            pixel_layer("bottom", rect(0, 0, W as i32, H as i32), 10, 255),
            pixel_layer("small", rect(3, 4, 6, 5), 200, 255),
        ]);
        let created = crate::duplicate_layer(&mut d, 1);
        assert_eq!(created, 2);
        assert_eq!(influence(&d, "2").expect("bounded"), rect(3, 4, 6, 5));

        let rgba = [255u8; 4 * 4 * 4];
        let inserted = crate::add_raster_layer_from_rgba(&mut d, "inserted", 4, 4, &rgba);
        assert!(!inserted.is_empty());
        assert_eq!(influence(&d, &inserted).expect("bounded"), rect(0, 0, 4, 4));
    }

    #[test]
    fn unmasked_adjustment_is_unbounded() {
        let d = doc(vec![Layer {
            name: "adjustment".into(),
            adjustment: Some(crate::encode_brightness_contrast(10, 0)),
            rect: rect(0, 0, W as i32, H as i32),
            ..Default::default()
        }]);
        assert_eq!(influence(&d, "0"), None);
    }

    #[test]
    fn feathered_adjustment_is_bounded() {
        let mask = masked(rect(16, 20, 4, 4), 8.0);
        let d = doc(vec![Layer {
            name: "adjustment".into(),
            adjustment: Some(crate::encode_brightness_contrast(10, 0)),
            rect: rect(0, 0, W as i32, H as i32),
            mask: Some(mask),
            ..Default::default()
        }]);
        let damage = influence(&d, "0").expect("bounded");
        assert!(damage.width() > 4 && damage.height() > 4);
        assert!(damage.left < 20 && damage.top < 16);
    }

    #[test]
    fn unexpected_missing_path_is_unbounded() {
        let d = doc(vec![pixel_layer("a", rect(0, 0, 4, 4), 10, 255)]);
        assert_eq!(influence(&d, "9"), None);
    }

    #[test]
    fn knockout_base_change_is_unbounded() {
        let d = doc(vec![
            pixel_layer("bottom", rect(0, 0, W as i32, H as i32), 10, 255),
            Layer {
                knockout: Knockout::Shallow,
                ..pixel_layer("knockout", rect(4, 4, 8, 8), 20, 255)
            },
        ]);
        assert_eq!(influence(&d, "0"), None);
    }

    #[test]
    fn vector_mask_adjustment_is_unbounded() {
        let mut mask = VectorMask::default();
        mask.subpaths.push(VectorSubpath {
            closed: true,
            points: vec![[512, 512], [1024, 512], [1024, 1024], [512, 1024]],
            ..Default::default()
        });
        let d = doc(vec![Layer {
            name: "adjustment".into(),
            adjustment: Some(crate::encode_brightness_contrast(10, 0)),
            rect: rect(0, 0, W as i32, H as i32),
            vector_mask: Some(mask),
            ..Default::default()
        }]);
        assert_eq!(influence(&d, "0"), None);
    }

    #[test]
    fn clipping_run_above_a_deleted_base_is_included() {
        let mut d = doc(vec![
            pixel_layer("bottom", rect(0, 0, 10, 10), 10, 255),
            pixel_layer("base", rect(0, 0, 10, 10), 20, 255),
            Layer {
                clipping: true,
                ..pixel_layer("clip", rect(10, 20, 4, 4), 30, 255)
            },
        ]);
        let damage = influence(&d, "1").expect("bounded");
        assert!(damage.right >= 24 && damage.bottom >= 14);
        assert_eq!(crate::delete_paths_report(&mut d, &["1"]).len(), 1);
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    fn covers(rect: PsdRect, x: i32, y: i32) -> bool {
        x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
    }

    fn assert_covered(
        before: &pictura_core::PixelBuffer,
        after: &pictura_core::PixelBuffer,
        damage: Option<PsdRect>,
    ) {
        let Some(damage) = damage else {
            return;
        };
        let plane = (before.width * before.height) as usize;
        for (i, (a, b)) in before
            .data
            .as_slice()
            .iter()
            .zip(after.data.as_slice())
            .enumerate()
        {
            if a == b {
                continue;
            }
            let pixel = i % plane;
            let x = (pixel as u32 % before.width) as i32;
            let y = (pixel as u32 / before.width) as i32;
            assert!(
                covers(damage, x, y),
                "changed pixel ({x}, {y}) outside damage {damage:?}"
            );
        }
    }

    fn fixed_doc() -> Document {
        let mut styled = pixel_layer("styled", rect(3, 4, 10, 8), 200, 255);
        crate::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
        crate::set_layer_style_value(&mut styled, "dropShadow.size", 3.0);
        let clip = Layer {
            clipping: true,
            ..pixel_layer("clip", rect(0, 0, 12, 10), 120, 200)
        };
        let mut masked_layer = pixel_layer("masked", rect(10, 10, 8, 8), 40, 255);
        masked_layer.mask = Some(masked(rect(10, 10, 8, 8), 3.0));
        let adjustment = Layer {
            name: "adjustment".into(),
            adjustment: Some(crate::encode_brightness_contrast(20, 0)),
            rect: rect(0, 0, W as i32, H as i32),
            mask: Some(masked(rect(24, 16, 5, 5), 0.0)),
            ..Default::default()
        };
        doc(vec![
            pixel_layer("bottom", rect(0, 0, W as i32, H as i32), 10, 255),
            styled,
            clip,
            group(
                "g",
                vec![
                    pixel_layer("g0", rect(20, 10, 6, 6), 70, 255),
                    pixel_layer("g1", rect(22, 12, 4, 4), 90, 255),
                ],
            ),
            masked_layer,
            adjustment,
        ])
    }

    #[test]
    fn random_structural_edits_are_covered() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut d = fixed_doc();
        let mut bounded = 0;
        for _ in 0..24 {
            let before = d.clone();
            let mut after = before.clone();
            let len = before.layers.len();
            let damage: Option<PsdRect> = match rng.below(11) {
                0 => {
                    let i = rng.below(len);
                    let delta = if rng.below(2) == 0 { 1 } else { -1 };
                    let path = i.to_string();
                    let before_rect = total(&before, &[&path]);
                    if !crate::move_path(&mut after, &path, delta) {
                        continue;
                    }
                    let landed = (i as i32 + delta).clamp(0, len as i32 - 1).to_string();
                    union(before_rect, total(&after, &[&landed]))
                }
                1 => {
                    let i = rng.below(len);
                    let created = crate::duplicate_layer(&mut after, i as i32);
                    if created < 0 {
                        continue;
                    }
                    union(
                        total(&before, &[&i.to_string()]),
                        total(&after, &[&created.to_string()]),
                    )
                }
                2 => {
                    let i = 1 + rng.below(len.saturating_sub(1).max(1));
                    if i >= len {
                        continue;
                    }
                    let path = i.to_string();
                    let before_rect = total(&before, &[&path]);
                    if crate::delete_paths_report(&mut after, &[&path]).is_empty() {
                        continue;
                    }
                    before_rect
                }
                3 => {
                    let i = rng.below(len);
                    let path = i.to_string();
                    let damage_rect = total(&before, &[&path]);
                    let Some(layer) = crate::resolve_path_mut(&mut after, &path) else {
                        continue;
                    };
                    layer.visible = !layer.visible;
                    damage_rect
                }
                4 => {
                    let i = rng.below(len.saturating_sub(1).max(1));
                    if i + 1 >= len {
                        continue;
                    }
                    let (a, b) = (i.to_string(), (i + 1).to_string());
                    let before_rect = total(&before, &[&a, &b]);
                    let Some(created) = crate::group_paths(&mut after, &[&a, &b]) else {
                        continue;
                    };
                    union(before_rect, total(&after, &[&created]))
                }
                5 => {
                    let group_index = (0..len).find(|&i| before.layers[i].is_group);
                    let Some(group_index) = group_index else {
                        continue;
                    };
                    let child_count = before.layers[group_index].children.len();
                    if child_count == 0 {
                        continue;
                    }
                    let group_path = group_index.to_string();
                    let before_rect = total(&before, &[&group_path]);
                    if crate::ungroup_paths(&mut after, &[&group_path]) == 0 {
                        continue;
                    }
                    let children: Vec<String> = (0..child_count)
                        .map(|offset| (group_index + offset).to_string())
                        .collect();
                    let refs: Vec<&str> = children.iter().map(String::as_str).collect();
                    union(before_rect, total(&after, &refs))
                }
                6 => {
                    let i = 1 + rng.below(len.saturating_sub(1).max(1));
                    if i >= len {
                        continue;
                    }
                    let path = i.to_string();
                    let before_rect = total(&before, &[&path]);
                    let changed = if before.layers[i].clipping {
                        crate::release_clipping_mask(&mut after, &[&path])
                    } else {
                        crate::create_clipping_mask(&mut after, &[&path])
                    };
                    if changed == 0 {
                        continue;
                    }
                    union(before_rect, total(&after, &[&path]))
                }
                7 => {
                    let path = "1";
                    let before_rect = total(&before, &[path]);
                    let Some(layer) = crate::resolve_path_mut(&mut after, path) else {
                        continue;
                    };
                    let _ = crate::set_layer_style_value(layer, "dropShadow.size", 6.0);
                    union(before_rect, total(&after, &[path]))
                }
                8 => {
                    let path = "5";
                    let before_rect = total(&before, &[path]);
                    let Some(layer) = crate::resolve_path_mut(&mut after, path) else {
                        continue;
                    };
                    layer.adjustment = Some(crate::encode_brightness_contrast(30, 0));
                    union(before_rect, total(&after, &[path]))
                }
                9 => {
                    let path = "5";
                    let before_rect = total(&before, &[path]);
                    if !crate::set_layer_mask_feather(&mut after, path, 8.0) {
                        continue;
                    }
                    union(before_rect, total(&after, &[path]))
                }
                _ => {
                    let path = "1";
                    let before_rect = total(&before, &[path]);
                    let Some(layer) = crate::resolve_path_mut(&mut after, path) else {
                        continue;
                    };
                    layer.knockout = if layer.knockout == Knockout::None {
                        Knockout::Shallow
                    } else {
                        Knockout::None
                    };
                    union(before_rect, total(&after, &[path]))
                }
            };
            let before_pixels = crate::composite_rgba(&before);
            let after_pixels = crate::composite_rgba(&after);
            assert_covered(&before_pixels, &after_pixels, damage);
            if damage.is_some() {
                bounded += 1;
            }
            d = after;
        }
        assert!(
            bounded > 0,
            "the generated cases exercised the bounded path"
        );
    }

    #[test]
    fn duplicate_and_insertion_paths_are_one_sided() {
        let mut d = doc(vec![pixel_layer("a", rect(0, 0, 4, 4), 10, 255)]);
        let created = crate::duplicate_layer(&mut d, 0);
        assert_eq!(created, 1);
        assert_eq!(influence(&d, "0"), Some(rect(0, 0, 4, 4)));
        assert_eq!(influence(&d, "1"), Some(rect(0, 0, 4, 4)));
    }
}
