//! `Image > Canvas Size` at document scope (`IMG-002`).

use super::{for_each_layer, recompute, valid_size};

// ponytail: the added canvas is transparent (fill 0) — the layer-stack model has
// no background layer, so the reference's background/extension-color options are out
// of scope for M12.
pub fn resize_canvas_document(
    doc: &mut pictura_core::Document,
    width: u32,
    height: u32,
    anchor: pictura_ops::Anchor,
) -> Result<(), pictura_ops::OpsError> {
    valid_size(width, height)?;
    let old_w = doc.width;
    let old_h = doc.height;
    if old_w == 0 || old_h == 0 {
        return Err(pictura_ops::OpsError::InvalidParams(
            "document has a zero dimension".into(),
        ));
    }

    let (dx, dy) = anchor_offset(anchor, old_w, old_h, width, height);

    for_each_layer(&mut doc.layers, &mut |layer| {
        layer.rect = offset_rect(layer.rect, dx, dy);
        if let Some(store) = &mut layer.source_channels {
            store.rect = offset_rect(store.rect, dx, dy);
        }
        if let Some(mask) = &mut layer.mask {
            mask.rect = offset_rect(mask.rect, dx, dy);
        }
    });

    for channel in &mut doc.channels {
        channel.data = extend_channel(&channel.data, old_w, old_h, width, height, dx, dy).into();
    }
    rebase_source_planes(doc, old_w, old_h, width, height, dx, dy);

    doc.width = width;
    doc.height = height;
    recompute(doc);
    Ok(())
}

/// Grow the Background layer to cover the whole canvas, filling what it did
/// not cover with `color` (Canvas Size's extension colour). False when there
/// is no Background or it already covers the canvas.
/// ponytail: the extended Background drops any retained 16/32-bit or Lab
/// source samples, so it re-saves from its 8-bit pixels.
pub fn extend_background(doc: &mut pictura_core::Document, color: [u8; 3]) -> bool {
    let canvas = pictura_core::PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let gray = doc.mode == pictura_core::ColorMode::Grayscale;
    let Some(layer) = doc.layers.iter_mut().find(|l| l.background && !l.is_group) else {
        return false;
    };
    if layer.rect == canvas {
        return false;
    }
    let old = layer.rect;
    let (width, height) = (canvas.width() as u32, canvas.height() as u32);
    let old_w = old.width().max(0) as usize;
    for channel in &mut layer.channels {
        let fill = match channel.id {
            0 if gray => {
                let [r, g, b] = color.map(f64::from);
                (0.299 * r + 0.587 * g + 0.114 * b).round() as u8
            }
            id @ 0..=2 => color[id as usize],
            -1 => 255,
            _ => 0,
        };
        let mut plane = vec![fill; width as usize * height as usize];
        let x0 = old.left.max(0);
        let x1 = old.right.min(width as i32);
        for y in old.top.max(0)..old.bottom.min(height as i32) {
            let src = (y - old.top) as usize * old_w;
            let dst = y as usize * width as usize;
            for x in x0..x1 {
                if let Some(&v) = channel.data.get(src + (x - old.left) as usize) {
                    plane[dst + x as usize] = v;
                }
            }
        }
        channel.data = plane.into();
    }
    layer.rect = canvas;
    layer.source_channels = None;
    recompute(doc);
    true
}

/// Offset-blit a retained composite store into the resized canvas so its dims
/// still match the document and `composite_retained` stays valid.
pub(super) fn rebase_source_planes(
    doc: &mut pictura_core::Document,
    old_w: u32,
    old_h: u32,
    width: u32,
    height: u32,
    dx: i32,
    dy: i32,
) {
    let Some(store) = &mut doc.source_planes else {
        return;
    };
    if store.depth == pictura_core::BitDepth::One {
        // The depth-1 store is a packed plane, not a `width * height` one; it is
        // only re-emitted for an unchanged flat Bitmap, so leave it stale.
        return;
    }
    store.samples = super::native_store::extend_samples(
        &store.samples,
        old_w as usize,
        old_h as usize,
        width as usize,
        height as usize,
        dx,
        dy,
    );
    store.width = width;
    store.height = height;
}

/// Top-left of the old canvas within the new one, per anchor.
fn anchor_offset(
    anchor: pictura_ops::Anchor,
    old_w: u32,
    old_h: u32,
    width: u32,
    height: u32,
) -> (i32, i32) {
    use pictura_ops::Anchor;
    let dx = match anchor {
        Anchor::TopLeft | Anchor::MiddleLeft | Anchor::BottomLeft => 0,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => {
            (width as i32 - old_w as i32) / 2
        }
        Anchor::TopRight | Anchor::MiddleRight | Anchor::BottomRight => width as i32 - old_w as i32,
    };
    let dy = match anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => 0,
        Anchor::MiddleLeft | Anchor::Center | Anchor::MiddleRight => {
            (height as i32 - old_h as i32) / 2
        }
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
            height as i32 - old_h as i32
        }
    };
    (dx, dy)
}

pub(crate) fn offset_rect(rect: pictura_core::PsdRect, dx: i32, dy: i32) -> pictura_core::PsdRect {
    pictura_core::PsdRect {
        top: rect.top + dy,
        bottom: rect.bottom + dy,
        left: rect.left + dx,
        right: rect.right + dx,
    }
}

/// Extend one planar `old_w×old_h` channel into `width×height`, blitting the old
/// data at `(dx, dy)` and zero-filling the added area.
pub(crate) fn extend_channel(
    data: &[u8],
    old_w: u32,
    old_h: u32,
    width: u32,
    height: u32,
    dx: i32,
    dy: i32,
) -> Vec<u8> {
    let mut out = vec![0u8; width as usize * height as usize];
    let x0 = dx.max(0);
    let x1 = (old_w as i32 + dx).min(width as i32);
    let y0 = dy.max(0);
    let y1 = (old_h as i32 + dy).min(height as i32);
    for y in y0..y1 {
        let src_row = (y - dy) as usize * old_w as usize;
        let dst_row = y as usize * width as usize;
        for x in x0..x1 {
            let src = src_row + (x - dx) as usize;
            out[dst_row + x as usize] = data.get(src).copied().unwrap_or(0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
        PsdRect, Samples, SourcePlanes,
    };
    use pictura_ops::Anchor;

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
                    data: vec![10; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![20; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![30; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255; n].into(),
                },
            ],
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        }
    }

    fn sample_doc() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![pixel_layer("layer", full(4, 4), None)]);
        doc.channels = vec![Channel {
            id: -1,
            data: (0..16u8).collect(),
        }];
        doc
    }

    #[test]
    fn extend_background_fills_the_new_area_with_the_colour() {
        let mut doc = sample_doc();
        doc.layers[0].background = true;
        resize_canvas_document(&mut doc, 6, 4, Anchor::TopLeft).unwrap();
        assert!(extend_background(&mut doc, [200, 100, 50]));
        let layer = &doc.layers[0];
        assert_eq!(layer.rect, full(6, 4));
        let at = |id: i16, i: usize| layer.channels.iter().find(|c| c.id == id).unwrap().data[i];
        assert_eq!(
            (at(0, 0), at(1, 0), at(2, 0)),
            (10, 20, 30),
            "the old pixels stay"
        );
        assert_eq!(
            (at(0, 4), at(1, 4), at(2, 4), at(-1, 4)),
            (200, 100, 50, 255)
        );
        assert!(
            !extend_background(&mut doc, [0; 3]),
            "already covers the canvas"
        );
        let mut plain = sample_doc();
        resize_canvas_document(&mut plain, 6, 4, Anchor::TopLeft).unwrap();
        assert!(!extend_background(&mut plain, [0; 3]), "no Background");
    }

    #[test]
    fn grow_topleft_keeps_rects_and_pads_channel() {
        let mut doc = sample_doc();
        let before_rect = doc.layers[0].rect;
        resize_canvas_document(&mut doc, 6, 4, Anchor::TopLeft).unwrap();

        assert_eq!(doc.layers[0].rect, before_rect, "TopLeft offset is zero");
        let ch = &doc.channels[0].data;
        assert_eq!(ch.len(), 24);
        assert_eq!(ch[0], 0);
        assert_eq!(ch[3], 3);
        assert_eq!(ch[4], 0, "new column is zero-filled");
        assert_eq!(ch[5], 0);
        assert_eq!(ch[6], 4, "next source row lands at row start");
        assert_eq!(ch[6 + 3], 7);
        assert_eq!(ch[6 + 4], 0);
        assert_eq!(ch[3 * 6 + 3], 15);
        assert_eq!(ch[3 * 6 + 4], 0);
    }

    #[test]
    fn grow_center_offsets_rects_and_blits_centered() {
        let mut doc = sample_doc();
        let before_rect = doc.layers[0].rect;
        resize_canvas_document(&mut doc, 6, 6, Anchor::Center).unwrap();

        assert_eq!(doc.layers[0].rect, offset_rect(before_rect, 1, 1));
        let ch = &doc.channels[0].data;
        assert_eq!(ch.len(), 36);
        assert_eq!(ch[0], 0, "top-left corner is padding");
        assert_eq!(ch[7], 0, "old origin at (1,1)");
        assert_eq!(ch[2 * 6 + 2], 5);
        assert_eq!(ch[4 * 6 + 4], 15);
        assert_eq!(ch[5 * 6 + 5], 0, "bottom-right corner is padding");
        assert_eq!(
            ch.iter().filter(|&&b| b != 0).count(),
            15,
            "source 0 stays 0"
        );
    }

    #[test]
    fn shrink_bottomright_crops_and_clips_channel() {
        let mut doc = sample_doc();
        let before_rect = doc.layers[0].rect;
        resize_canvas_document(&mut doc, 2, 2, Anchor::BottomRight).unwrap();

        assert_eq!(doc.layers[0].rect, offset_rect(before_rect, -2, -2));
        assert_eq!(doc.channels[0].data, vec![10, 11, 14, 15]);
    }

    #[test]
    fn dimensions_update_and_composite_stays_consistent() {
        let mut doc = sample_doc();
        resize_canvas_document(&mut doc, 6, 5, Anchor::MiddleRight).unwrap();

        assert_eq!((doc.width, doc.height), (6, 5));
        assert_eq!(doc.composite, crate::composite_rgba(&doc));
        assert_eq!(doc.composite.width, 6);
        assert_eq!(doc.composite.height, 5);
    }

    #[test]
    fn mask_rect_translates_with_layer() {
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
        resize_canvas_document(&mut doc, 6, 6, Anchor::Center).unwrap();

        assert_eq!(doc.layers[0].rect, rect(1, 1, 5, 5));
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().rect, rect(1, 1, 5, 5));
    }

    #[test]
    fn zero_width_is_invalid_and_unchanged() {
        let mut doc = sample_doc();
        let before = doc.clone();
        let err = resize_canvas_document(&mut doc, 0, 4, Anchor::Center);
        assert!(matches!(err, Err(pictura_ops::OpsError::InvalidParams(_))));
        assert_eq!(doc, before);
    }

    #[test]
    fn one_by_one_does_not_panic() {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![pixel_layer("one", full(1, 1), None)]);
        doc.channels = vec![Channel {
            id: -1,
            data: vec![9].into(),
        }];
        assert!(resize_canvas_document(&mut doc, 2, 2, Anchor::Center).is_ok());
        assert!(resize_canvas_document(&mut doc, 1, 1, Anchor::BottomRight).is_ok());
    }

    #[test]
    fn canvas_rebases_the_retained_store() {
        let mut doc = sample_doc();
        doc.source_depth = Some(BitDepth::Sixteen);
        doc.source_planes = Some(SourcePlanes {
            depth: BitDepth::Sixteen,
            width: 4,
            height: 4,
            samples: Samples::U16((0..16).map(|i| i as u16).collect()),
        });

        resize_canvas_document(&mut doc, 6, 6, Anchor::Center).unwrap();

        let store = doc.source_planes.as_ref().unwrap();
        assert_eq!((store.width, store.height), (6, 6));
        let Samples::U16(v) = &store.samples else {
            panic!("expected a u16 store")
        };
        assert_eq!(v.len(), 36);
        assert_eq!(v[6 + 1], 0, "old origin lands at (1,1)");
        assert_eq!(v[2 * 6 + 2], 5, "old (1,1) lands at (2,2)");
    }
}
