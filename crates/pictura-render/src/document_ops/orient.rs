//! `Image > Image Rotation` and flips at document scope (`IMG-003`).
//!
//! Exact 90/270/180 index remaps of every layer rect and channel plane. The
//! engine leaves `doc.composite` untouched; the app materializes it. Every
//! plane is remapped into spillable tiles a strip at a time (#1294,
//! `geometry_tiled::orient`); the M10 `pictura_ops` pixel-buffer remaps stay
//! as its oracle.

use pictura_core::memory::MemoryError;
#[cfg(test)]
use pictura_core::PixelBuffer;
use pictura_core::{Document, Layer, PsdRect, Samples, SourceChannels};
use pictura_ops::OpsError;

use super::{for_each_layer, geometry_tiled, native_store};

/// The five exact orientation remaps.
#[derive(Clone, Copy)]
enum Kind {
    Rot90,
    Rot180,
    Rot270,
    FlipH,
    FlipV,
}

impl Kind {
    /// 90°/270° swap the two canvas axes.
    fn swaps_axes(self) -> bool {
        matches!(self, Kind::Rot90 | Kind::Rot270)
    }

    fn remap(self) -> native_store::Remap {
        match self {
            Kind::Rot90 => native_store::Remap::Rot90,
            Kind::Rot180 => native_store::Remap::Rot180,
            Kind::Rot270 => native_store::Remap::Rot270,
            Kind::FlipH => native_store::Remap::FlipH,
            Kind::FlipV => native_store::Remap::FlipV,
        }
    }
}

/// Rotate the whole document by 1 (90° CW), 2 (180°) or 3 (270° CW) quarter
/// turns. Any other value is `InvalidParams` and leaves the document untouched.
pub fn rotate_document(doc: &mut Document, quarter_turns: u8) -> Result<(), OpsError> {
    let kind = match quarter_turns {
        1 => Kind::Rot90,
        2 => Kind::Rot180,
        3 => Kind::Rot270,
        _ => {
            return Err(OpsError::InvalidParams(
                "quarter_turns must be 1, 2, or 3".into(),
            ))
        }
    };
    transform_document(doc, kind)
}

/// Mirror the whole document along the vertical axis (`horizontal`) or the
/// horizontal axis. A refused strip (`OpsError::Memory`) leaves it unchanged.
pub fn flip_document(doc: &mut Document, horizontal: bool) -> Result<(), OpsError> {
    transform_document(doc, if horizontal { Kind::FlipH } else { Kind::FlipV })
}

/// Remap a raw planar channel through the same pixel-buffer function used for
/// the M10 per-image ops: the dense remap, kept as the tiled remap's oracle.
#[cfg(test)]
pub(super) fn remap_plane(
    data: &[u8],
    lw: u32,
    lh: u32,
    op: fn(&PixelBuffer) -> PixelBuffer,
) -> Vec<u8> {
    let mut plane = PixelBuffer::new(lw, lh, 1);
    let n = plane.data.len().min(data.len());
    plane.data[..n].copy_from_slice(&data[..n]);
    op(&plane).data.to_vec()
}

/// Transform a rect in a `w×h` canvas. `m = ` the corresponding plane remap.
fn transform_rect(r: PsdRect, w: i32, h: i32, kind: Kind) -> PsdRect {
    match kind {
        Kind::Rot90 => PsdRect {
            top: r.left,
            left: h - r.bottom,
            bottom: r.right,
            right: h - r.top,
        },
        Kind::Rot180 => PsdRect {
            top: h - r.bottom,
            left: w - r.right,
            bottom: h - r.top,
            right: w - r.left,
        },
        Kind::Rot270 => PsdRect {
            top: w - r.right,
            left: r.top,
            bottom: w - r.left,
            right: r.bottom,
        },
        Kind::FlipH => PsdRect {
            top: r.top,
            left: w - r.right,
            bottom: r.bottom,
            right: w - r.left,
        },
        Kind::FlipV => PsdRect {
            top: h - r.bottom,
            left: r.left,
            bottom: h - r.top,
            right: r.right,
        },
    }
}

/// What [`rotate_document`] by `quarter_turns` (or a flip, at 0) writes
/// (#1294, the app's memory gate): every plane at its remapped size and sample
/// width, and the widest plane in samples.
pub fn oriented_plane_bytes(doc: &Document, quarter_turns: u8) -> (u64, u64) {
    let swap = quarter_turns % 2 == 1;
    let turn = |(w, h): (u32, u32)| if swap { (h, w) } else { (w, h) };
    super::resize::mapped_plane_bytes(doc, turn((doc.width, doc.height)), |rect| {
        let (w, h) = dims(rect);
        (w * h > 0).then(|| turn((w as u32, h as u32)))
    })
}

/// Remap every plane on a copy that shares the document's planes, so a strip
/// refusal leaves the document unchanged (#1294).
fn transform_document(doc: &mut Document, kind: Kind) -> Result<(), OpsError> {
    let mut next = doc.clone();
    super::materialize_merged_composite(&mut next);
    let (w, h) = (next.width as i32, next.height as i32);
    if let Some(job) = pictura_core::job::current() {
        job.begin_stage(orient_rows(&mut next, kind));
    }

    let mut refused = None;
    for_each_layer(&mut next.layers, &mut |layer| {
        if refused.is_none() {
            refused = orient_layer(layer, w, h, kind).err();
        }
    });
    if let Some(err) = refused {
        return Err(err.into());
    }

    let canvas = (w as usize, h as usize);
    for channel in &mut next.channels {
        channel.data = geometry_tiled::orient(&channel.data, canvas, 1, kind.remap())?;
    }
    if let Some(store) = &mut next.source_planes {
        if store.depth != pictura_core::BitDepth::One {
            store.samples = orient_samples(&store.samples, canvas, kind.remap())?;
            if kind.swaps_axes() {
                std::mem::swap(&mut store.width, &mut store.height);
            }
        }
    }

    if kind.swaps_axes() {
        std::mem::swap(&mut next.width, &mut next.height);
    }
    *doc = next;
    Ok(())
}

fn orient_layer(layer: &mut Layer, w: i32, h: i32, kind: Kind) -> Result<(), MemoryError> {
    let old_rect = layer.rect;
    for channel in &mut layer.channels {
        channel.data = geometry_tiled::orient(&channel.data, dims(old_rect), 1, kind.remap())?;
    }
    if let Some(store) = layer.source_channels.take() {
        let mask_rect = layer.mask.as_ref().map(|m| m.rect);
        layer.source_channels = Some(remap_store(&store, old_rect, mask_rect, w, h, kind)?);
    }
    layer.rect = transform_rect(old_rect, w, h, kind);
    // ponytail: the exact remap keeps the native store, but an unmodeled raw
    // on-disk stream still has no resampler and drops.
    layer.raw_channels.clear();

    if let Some(mask) = &mut layer.mask {
        if let Some(data) = &mut mask.data {
            *data = geometry_tiled::orient(data, dims(mask.rect), 1, kind.remap())?;
        }
        mask.rect = transform_rect(mask.rect, w, h, kind);
    }
    Ok(())
}

fn dims(rect: PsdRect) -> (usize, usize) {
    (rect.width().max(0) as usize, rect.height().max(0) as usize)
}

/// Every stacked `w×h` plane of a retained store through the tiled remap; a
/// trailing partial plane is dropped, as the dense remap drops it.
fn orient_samples(
    samples: &Samples,
    (w, h): (usize, usize),
    kind: native_store::Remap,
) -> Result<Samples, MemoryError> {
    let channels = samples.len() / (w * h).max(1);
    Ok(match samples {
        Samples::U8(p) => Samples::U8(geometry_tiled::orient(p, (w, h), channels, kind)?),
        Samples::U16(p) => Samples::U16(geometry_tiled::orient(p, (w, h), channels, kind)?),
        Samples::F32(p) => Samples::F32(geometry_tiled::orient(p, (w, h), channels, kind)?),
    })
}

/// The output rows [`transform_document`] writes, which the tiled remap
/// reports as a job's progress.
fn orient_rows(doc: &mut Document, kind: Kind) -> u64 {
    let rows = |rect: PsdRect, planes: usize| {
        let (w, h) = dims(rect);
        let out = if kind.swaps_axes() { w } else { h };
        if w * h == 0 {
            0
        } else {
            planes as u64 * out as u64
        }
    };
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let mut total = rows(canvas, doc.channels.len());
    if let Some(store) = doc
        .source_planes
        .as_ref()
        .filter(|s| s.depth != pictura_core::BitDepth::One)
    {
        let area = (doc.width as usize * doc.height as usize).max(1);
        total += rows(canvas, store.samples.len() / area);
    }
    for_each_layer(&mut doc.layers, &mut |layer| {
        total += rows(layer.rect, layer.channels.len());
        if let Some(mask) = layer.mask.as_ref().filter(|m| m.data.is_some()) {
            total += rows(mask.rect, 1);
        }
        if let Some(store) = &layer.source_channels {
            for (id, samples) in &store.planes {
                let rect = match (&layer.mask, *id) {
                    (Some(mask), -2) => mask.rect,
                    _ => layer.rect,
                };
                let area = dims(rect).0 * dims(rect).1;
                total += rows(rect, samples.len() / area.max(1));
            }
        }
    });
    total
}

/// Remap a layer's retained native store with the same exact index map; the
/// `-2` mask plane follows the mask rect (which may differ from the layer rect).
fn remap_store(
    store: &SourceChannels,
    layer_rect: PsdRect,
    mask_rect: Option<PsdRect>,
    w: i32,
    h: i32,
    kind: Kind,
) -> Result<SourceChannels, MemoryError> {
    let planes = store
        .planes
        .iter()
        .map(|(id, samples)| {
            let rect = if *id == -2 {
                mask_rect.unwrap_or(layer_rect)
            } else {
                layer_rect
            };
            Ok((*id, orient_samples(samples, dims(rect), kind.remap())?))
        })
        .collect::<Result<_, MemoryError>>()?;
    Ok(SourceChannels::new(
        store.depth,
        transform_rect(layer_rect, w, h, kind),
        planes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Layer, LayerMask, LockFlags, Samples,
        SourceChannels, SourcePlanes,
    };

    fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    fn plane(base: u8, n: usize) -> Vec<u8> {
        (0..n).map(|i| base.wrapping_add(i as u8)).collect()
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
                    data: plane(0, n).into(),
                },
                Channel {
                    id: 1,
                    data: plane(60, n).into(),
                },
                Channel {
                    id: 2,
                    data: plane(120, n).into(),
                },
                Channel {
                    id: -1,
                    data: plane(200, n).into(),
                },
            ],
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        }
    }

    /// Non-square 5×3 document with a full layer, a masked layer and a
    /// document-level channel. `composite` is canonical so full-doc equality is
    /// a valid identity check.
    fn sample_doc() -> Document {
        let mut doc = Document::new(5, 3, ColorMode::Rgb, BitDepth::Eight);
        let mask = LayerMask {
            rect: rect(0, 2, 1, 5),
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(vec![1, 2, 3].into()),
            ..Default::default()
        };
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("full", rect(0, 0, 3, 5), None),
            pixel_layer("partial", rect(1, 1, 2, 4), Some(mask)),
        ]);
        doc.channels = vec![Channel {
            id: -1,
            data: plane(0, 15).into(),
        }];
        doc.composite = crate::composite_rgba(&doc);
        doc
    }

    fn assert_consistent(doc: &mut Document) {
        doc.composite = crate::composite_rgba(doc);
        assert_eq!(doc.composite, crate::composite_rgba(doc));
    }

    #[test]
    fn rotate90_cw_swaps_dimensions_and_plane_dims() {
        let mut doc = sample_doc();
        rotate_document(&mut doc, 1).unwrap();

        assert_eq!((doc.width, doc.height), (3, 5));
        for layer in &doc.layers {
            let area = (layer.rect.width() * layer.rect.height()) as usize;
            for channel in &layer.channels {
                assert_eq!(channel.data.len(), area, "{} {}", layer.name, channel.id);
            }
        }
        assert_eq!(doc.layers[0].rect, rect(0, 0, 5, 3));
        assert_eq!(doc.layers[1].rect, rect(1, 1, 4, 2));
        assert_eq!(doc.channels[0].data.len(), 15);
        assert_consistent(&mut doc);
    }

    #[test]
    fn rotate90_cw_remaps_channel_bytes_exactly() {
        let mut doc = Document::new(3, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![pixel_layer(
            "only",
            rect(0, 0, 2, 3),
            None,
        )]);
        doc.composite = crate::composite_rgba(&doc);

        rotate_document(&mut doc, 1).unwrap();

        assert_eq!((doc.width, doc.height), (2, 3));
        assert_eq!(doc.layers[0].channels[0].data, vec![3, 0, 4, 1, 5, 2]);
        assert_consistent(&mut doc);
    }

    #[test]
    fn four_quarter_turns_restore_document() {
        let mut doc = sample_doc();
        let before = doc.clone();
        for _ in 0..4 {
            rotate_document(&mut doc, 1).unwrap();
        }
        assert_eq!(doc, before);
    }

    #[test]
    fn cw_then_ccw_is_identity() {
        let mut doc = sample_doc();
        let before = doc.clone();
        rotate_document(&mut doc, 1).unwrap();
        rotate_document(&mut doc, 3).unwrap();
        assert_eq!(doc, before);
    }

    #[test]
    fn invalid_quarter_turns_leave_document_unchanged() {
        for q in [0u8, 4, 5, 255] {
            let mut doc = sample_doc();
            let before = doc.clone();
            let err = rotate_document(&mut doc, q);
            assert!(matches!(err, Err(OpsError::InvalidParams(_))), "q={q}");
            assert_eq!(doc, before, "q={q}");
        }
    }

    #[test]
    fn flip_horizontal_mirrors_channel_bytes() {
        let mut doc = sample_doc();
        assert_eq!(doc.layers[0].channels[0].data, plane(0, 15));

        flip_document(&mut doc, true).unwrap();

        let want: Vec<u8> = vec![4, 3, 2, 1, 0, 9, 8, 7, 6, 5, 14, 13, 12, 11, 10];
        assert_eq!(doc.layers[0].channels[0].data, want);
        assert_eq!(doc.layers[0].rect, rect(0, 0, 3, 5));
        assert_consistent(&mut doc);
    }

    #[test]
    fn flip_twice_is_identity() {
        for horizontal in [true, false] {
            let mut doc = sample_doc();
            let before = doc.clone();
            flip_document(&mut doc, horizontal).unwrap();
            flip_document(&mut doc, horizontal).unwrap();
            assert_eq!(doc, before, "horizontal={horizontal}");
        }
    }

    #[test]
    fn masks_transform_with_layer() {
        let mut doc = sample_doc();
        rotate_document(&mut doc, 1).unwrap();
        let mask = doc.layers[1].mask.as_ref().unwrap();
        assert_eq!(mask.rect, rect(2, 2, 5, 3));
        assert_eq!(mask.data.as_ref().unwrap().len(), 3);
        assert_consistent(&mut doc);

        let mut doc = sample_doc();
        flip_document(&mut doc, true).unwrap();
        let mask = doc.layers[1].mask.as_ref().unwrap();
        assert_eq!(mask.rect, rect(0, 0, 1, 3));
        assert_eq!(mask.data.as_ref().unwrap().len(), 3);
        assert_consistent(&mut doc);
    }

    #[test]
    fn depth16_orientation_remaps_the_native_store_and_saves() {
        let mut doc = Document::new(3, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.source_depth = Some(BitDepth::Sixteen);
        doc.source_planes = Some(SourcePlanes {
            depth: BitDepth::Sixteen,
            width: 3,
            height: 2,
            samples: Samples::U16((0..6 * 3).map(|i| i as u16 * 257 + 5).collect()),
        });
        let mut layer = pixel_layer("only", rect(0, 0, 2, 3), None);
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            layer.rect,
            vec![(
                0,
                Samples::U16((0..6).map(|i| i as u16 * 257 + 5).collect()),
            )],
        ));
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        doc.composite = crate::composite_rgba(&doc);

        rotate_document(&mut doc, 1).unwrap();

        assert_eq!((doc.width, doc.height), (2, 3));
        let store = doc.layers[0].source_channels.as_ref().unwrap();
        assert_eq!(store.rect, doc.layers[0].rect);
        let (_, samples) = store.planes.iter().find(|(id, _)| *id == 0).unwrap();
        let want: Vec<u16> = [3, 0, 4, 1, 5, 2]
            .iter()
            .map(|v| *v as u16 * 257 + 5)
            .collect();
        assert_eq!(samples, &Samples::U16(want.into()), "exact 90° index remap");
        let planes = doc.source_planes.as_ref().unwrap();
        assert_eq!((planes.width, planes.height), (2, 3));
        doc.composite = crate::composite_rgba(&doc);
        let bytes = pictura_codec::write_psd(&doc).expect("a rotated high-depth doc saves");
        assert_eq!(u16::from_be_bytes(bytes[22..24].try_into().unwrap()), 16);
    }
}
