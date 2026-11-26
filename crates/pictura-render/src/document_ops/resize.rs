//! `Image > Image Size` at document scope (`IMG-001`). Every plane is
//! resampled into spillable tiles a strip at a time (#1293, `resize_tiled`).

use pictura_core::memory::MemoryError;
use pictura_core::{Plane, Samples};

use super::layer_ops::scale_layer_style;
use super::resize_tiled::{self, Kernel};
use super::{for_each_layer, materialize_merged_composite, valid_size};

/// Scale every layer rect, resample its channel planes and mask from the old
/// rect size to the new. The engine leaves `doc.composite` untouched; the
/// app edit transaction materializes it from the resampled tree. The resize
/// runs on a copy that shares the document's planes, so a refusal leaves the
/// document unchanged.
pub fn resize_document(
    doc: &mut pictura_core::Document,
    width: u32,
    height: u32,
    resample: pictura_ops::Resample,
) -> Result<(), pictura_ops::OpsError> {
    resize_document_with_styles(doc, width, height, resample, false)
}

/// Resize the document and optionally scale stored effects as part of the same edit.
pub fn resize_document_with_styles(
    doc: &mut pictura_core::Document,
    width: u32,
    height: u32,
    resample: pictura_ops::Resample,
    scale_styles: bool,
) -> Result<(), pictura_ops::OpsError> {
    valid_size(width, height)?;
    let old_w = doc.width;
    let old_h = doc.height;
    if old_w == 0 || old_h == 0 {
        return Err(pictura_ops::OpsError::InvalidParams(
            "document has a zero dimension".into(),
        ));
    }
    let sx = width as f64 / old_w as f64;
    let sy = height as f64 / old_h as f64;
    let mut next = doc.clone();
    let was_layerless = next.layers.is_empty();
    materialize_merged_composite(&mut next);
    if let Some(job) = pictura_core::job::current() {
        job.begin_stage(resampled_rows(&mut next, sx, sy, (width, height)));
    }

    let style_factor = if scale_styles {
        style_scale(old_w, old_h, width, height)
    } else {
        None
    };
    let mut refused = None;
    for_each_layer(&mut next.layers, &mut |layer| {
        if refused.is_none() {
            refused = resize_layer(layer, sx, sy, resample).err();
            if let Some(factor) = style_factor.filter(|_| refused.is_none()) {
                scale_layer_style(layer, factor, true);
            }
        }
    });
    if let Some(err) = refused {
        return Err(err.into());
    }

    let (from, to) = (
        (old_w as usize, old_h as usize),
        (width as usize, height as usize),
    );
    for channel in &mut next.channels {
        channel.data = resize_bytes(&channel.data, from, to, resample)?;
    }
    if let Some(mut store) = next.source.planes().clone() {
        if store.depth != pictura_core::BitDepth::One {
            store.samples = resize_native(&store.samples, from, to, resample)?;
            store.width = width;
            store.height = height;
        }

        next.source
            .replace_planes(Some(store))
            .expect("valid source planes");
    }

    if was_layerless && !next.layers.is_empty() {
        attach_resized_merged_native(&mut next, width, height)?;
    }
    next.width = width;
    next.height = height;
    if (width, height) != (old_w, old_h) {
        next.measurement_scale.invalidate_for_resample();
    }
    super::refresh_plate_preview(&mut next);
    *doc = next;
    Ok(())
}

// The synthetic layer must retain the resized native samples for compatible saves.
fn attach_resized_merged_native(
    doc: &mut pictura_core::Document,
    width: u32,
    height: u32,
) -> Result<(), MemoryError> {
    use pictura_core::{BitDepth, ColorMode, SourceChannels, TileSample};
    if doc.source.mode().is_some() || !matches!(doc.mode, ColorMode::Rgb | ColorMode::Grayscale) {
        return Ok(());
    }
    let Some(store) = doc.source.planes().as_ref() else { return Ok(()) };
    if !matches!(store.depth, BitDepth::Sixteen | BitDepth::ThirtyTwo)
        || (store.width, store.height) != (width, height)
    {
        return Ok(());
    }
    let (w, h) = (width as usize, height as usize);
    let area = w * h;
    let colors = doc.mode.color_channels() as usize;
    if store.samples.len() < area * colors { return Ok(()); }
    let mut ids: Vec<(i16, usize)> = (0..colors).map(|i| (i as i16, i)).collect();
    if doc.mode == ColorMode::Rgb && doc.composite.signed_merged_alpha()
        && store.samples.len() >= area * (colors + 1)
    {
        ids.push((-1, colors));
    }
    fn copy_plane<T: TileSample>(src: &Plane<T>, w: usize, h: usize, index: usize)
        -> Result<Plane<T>, MemoryError>
    {
        Plane::build_tiled(w, h, 1, |y, row| {
            src.read_into(index * w * h + y * w, row);
            Ok(())
        })
    }
    let mut planes = Vec::with_capacity(ids.len());
    for (id, index) in ids {
        let samples = match &store.samples {
            Samples::U16(src) => Samples::U16(copy_plane(src, w, h, index)?),
            Samples::F32(src) => Samples::F32(copy_plane(src, w, h, index)?),
            Samples::U8(_) => return Ok(()),
        };
        planes.push((id, samples));
    }
    let layer = &mut doc.layers[0];
    layer.source_channels = Some(SourceChannels::new(store.depth, layer.rect, planes));
    Ok(())
}

// A linked target may round to different apparent scale factors per axis.
fn style_scale(old_w: u32, old_h: u32, width: u32, height: u32) -> Option<f64> {
    if (old_w, old_h) == (width, height) {
        return None;
    }
    let scale_bounds = |src: u32, dst: u32| {
        let lo = if dst == 1 { 0.0 } else { f64::from(dst) - 0.5 };
        (lo / f64::from(src), (f64::from(dst) + 0.5) / f64::from(src))
    };
    let (x_lo, x_hi) = scale_bounds(old_w, width);
    let (y_lo, y_hi) = scale_bounds(old_h, height);
    if x_lo.max(y_lo) >= x_hi.min(y_hi) {
        return None;
    }
    Some(if old_w >= old_h {
        f64::from(width) / f64::from(old_w)
    } else {
        f64::from(height) / f64::from(old_h)
    })
}

/// An 8-bit plane through the tiled resampler with `pictura_ops::resize`'s
/// rounding.
fn resize_bytes(
    plane: &Plane<u8>,
    from: (usize, usize),
    to: (usize, usize),
    resample: pictura_ops::Resample,
) -> Result<Plane<u8>, MemoryError> {
    resize_tiled::resize(plane, from, 1, to, Kernel::bytes(resample))
}

/// Every stacked `from`-sized plane of a retained store through the tiled
/// resampler at native precision; a trailing partial plane is dropped, as the
/// dense resampler drops it.
fn resize_native(
    samples: &Samples,
    from: (usize, usize),
    to: (usize, usize),
    resample: pictura_ops::Resample,
) -> Result<Samples, MemoryError> {
    let area = from.0 * from.1;
    if area == 0 && from != to {
        return Ok(match samples {
            Samples::U8(_) => Samples::U8(Plane::default()),
            Samples::U16(_) => Samples::U16(Plane::default()),
            Samples::F32(_) => Samples::F32(Plane::default()),
        });
    }
    let channels = samples.len() / area.max(1);
    Ok(match samples {
        Samples::U8(p) => Samples::U8(resize_tiled::resize(
            p,
            from,
            channels,
            to,
            Kernel::unit(resample),
        )?),
        Samples::U16(p) => Samples::U16(resize_tiled::resize(
            p,
            from,
            channels,
            to,
            Kernel::unit(resample),
        )?),
        Samples::F32(p) => Samples::F32(resize_tiled::resize(
            p,
            from,
            channels,
            to,
            Kernel::float(resample),
        )?),
    })
}

/// The output rows [`resize_document`] resamples, which the tiled resampler
/// reports as a job's progress: every plane whose size changes.
fn resampled_rows(doc: &mut pictura_core::Document, sx: f64, sy: f64, to: (u32, u32)) -> u64 {
    let resized = (doc.width, doc.height) != to;
    let mut rows = 0;
    if resized {
        rows += doc.channels.len() as u64 * u64::from(to.1);
        if let Some(store) = doc
            .source
            .planes()
            .as_ref()
            .filter(|s| s.depth != pictura_core::BitDepth::One)
        {
            let plane = doc.width as usize * doc.height as usize;
            rows += (store.samples.len() / plane.max(1)) as u64 * u64::from(to.1);
        }
    }
    let mut count = |rect: pictura_core::PsdRect, planes: u64| {
        let (w, h) = (rect.width(), rect.height());
        if w > 0 && h > 0 {
            let (nw, nh) = scaled_dims(w, h, sx, sy);
            if (nw, nh) != (w as u32, h as u32) {
                rows += planes * u64::from(nh);
            }
        }
    };
    for_each_layer(&mut doc.layers, &mut |layer| {
        count(layer.rect, layer.channels().len() as u64);
        let mask = layer
            .mask
            .as_ref()
            .filter(|m| m.data.is_some())
            .map(|m| m.rect);
        if let Some(rect) = mask {
            count(rect, 1);
        }
        if sx != 1.0 || sy != 1.0 {
            if let Some(store) = &layer.source_channels {
                for (id, samples) in &store.planes {
                    let rect = if *id == -2 {
                        layer.mask.as_ref().map_or(layer.rect, |m| m.rect)
                    } else {
                        layer.rect
                    };
                    let area = (rect.width().max(0) * rect.height().max(0)) as usize;
                    count(rect, (samples.len() / area.max(1)) as u64);
                }
            }
        }
    });
    rows
}

/// What [`resize_document`] to `width × height` writes (#1293, the app's
/// memory gate): the bytes of every plane it resamples, at its new size and
/// sample width, and the widest of those planes in samples.
pub fn resized_plane_bytes(doc: &pictura_core::Document, width: u32, height: u32) -> (u64, u64) {
    let sx = width as f64 / f64::from(doc.width.max(1));
    let sy = height as f64 / f64::from(doc.height.max(1));
    let (mut bytes, mut widest) = mapped_plane_bytes(doc, (width, height), |r| {
        (r.width() > 0 && r.height() > 0).then(|| scaled_dims(r.width(), r.height(), sx, sy))
    });
    if doc.layers.is_empty() && doc.composite.merged_output() {
        let area = u64::from(width) * u64::from(height);
        let colors = doc.mode.color_channels() as u64;
        bytes += area * (colors + 1);
        if doc.source.mode().is_none() && matches!(doc.mode, pictura_core::ColorMode::Rgb | pictura_core::ColorMode::Grayscale) {
            if let Some(store) = doc.source.planes().as_ref().filter(|s| matches!(s.depth, pictura_core::BitDepth::Sixteen | pictura_core::BitDepth::ThirtyTwo)) {
                let alpha = u64::from(doc.mode == pictura_core::ColorMode::Rgb && doc.composite.signed_merged_alpha());
                bytes += area * (colors + alpha) * sample_bytes(&store.samples);
            }
        }
        widest = widest.max(u64::from(width));
    }
    (bytes, widest)
}

/// The bytes a whole-document geometry command writes (#1293, #1294): every
/// alpha channel and retained document-store plane at `canvas`, and every
/// layer channel, mask and layer-store plane at `layer_dims` of its rect
/// (`None` when it is not rewritten), each at its sample width; and the widest
/// plane in samples.
pub(crate) fn mapped_plane_bytes(
    doc: &pictura_core::Document,
    canvas: (u32, u32),
    layer_dims: impl Fn(pictura_core::PsdRect) -> Option<(u32, u32)>,
) -> (u64, u64) {
    let (mut bytes, mut widest) = (0u64, 0u64);
    let mut add = |(w, h): (u32, u32), planes: usize, sample: u64| {
        bytes += planes as u64 * u64::from(w) * u64::from(h) * sample;
        widest = widest.max(u64::from(w));
    };
    add(canvas, doc.channels.len(), 1);
    if let Some(store) = doc
        .source
        .planes()
        .as_ref()
        .filter(|s| s.depth != pictura_core::BitDepth::One)
    {
        let plane = doc.width as usize * doc.height as usize;
        let planes = store.samples.len() / plane.max(1);
        add(canvas, planes, sample_bytes(&store.samples));
    }
    walk(&doc.layers, &mut |layer| {
        if let Some(to) = layer_dims(layer.rect) {
            add(to, layer.channels().len(), 1);
        }
        let mask = layer.mask.as_ref().filter(|m| m.data.is_some());
        if let Some(to) = mask.and_then(|m| layer_dims(m.rect)) {
            add(to, 1, 1);
        }
        let stores = layer.source_channels.iter().flat_map(|s| &s.planes);
        for (id, samples) in stores {
            let rect = match (&layer.mask, *id) {
                (Some(mask), -2) => mask.rect,
                _ => layer.rect,
            };
            if let Some(to) = layer_dims(rect) {
                let area = (rect.width().max(0) * rect.height().max(0)) as usize;
                add(to, samples.len() / area.max(1), sample_bytes(samples));
            }
        }
    });
    (bytes, widest)
}

fn walk(layers: &pictura_core::LayerStack, f: &mut dyn FnMut(&pictura_core::Layer)) {
    for layer in layers.iter() {
        f(layer);
        walk(layer.children(), f);
    }
}

fn sample_bytes(samples: &Samples) -> u64 {
    match samples {
        Samples::U8(_) => 1,
        Samples::U16(_) => 2,
        Samples::F32(_) => 4,
    }
}

/// New dimensions for an old `w×h` rect under scale factors, floored at 1.
fn scaled_dims(w: i32, h: i32, sx: f64, sy: f64) -> (u32, u32) {
    let nw = ((w as f64 * sx).round() as i64).max(1) as u32;
    let nh = ((h as f64 * sy).round() as i64).max(1) as u32;
    (nw, nh)
}

/// Rescale a rect's origin, then size it from the resampled dimensions.
fn scaled_rect(
    rect: pictura_core::PsdRect,
    nw: u32,
    nh: u32,
    sx: f64,
    sy: f64,
) -> pictura_core::PsdRect {
    let top = (rect.top as f64 * sy).round() as i32;
    let left = (rect.left as f64 * sx).round() as i32;
    pictura_core::PsdRect {
        top,
        left,
        bottom: top + nh as i32,
        right: left + nw as i32,
    }
}

fn resize_layer(
    layer: &mut pictura_core::Layer,
    sx: f64,
    sy: f64,
    resample: pictura_ops::Resample,
) -> Result<(), MemoryError> {
    let old_rect = layer.rect;
    let old_mask = layer.mask.as_ref().map(|m| m.rect);
    let lw = old_rect.width();
    let lh = old_rect.height();
    if lw > 0 && lh > 0 {
        let (nw, nh) = scaled_dims(lw, lh, sx, sy);
        let (from, to) = ((lw as usize, lh as usize), (nw as usize, nh as usize));
        for channel in layer.channels_mut() {
            channel.data = resize_bytes(&channel.data, from, to, resample)?;
        }
        layer.rect = scaled_rect(old_rect, nw, nh, sx, sy);
    }
    if let Some(mask) = &mut layer.mask {
        let mw = mask.rect.width();
        let mh = mask.rect.height();
        if mw > 0 && mh > 0 {
            if let Some(data) = &mut mask.data {
                let (nw, nh) = scaled_dims(mw, mh, sx, sy);
                let (from, to) = ((mw as usize, mh as usize), (nw as usize, nh as usize));
                *data = resize_bytes(data, from, to, resample)?;
                mask.rect = scaled_rect(mask.rect, nw, nh, sx, sy);
            }
        }
    }
    if sx != 1.0 || sy != 1.0 {
        // ponytail: an unmodeled raw on-disk stream still has no resampler.
        layer.discard_raw_channels();
        if let Some(store) = layer.source_channels.take() {
            layer.source_channels =
                Some(resize_store(store, old_rect, old_mask, sx, sy, resample)?);
        }
    }
    Ok(())
}

/// Resample a layer's retained native store to the scaled bounds; the `-2`
/// mask plane follows the mask's rect, `mask_rect` before the resize.
fn resize_store(
    store: pictura_core::SourceChannels,
    old_rect: pictura_core::PsdRect,
    mask_rect: Option<pictura_core::PsdRect>,
    sx: f64,
    sy: f64,
    resample: pictura_ops::Resample,
) -> Result<pictura_core::SourceChannels, MemoryError> {
    let (lw, lh) = (old_rect.width(), old_rect.height());
    let (nw, nh) = if lw > 0 && lh > 0 {
        scaled_dims(lw, lh, sx, sy)
    } else {
        (0, 0)
    };
    let planes = store
        .planes
        .iter()
        .map(|(id, samples)| {
            let (ow, oh, dw, dh) = if *id == -2 {
                match mask_rect {
                    Some(m) => {
                        let (mw, mh) = (m.width(), m.height());
                        let (nw, nh) = if mw > 0 && mh > 0 {
                            scaled_dims(mw, mh, sx, sy)
                        } else {
                            (0, 0)
                        };
                        (mw, mh, nw, nh)
                    }
                    None => (lw, lh, nw, nh),
                }
            } else {
                (lw, lh, nw, nh)
            };
            let from = (ow.max(0) as usize, oh.max(0) as usize);
            let resized = resize_native(samples, from, (dw as usize, dh as usize), resample)?;
            Ok((*id, resized))
        })
        .collect::<Result<_, MemoryError>>()?;
    Ok(pictura_core::SourceChannels::new(
        store.depth,
        scaled_rect(old_rect, nw, nh, sx, sy),
        planes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
        PsdRect, RawChannel, Samples, SourceChannels, SourcePlanes,
    };

    #[test]
    fn resize_style_rounding_intervals() {
        assert_eq!(style_scale(3, 2, 5, 3), Some(5.0 / 3.0));
        assert_eq!(style_scale(30, 2, 2, 1), Some(2.0 / 30.0));
        assert_eq!(style_scale(3, 2, 3, 2), None);
        assert_eq!(style_scale(3, 2, 5, 6), None);
        assert_eq!(style_scale(3, 2, 11, 9), None);
        assert_eq!(style_scale(3, 2, 11, 7), Some(11.0 / 3.0));
    }

    #[test]
    fn image_size_scales_legacy_effects_without_rewriting_unknown_records() {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("Legacy", rect(0, 0, 4, 4), None);
        let mut shadow = Vec::new();
        for value in [0u32, 8, 25, 45, 12] { shadow.extend_from_slice(&value.to_be_bytes()); }
        shadow.extend_from_slice(&[0; 10]);
        shadow.extend_from_slice(b"8BIMnorm");
        shadow.extend_from_slice(&[0, 0, 255]);
        let mut legacy = vec![0, 0, 0, 2];
        legacy.extend_from_slice(b"8BIMdsdw");
        legacy.extend_from_slice(&(shadow.len() as u32).to_be_bytes());
        legacy.extend_from_slice(&shadow);
        legacy.extend_from_slice(b"8BIMzzzz");
        legacy.extend_from_slice(&3u32.to_be_bytes());
        legacy.extend_from_slice(&[7, 8, 9]);
        assert!(layer.set_metadata_block(*b"lrFX", Some(legacy.clone().into())));
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        let before = doc.clone();
        resize_document_with_styles(&mut doc, 8, 8, pictura_ops::Resample::Nearest, true).unwrap();
        let result = doc.layers[0].extra_block(b"lrFX").unwrap().data.to_vec();
        let mut expected = legacy.clone();
        expected[20..24].copy_from_slice(&16u32.to_be_bytes());
        expected[32..36].copy_from_slice(&24u32.to_be_bytes());
        assert_eq!(result, expected);
        assert_eq!(before.layers[0].extra_block(b"lrFX").unwrap().data.to_vec(), legacy);
        let mut unchanged = before;
        resize_document_with_styles(&mut unchanged, 8, 8, pictura_ops::Resample::Nearest, false).unwrap();
        assert_eq!(unchanged.layers[0].extra_block(b"lrFX").unwrap().data.to_vec(), legacy);
    }

    #[test]
    fn resize_scales_layer_styles_only_when_requested() {
        use super::super::layer_ops::{layer_style_value, set_layer_style_value};
        let mut original = Document::new(3, 2, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("Styled", rect(0, 0, 2, 3), None);
        assert!(set_layer_style_value(&mut layer, "dropShadow.on", 1.0));
        assert!(set_layer_style_value(&mut layer, "dropShadow.size", 9.0));
        assert!(set_layer_style_value(&mut layer, "innerShadow.size", 6.0));
        original.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        for (scale, outer, inner) in [(false, 9.0, 6.0), (true, 15.0, 10.0)] {
            let mut doc = original.clone();
            resize_document_with_styles(&mut doc, 5, 3, pictura_ops::Resample::Nearest, scale)
                .unwrap();
            assert_eq!(layer_style_value(&doc.layers[0], "innerShadow.on"), Some(0.0));
            assert!(set_layer_style_value(&mut doc.layers[0], "innerShadow.on", 1.0));
            assert_eq!(layer_style_value(&doc.layers[0], "dropShadow.size"), Some(outer));
            assert_eq!(layer_style_value(&doc.layers[0], "innerShadow.size"), Some(inner));
        }
    }

    fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    fn pixel_layer(name: &str, r: PsdRect, mask: Option<LayerMask>) -> Layer {
        let n = (r.width().max(0) * r.height().max(0)) as usize;
        pictura_codec::layer_from_parts(pictura_core::LayerParts {
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
        })
    }

    fn sample_doc() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        let mask = LayerMask {
            rect: rect(0, 0, 4, 4),
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(vec![128; 16].into()),
            ..Default::default()
        };
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("full", rect(0, 0, 4, 4), None),
            pixel_layer("masked", rect(1, 1, 3, 3), Some(mask)),
        ]);
        doc.channels = vec![Channel {
            id: -1,
            data: vec![7; 16].into(),
        }];
        doc
    }

    #[test]
    fn resize_2x_scales_document_layers_mask_and_channel() {
        let mut doc = sample_doc();
        let before = doc.clone();
        resize_document(&mut doc, 8, 8, pictura_ops::Resample::Nearest).unwrap();

        assert_eq!((doc.width, doc.height), (8, 8));

        for (layer, old) in doc.layers.iter().zip(&before.layers) {
            assert!(layer.rect.width() > 0 && layer.rect.height() > 0);
            let area = (layer.rect.width() * layer.rect.height()) as usize;
            for channel in layer.channels() {
                assert_eq!(
                    channel.data.len(),
                    area,
                    "{} channel {}",
                    layer.name,
                    channel.id
                );
            }
            if let (Some(new_mask), Some(old_mask)) = (&layer.mask, &old.mask) {
                assert_eq!(new_mask.rect.width(), old_mask.rect.width() * 2);
                assert_eq!(new_mask.rect.height(), old_mask.rect.height() * 2);
                let marea = (new_mask.rect.width() * new_mask.rect.height()) as usize;
                assert_eq!(new_mask.data.as_ref().unwrap().len(), marea);
            }
        }
        assert_eq!(doc.channels[0].data.len(), 64);
        doc.composite
            .replace(crate::composite_rgba(&doc))
            .expect("valid composite");
        assert_eq!(doc.composite, crate::composite_rgba(&doc));
    }

    #[test]
    fn zero_width_is_invalid_and_unchanged() {
        let mut doc = sample_doc();
        let before = doc.clone();
        let err = resize_document(&mut doc, 0, 8, pictura_ops::Resample::Bilinear);
        assert!(matches!(err, Err(pictura_ops::OpsError::InvalidParams(_))));
        assert_eq!(doc, before);
    }

    #[test]
    fn double_then_half_restores_dims() {
        let mut doc = sample_doc();
        let (w0, h0) = (doc.width, doc.height);
        let dims0: Vec<(i32, i32)> = doc
            .layers
            .iter()
            .map(|l| (l.rect.width(), l.rect.height()))
            .collect();

        resize_document(&mut doc, 8, 8, pictura_ops::Resample::Nearest).unwrap();
        resize_document(&mut doc, 4, 4, pictura_ops::Resample::Nearest).unwrap();

        assert_eq!((doc.width, doc.height), (w0, h0));
        let dims1: Vec<(i32, i32)> = doc
            .layers
            .iter()
            .map(|l| (l.rect.width(), l.rect.height()))
            .collect();
        assert_eq!(dims0, dims1);
        for layer in &doc.layers {
            let area = (layer.rect.width() * layer.rect.height()) as usize;
            for channel in layer.channels() {
                assert_eq!(channel.data.len(), area);
            }
        }
    }

    #[test]
    fn resize_resamples_the_native_store_and_saves() {
        let mut doc = sample_doc();
        doc.source
            .set_native(
                Some(BitDepth::Sixteen),
                Some(SourcePlanes {
                    depth: BitDepth::Sixteen,
                    width: 4,
                    height: 4,
                    samples: Samples::U16(vec![0; 3 * 4 * 4].into()),
                }),
            )
            .expect("valid native source");
        pictura_codec::edit_layer(doc.layers[0].as_mut(), |parts| {
            parts.raw_channels = vec![RawChannel {
                id: 3,
                data: vec![0, 0, 1].into(),
            }]
        });
        doc.layers[0].source_channels = Some(SourceChannels {
            depth: BitDepth::Sixteen,
            rect: rect(0, 0, 4, 4),
            planes: vec![(3, Samples::U16(vec![0; 16].into()))],
            working_idents: [0; 3],
        });

        // A no-op resize (unchanged dimensions) keeps the channel.
        resize_document(&mut doc, 4, 4, pictura_ops::Resample::Nearest).unwrap();
        assert_eq!(
            doc.layers[0].raw_channels().len(),
            1,
            "a no-op resize keeps the unmodeled channel"
        );
        assert!(doc.layers[0].source_channels.is_some());

        resize_document(&mut doc, 8, 8, pictura_ops::Resample::Nearest).unwrap();
        assert!(
            doc.layers[0].raw_channels().is_empty(),
            "the unmodeled raw stream is dropped"
        );
        let store = doc.layers[0]
            .source_channels
            .as_ref()
            .expect("the native store is resampled");
        assert_eq!(store.rect, doc.layers[0].rect);
        assert_eq!(
            store.planes.iter().find(|(id, _)| *id == 3).unwrap().1,
            Samples::U16(vec![0; 64].into())
        );
        let planes = doc.source.planes().as_ref().unwrap();
        assert_eq!((planes.width, planes.height), (8, 8));
        assert_eq!(planes.samples.len(), 3 * 64);

        doc.composite
            .replace(crate::composite_rgba(&doc))
            .expect("valid composite");
        let bytes = pictura_codec::write_psd(&doc).expect("a resized high-depth doc saves");
        assert_eq!(u16::from_be_bytes(bytes[22..24].try_into().unwrap()), 16);
    }
}
