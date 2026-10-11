//! `Image > Crop` and layer translation at document scope.
//!
//! A crop is a canvas resize with a negative offset: the new origin is the
//! clamped rect's top-left, so every rect shifts by `(-x, -y)` and every
//! document channel is re-blitted into the smaller canvas.

use pictura_core::{layer_move_locked, Document, Layer, PsdRect};

use super::canvas::{extend_channel, offset_rect, rebase_source_planes};
use super::{for_each_layer, recompute, resolve_path_mut};

/// Crop the document to the clamped `width`×`height` rect at `(x, y)`.
///
/// Returns false and leaves the document untouched when the intersection with
/// the canvas is empty.
pub fn crop_document(doc: &mut Document, x: i32, y: i32, width: u32, height: u32) -> bool {
    let old_w = doc.width;
    let old_h = doc.height;
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = x.saturating_add(width as i32).min(old_w as i32);
    let y1 = y.saturating_add(height as i32).min(old_h as i32);
    if x1 <= x0 || y1 <= y0 {
        return false;
    }
    crop_to_frame(doc, x0, y0, (x1 - x0) as u32, (y1 - y0) as u32)
}

/// Crop to the `width`×`height` rect at `(x, y)` where the rect may extend
/// beyond the canvas: the document grows to the rect and the added area starts
/// transparent. Unlike [`crop_document`] nothing is clamped, so a box dragged
/// past the original edge grows the canvas. Rejects a zero dimension or an
/// empty document.
pub fn crop_document_grow(doc: &mut Document, x: i32, y: i32, width: u32, height: u32) -> bool {
    if width == 0 || height == 0 || doc.width == 0 || doc.height == 0 {
        return false;
    }
    crop_to_frame(doc, x, y, width, height)
}

/// Re-frame the document to the `new_w`×`new_h` window whose top-left is the
/// old-canvas point `(x0, y0)`; `x0`/`y0` may be negative to grow left/up.
/// Pixels are shifted, not resampled, and the uncovered area is transparent.
fn crop_to_frame(doc: &mut Document, x0: i32, y0: i32, new_w: u32, new_h: u32) -> bool {
    let old_w = doc.width;
    let old_h = doc.height;
    let (dx, dy) = (-x0, -y0);

    for_each_layer(&mut doc.layers, &mut |layer| {
        layer.rect = offset_rect(layer.rect, dx, dy);
        offset_store(layer, dx, dy);
        if let Some(mask) = &mut layer.mask {
            mask.rect = offset_rect(mask.rect, dx, dy);
        }
    });

    for channel in &mut doc.channels {
        channel.data = extend_channel(&channel.data, old_w, old_h, new_w, new_h, dx, dy).into();
    }
    rebase_source_planes(doc, old_w, old_h, new_w, new_h, dx, dy);

    doc.width = new_w;
    doc.height = new_h;
    recompute(doc);
    true
}

/// The Crop tool's "Delete Cropped Pixels": after [`crop_document`], discard
/// each pixel layer's pixels (and its mask plane) outside the canvas, so they
/// cannot be revealed by enlarging the canvas again. A layer entirely off the
/// canvas keeps its place in the stack but loses its pixels. Returns how many
/// layers were trimmed.
///
/// ponytail: layers with live type, a smart object, a vector mask, or retained
/// 16/32-bit samples keep their off-canvas pixels (as with the checkbox off);
/// trimming them needs the native/vector stores trimmed in step.
pub fn delete_cropped_pixels(doc: &mut Document) -> usize {
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let mut trimmed = 0;
    for_each_layer(&mut doc.layers, &mut |layer| {
        let rect = layer.rect;
        let live = layer.is_type()
            || layer.smart_object.is_some()
            || layer.vector_mask.is_some()
            || layer.source_channels.is_some();
        if layer.is_group || layer.channels.is_empty() || live || rect.width() <= 0 {
            return;
        }
        let keep = intersect(rect, canvas);
        if keep == rect {
            return;
        }
        let empty = keep.width() <= 0 || keep.height() <= 0;
        let keep = if empty {
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            }
        } else {
            keep
        };
        for channel in &mut layer.channels {
            channel.data = trim_plane(&channel.data, rect, keep).into();
        }
        if let Some(mask) = &mut layer.mask {
            if let Some(data) = &mask.data {
                let mask_keep = intersect(mask.rect, canvas);
                if mask_keep.width() > 0 && mask_keep.height() > 0 {
                    mask.data = Some(trim_plane(data, mask.rect, mask_keep).into());
                    mask.rect = mask_keep;
                }
            }
        }
        layer.rect = keep;
        layer.raw_channels.clear();
        trimmed += 1;
    });
    if trimmed > 0 {
        recompute(doc);
    }
    trimmed
}

fn intersect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.max(b.top),
        left: a.left.max(b.left),
        bottom: a.bottom.min(b.bottom),
        right: a.right.min(b.right),
    }
}

/// The `keep` window of a plane laid out over `rect` (`keep` inside `rect`).
fn trim_plane(data: &[u8], rect: PsdRect, keep: PsdRect) -> Vec<u8> {
    extend_channel(
        data,
        rect.width().max(0) as u32,
        rect.height().max(0) as u32,
        keep.width().max(0) as u32,
        keep.height().max(0) as u32,
        rect.left - keep.left,
        rect.top - keep.top,
    )
}

/// Shift a layer's retained-store rect with its bounds; the plane data is
/// layer-local, so a pure move re-anchors it without resampling.
fn offset_store(layer: &mut Layer, dx: i32, dy: i32) {
    if let Some(store) = &mut layer.source_channels {
        store.rect = offset_rect(store.rect, dx, dy);
    }
}

/// Shift a pixel layer's bounds, retained store, and mask by `(dx, dy)`; no
/// pixels move, the compositor reads the shifted `rect`.
pub(crate) fn offset_layer(layer: &mut Layer, dx: i32, dy: i32) {
    layer.rect = offset_rect(layer.rect, dx, dy);
    offset_store(layer, dx, dy);
    if let Some(mask) = &mut layer.mask {
        mask.rect = offset_rect(mask.rect, dx, dy);
    }
}

/// Shift the topmost pixel layer's bounds by `(dx, dy)`.
///
/// Groups and adjustment layers are ignored; no pixels move, the compositor
/// reads the shifted `rect`. Returns false when there is no pixel layer.
pub fn translate_layer(doc: &mut Document, dx: i32, dy: i32) -> bool {
    let Some(layer) = topmost_pixel_layer(&mut doc.layers) else {
        return false;
    };
    if layer_move_locked(layer) {
        return false;
    }
    offset_layer(layer, dx, dy);
    recompute(doc);
    true
}

/// Shift the topmost pixel layer's bounds by `(dx, dy)` without recompositing.
///
/// Same rect/mask shift as [`translate_layer`]; the caller is expected to
/// refresh the dirty region through the active backend. Use [`translate_layer`]
/// or [`translate_layer_active`] when a full recomposite is wanted. Returns
/// false when there is no pixel layer.
pub fn translate_layer_rect(doc: &mut Document, dx: i32, dy: i32) -> bool {
    let Some(layer) = topmost_pixel_layer(&mut doc.layers) else {
        return false;
    };
    if layer_move_locked(layer) {
        return false;
    }
    offset_layer(layer, dx, dy);
    true
}

/// Shift the topmost pixel layer's bounds by `(dx, dy)` and refresh the
/// composite through the active backend.
///
/// Same rect/mask shift as [`translate_layer`]; only the composite refresh
/// differs — `composite_active` uses the GPU when `gpu_enabled` and an adapter
/// is usable, falling back to the CPU oracle otherwise.
pub fn translate_layer_active(doc: &mut Document, dx: i32, dy: i32, gpu_enabled: bool) -> bool {
    let Some(layer) = topmost_pixel_layer(&mut doc.layers) else {
        return false;
    };
    if layer_move_locked(layer) {
        return false;
    }
    offset_layer(layer, dx, dy);
    let (composite, _) = crate::gpu::composite_active(doc, gpu_enabled);
    doc.composite = composite;
    true
}

/// Shift the top-level pixel layer `index`'s bounds by `(dx, dy)` without
/// recompositing; the caller refreshes the dirty region. Returns false for an
/// out-of-range index, a group/adjustment, or a position-locked layer.
pub fn translate_layer_index(doc: &mut Document, index: usize, dx: i32, dy: i32) -> bool {
    let Some(layer) = doc.layers.get_mut(index) else {
        return false;
    };
    if layer.is_group || layer.adjustment.is_some() {
        return false;
    }
    if layer_move_locked(layer) {
        return false;
    }
    offset_layer(layer, dx, dy);
    true
}

/// Shift the layer at `path`'s bounds by `(dx, dy)` without recompositing; the
/// caller refreshes the dirty region. Returns false for an unresolved path, a
/// group/adjustment, or a position-locked layer.
pub fn translate_layer_path(doc: &mut Document, path: &str, dx: i32, dy: i32) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.is_group || layer.adjustment.is_some() {
        return false;
    }
    if layer_move_locked(layer) {
        return false;
    }
    offset_layer(layer, dx, dy);
    true
}

fn topmost_pixel_layer(layers: &mut [Layer]) -> Option<&mut Layer> {
    layers
        .iter_mut()
        .rev()
        .find(|l| l.adjustment.is_none() && !l.is_group)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorLabel, ColorMode, LayerMask, LockFlags, PsdRect,
        Samples, SourceChannels, SourcePlanes,
    };

    fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    fn full(w: u32, h: u32) -> PsdRect {
        rect(0, 0, h as i32, w as i32)
    }

    fn pixel_layer(name: &str, r: PsdRect, mask: Option<LayerMask>) -> Layer {
        let n = (r.width().max(0) * r.height().max(0)) as usize;
        let ramp = |offset: u8| (0..n).map(|i| (i as u8).wrapping_add(offset)).collect();
        Layer {
            name: name.into(),
            rect: r,
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask,
            adjustment: None,
            channels: vec![
                Channel {
                    id: 0,
                    data: ramp(0),
                },
                Channel {
                    id: 1,
                    data: ramp(100),
                },
                Channel {
                    id: 2,
                    data: ramp(200),
                },
                Channel {
                    id: -1,
                    data: vec![255; n].into(),
                },
            ],
            children: Vec::new(),
            is_group: false,
            background: false,
            ..Default::default()
        }
    }

    fn sample_doc() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![pixel_layer("layer", full(4, 4), None)];
        doc.channels = vec![Channel {
            id: -1,
            data: (0..16u8).collect(),
        }];
        doc
    }

    #[test]
    fn crop_remaps_pixel_and_resizes_document() {
        let mut doc = sample_doc();
        assert!(crop_document(&mut doc, 1, 1, 2, 2));

        assert_eq!((doc.width, doc.height), (2, 2));
        assert_eq!(doc.channels[0].data, vec![5, 6, 9, 10]);
        assert_eq!(doc.layers[0].rect, rect(-1, -1, 3, 3));

        // The layer rect moved to (-1,-1), so doc (0,0) samples local (1,1) = 5.
        let plane = 4usize;
        assert_eq!(doc.composite.width, 2);
        assert_eq!(doc.composite.height, 2);
        assert_eq!(doc.composite.data[0], 5);
        assert_eq!(doc.composite.data[plane], 105);
        assert_eq!(doc.composite.data[2 * plane], 205);
    }

    #[test]
    fn crop_clamps_and_shifts_out_of_bounds_rect() {
        let mut doc = sample_doc();
        assert!(crop_document(&mut doc, 3, 3, 4, 4));
        assert_eq!((doc.width, doc.height), (1, 1));
        assert_eq!(doc.channels[0].data, vec![15]);
        assert_eq!(doc.layers[0].rect, rect(-3, -3, 1, 1));
        assert_eq!(doc.composite.data[0], 15, "old (3,3) becomes the origin");
    }

    #[test]
    fn crop_document_grow_extends_the_canvas_and_pads_transparent() {
        let mut doc = sample_doc();
        assert!(crop_document_grow(&mut doc, -1, -1, 6, 6));
        assert_eq!((doc.width, doc.height), (6, 6));
        assert_eq!(doc.channels[0].data.len(), 36);
        assert_eq!(
            doc.channels[0].data[0], 0,
            "the added border is transparent"
        );
        let plane = 6usize;
        // The old (0,0) lands at (1,1); the old (1,0) at (2,1).
        assert_eq!(doc.channels[0].data[plane + 1], 0);
        assert_eq!(doc.channels[0].data[plane + 2], 1);
        assert_eq!(doc.layers[0].rect, rect(1, 1, 5, 5));
        assert!(
            !crop_document_grow(&mut doc, 0, 0, 0, 2),
            "zero width is refused"
        );
    }

    #[test]
    fn crop_empty_rect_is_rejected_and_unchanged() {
        let mut doc = sample_doc();
        let before = doc.clone();
        assert!(!crop_document(&mut doc, 10, 10, 2, 2));
        assert!(!crop_document(&mut doc, 0, 0, 0, 2));
        assert!(!crop_document(&mut doc, 0, 0, 2, 0));
        assert_eq!(doc, before);
    }

    #[test]
    fn crop_shifts_layer_mask() {
        let mask = LayerMask {
            rect: full(4, 4),
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(vec![128; 16].into()),
            ..Default::default()
        };
        let mut doc = sample_doc();
        doc.layers[0] = pixel_layer("masked", full(4, 4), Some(mask));
        assert!(crop_document(&mut doc, 1, 1, 2, 2));
        assert_eq!(
            doc.layers[0].mask.as_ref().unwrap().rect,
            rect(-1, -1, 3, 3)
        );
    }

    #[test]
    fn translate_layer_shifts_rect_and_recomposites() {
        let mut doc = sample_doc();
        let before = doc.composite.data[0];
        assert_eq!(before, 0, "old origin holds layer local (0,0)");

        assert!(translate_layer(&mut doc, 1, 1));
        assert_eq!(doc.layers[0].rect, rect(1, 1, 5, 5));

        let plane = 16usize;
        // Doc (1,1) now samples layer local (0,0); doc (2,1) samples local (1,0).
        assert_eq!(doc.composite.data[5], 0);
        assert_eq!(doc.composite.data[6], 1);
        assert_eq!(doc.composite.data[2 * 4 + 1], 4);
        // The old origin is no longer covered by the shifted layer.
        assert_eq!(doc.composite.data[3 * plane], 0, "uncovered alpha");
    }

    #[test]
    fn translate_layer_active_cpu_matches_translate_layer() {
        let mut active = sample_doc();
        let mut oracle = sample_doc();
        assert!(translate_layer_active(&mut active, 1, 1, false));
        assert!(translate_layer(&mut oracle, 1, 1));
        assert_eq!(active, oracle);
        assert_eq!(active.composite.data, oracle.composite.data);
    }

    #[test]
    fn translate_layer_rect_shifts_without_recompositing() {
        let mut doc = sample_doc();
        let before = doc.composite.clone();
        let mask = LayerMask {
            rect: full(4, 4),
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(vec![128; 16].into()),
            ..Default::default()
        };
        doc.layers[0] = pixel_layer("masked", full(4, 4), Some(mask));

        assert!(translate_layer_rect(&mut doc, 2, -1));
        assert_eq!(doc.layers[0].rect, rect(-1, 2, 3, 6));
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().rect, rect(-1, 2, 3, 6));
        assert_eq!(doc.composite.data, before.data, "no recomposite");
    }

    #[test]
    fn translate_layer_rejects_non_pixel_layers() {
        let mut doc = sample_doc();
        doc.layers = vec![Layer {
            name: "adj".into(),
            rect: full(4, 4),
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: Some(pictura_core::AdjustmentData {
                key: *b"nvrt",
                data: vec![],
            }),
            channels: Vec::new(),
            children: Vec::new(),
            is_group: false,
            background: false,
            ..Default::default()
        }];
        let before = doc.clone();
        assert!(!translate_layer(&mut doc, 2, 2));
        assert_eq!(doc, before);
    }

    #[test]
    fn position_lock_blocks_translate_but_allows_reorder() {
        let mut doc = sample_doc();
        doc.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
        let before = doc.clone();
        assert!(
            !translate_layer(&mut doc, 2, 0),
            "position lock refuses translate"
        );
        assert!(!translate_layer_rect(&mut doc, 2, 0));
        assert_eq!(doc, before, "refusal leaves the document unchanged");

        let mut doc = sample_doc();
        doc.layers.push(pixel_layer("top", full(4, 4), None));
        doc.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
        assert!(
            super::super::move_path(&mut doc, "0", 1),
            "structural reorder ignores the position lock"
        );
        assert_eq!(doc.layers[0].name, "top");
        assert_eq!(doc.layers[1].name, "layer");
    }

    #[test]
    fn crop_rebases_the_retained_store() {
        let mut doc = sample_doc();
        doc.source_depth = Some(BitDepth::Sixteen);
        doc.source_planes = Some(SourcePlanes {
            depth: BitDepth::Sixteen,
            width: 4,
            height: 4,
            samples: Samples::U16((0..16).map(|i| i as u16).collect()),
        });
        doc.layers[0].source_channels = Some(SourceChannels {
            depth: BitDepth::Sixteen,
            rect: doc.layers[0].rect,
            planes: vec![(0, Samples::U16(vec![0; 16]))],
        });

        assert!(crop_document(&mut doc, 1, 1, 2, 2));

        let store = doc.source_planes.as_ref().unwrap();
        assert_eq!((store.width, store.height), (2, 2));
        assert_eq!(store.samples, Samples::U16(vec![5, 6, 9, 10]));
        assert_eq!(
            doc.layers[0].source_channels.as_ref().unwrap().rect,
            doc.layers[0].rect,
            "the layer store rect follows the crop offset"
        );
    }

    /// Manual M31 evidence: a small region composite against the full 4000²
    /// composite through the active backend. Ignored by default (it allocates a
    /// 4000² layer); run with `--ignored --nocapture`.
    #[test]
    #[ignore = "manual 4000x4000 region-vs-full timing measurement"]
    fn region_move_timing_4000() {
        use std::time::Instant;

        let (w, h) = (4000u32, 4000u32);
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![pixel_layer(
            "big",
            PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            None,
        )];
        doc.composite = crate::composite_rgba(&doc);

        let full = Instant::now();
        let _ = crate::composite_active(&doc, true);
        let full_ms = full.elapsed().as_secs_f64() * 1000.0;

        let rect = PsdRect {
            top: 1000,
            left: 1000,
            bottom: 1064,
            right: 1064,
        };
        let region = Instant::now();
        let (buffer, backend) = crate::gpu::composite_region_active(&doc, rect, true);
        let region_ms = region.elapsed().as_secs_f64() * 1000.0;

        assert_eq!((buffer.width, buffer.height), (64, 64));
        eprintln!(
            "region timing 4000^2 backend={backend:?} full={full_ms:.1}ms region64={region_ms:.3}ms"
        );
    }

    #[test]
    fn delete_cropped_pixels_trims_layers_and_masks_to_the_canvas() {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        let mask = LayerMask {
            rect: full(8, 8),
            default_color: 0,
            data: Some((0..64).map(|i| i as u8).collect()),
            ..Default::default()
        };
        doc.layers = vec![
            pixel_layer("kept", full(8, 8), Some(mask)),
            pixel_layer("off", rect(0, 0, 2, 2), None),
        ];
        assert!(crop_document(&mut doc, 2, 3, 4, 4));
        // Before trimming the layer still hangs off the new canvas.
        assert_eq!(doc.layers[0].rect, rect(-3, -2, 5, 6));

        assert_eq!(delete_cropped_pixels(&mut doc), 2);
        let kept = &doc.layers[0];
        assert_eq!(kept.rect, full(4, 4));
        // Old pixel (x=2, y=3) is the new top-left: ramp index 3*8+2.
        assert_eq!(kept.channels[0].data.len(), 16);
        assert_eq!(kept.channels[0].data[0], 26);
        let mask = kept.mask.as_ref().unwrap();
        assert_eq!(
            (mask.rect, mask.data.as_ref().unwrap()[0]),
            (full(4, 4), 26)
        );
        let off = &doc.layers[1];
        assert_eq!(
            off.rect,
            rect(0, 0, 0, 0),
            "an off-canvas layer loses its pixels"
        );
        assert!(off.channels.iter().all(|c| c.data.is_empty()));
        assert_eq!(delete_cropped_pixels(&mut doc), 0, "nothing left to trim");
    }

    #[test]
    fn delete_cropped_pixels_leaves_native_depth_layers_alone() {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("native", full(8, 8), None);
        layer.source_channels = Some(SourceChannels {
            depth: BitDepth::Sixteen,
            rect: full(8, 8),
            planes: Vec::new(),
        });
        doc.layers = vec![layer];
        assert!(crop_document(&mut doc, 2, 2, 4, 4));
        assert_eq!(delete_cropped_pixels(&mut doc), 0);
        assert_eq!(doc.layers[0].rect, rect(-2, -2, 6, 6));
    }
}
