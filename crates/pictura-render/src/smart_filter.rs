//! Apply a smart object's `filterFX` chain to its rendered source.
//!
//! Models the camera-raw smart filter and the Pictura-only Lens Correction
//! smart filter. Unknown ids are skipped, so an unsupported stack leaves the
//! caller on its proxy/source path unchanged.

use pictura_adjust::render_pictura_raw;
use pictura_codec::{
    decode_lens_correction_slots, decode_pictura_raw_settings, CAMERA_RAW_FILTER_ID,
    LENS_CORRECTION_FILTER_ID,
};
use pictura_core::{
    Document, Layer, PicturaRawSettings, PixelBuffer, PsdRect, SmartFilter, SmartObject,
};
use pictura_filters::{Filter, LensCorrectionParams};

use crate::composite::{blend_into, render_smart_source, Canvas};

/// A decoded smart filter ready to render.
#[derive(Debug, Clone)]
pub enum SmartFilterOp {
    CameraRaw(Box<PicturaRawSettings>),
    LensCorrection(LensCorrectionParams),
}

/// Decode a [`SmartFilter`]'s options into a renderable op, or `None` for an
/// id this engine does not model, or a Lens Correction record whose slots do
/// not map (an unknown profile id).
pub fn decode_smart_filter(filter: &SmartFilter) -> Option<SmartFilterOp> {
    match filter.filter_id {
        CAMERA_RAW_FILTER_ID => Some(SmartFilterOp::CameraRaw(Box::new(
            decode_pictura_raw_settings(&filter.options),
        ))),
        LENS_CORRECTION_FILTER_ID => Some(SmartFilterOp::LensCorrection(
            LensCorrectionParams::from_slots(&decode_lens_correction_slots(&filter.options))?,
        )),
        _ => None,
    }
}

/// Apply the enabled smart filters to `base`, in order, over the colour planes;
/// alpha is preserved.
///
/// When `enabled` is false (the `filterFXStyle.enab` group flag) the chain is
/// bypassed and `base` is returned unchanged.
pub fn apply_smart_filter_chain(
    base: &PixelBuffer,
    filters: &[SmartFilter],
    enabled: bool,
) -> PixelBuffer {
    if !enabled {
        return base.clone();
    }
    let mut out = base.clone();
    for filter in filters.iter().filter(|f| f.enabled) {
        if let Some(op) = decode_smart_filter(filter) {
            out = apply_op(&out, op);
        }
    }
    out
}

fn apply_op(base: &PixelBuffer, op: SmartFilterOp) -> PixelBuffer {
    match op {
        SmartFilterOp::CameraRaw(settings) => {
            render_pictura_raw(base, &settings).unwrap_or_else(|_| base.clone())
        }
        SmartFilterOp::LensCorrection(params) => {
            let mut out = base.clone();
            pictura_filters::apply(&Filter::LensCorrection { params }, &mut out)
                .map(|()| out)
                .unwrap_or_else(|_| base.clone())
        }
    }
}

/// Apply the chain to `base`, then mix it back through the smart object's
/// filter mask: a fully masked-out pixel keeps `base` (the unfiltered source).
/// `origin` is `base`'s top-left in document coordinates, which the mask rect
/// is expressed in. Without a mask (or with it disabled) the filtered result is
/// returned unchanged.
pub(crate) fn apply_masked_chain(
    base: &PixelBuffer,
    so: &SmartObject,
    origin: [i32; 2],
) -> PixelBuffer {
    let filtered = apply_smart_filter_chain(base, &so.smart_filters, so.smart_filters_enabled);
    if so
        .filter_mask
        .as_ref()
        .filter(|_| so.filter_mask_enabled)
        .is_none()
    {
        return filtered;
    }
    let plane = base.width as usize * base.height as usize;
    let mut out = filtered.clone();
    for by in 0..base.height as usize {
        for bx in 0..base.width as usize {
            let m = mask_value(so, origin[0] + bx as i32, origin[1] + by as i32);
            if m >= 1.0 {
                continue;
            }
            let i = by * base.width as usize + bx;
            for c in 0..4 {
                let b = f32::from(base.data[c * plane + i]);
                let f = f32::from(filtered.data[c * plane + i]);
                out.data[c * plane + i] = (b + (f - b) * m).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// The mask coverage at document pixel `(x, y)`, `1.0` when the filter applies.
/// Outside the mask rect the extend value is used (`filterMaskExtendWithWhite`
/// white, otherwise transparent). `1.0` when there is no enabled mask.
fn mask_value(so: &SmartObject, x: i32, y: i32) -> f32 {
    let Some(mask) = so.filter_mask.as_ref().filter(|_| so.filter_mask_enabled) else {
        return 1.0;
    };
    let (w, h) = (mask.rect.width(), mask.rect.height());
    let (mx, my) = (x - mask.rect.left, y - mask.rect.top);
    if w <= 0 || h <= 0 || mx < 0 || my < 0 || mx >= w || my >= h {
        return if so.filter_mask_extend_with_white {
            1.0
        } else {
            0.0
        };
    }
    f32::from(mask.data[my as usize * w as usize + mx as usize]) / 255.0
}

/// Composite a smart object's filtered source, rendering the source first so an
/// already-baked raster proxy is never filtered a second time.
///
/// Returns `false` (leaving the layer to the normal proxy/source path) unless
/// there is at least one enabled filter and every enabled filter is understood.
pub(crate) fn composite_smart_filtered_source(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
) -> bool {
    let Some(so) = layer.smart_object.as_ref() else {
        return false;
    };
    if !crate::composite_route::smart_filters_apply(layer) {
        return false;
    }
    let region = PsdRect {
        top: layer.rect.top.max(canvas.y0()),
        left: layer.rect.left.max(canvas.x0()),
        bottom: layer.rect.bottom.min(canvas.y1()),
        right: layer.rect.right.min(canvas.x1()),
    };
    if region.width() <= 0 || region.height() <= 0 {
        return false;
    }
    let Some(base) = render_smart_source(so, layer.rect, region) else {
        return false;
    };
    let src = apply_masked_chain(&base, so, [region.left, region.top]);

    let rw = region.width() as usize;
    let rh = region.height() as usize;
    let plane = rw * rh;
    for by in 0..rh {
        for bx in 0..rw {
            let i = by * rw + bx;
            blend_into(
                canvas,
                layer,
                doc,
                region.left as usize + bx,
                region.top as usize + by,
                [
                    src.data[i] as f32 / 255.0,
                    src.data[plane + i] as f32 / 255.0,
                    src.data[2 * plane + i] as f32 / 255.0,
                ],
                src.data[3 * plane + i] as f32 / 255.0,
            );
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_codec::LENS_CORRECTION_FILTER_NAME;
    use pictura_core::{FilterMask, SmartObject, SmartObjectKind};

    /// A Lens Correction smart filter carrying `slots`, built through the codec
    /// so the options bytes are the ones a commit would write.
    fn lens_filter(slots: &[f64]) -> SmartFilter {
        let mut layer = Layer {
            smart_object: Some(SmartObject {
                kind: SmartObjectKind::Embedded,
                payload: Some(b"8BPS\x00\x01x".to_vec()),
                ..Default::default()
            }),
            ..Default::default()
        };
        pictura_codec::attach_lens_correction_filter(&mut layer, slots).expect("attaches");
        layer
            .smart_object
            .as_ref()
            .expect("smart object")
            .smart_filters[0]
            .clone()
    }

    #[test]
    fn lens_correction_renders_the_chain() {
        let slots = [
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 25.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
        ];
        let filter = lens_filter(&slots);
        let base = PixelBuffer {
            width: 8,
            height: 8,
            channels: 4,
            data: (0..8 * 8 * 4)
                .map(|i| (i * 7) as u8)
                .collect::<Vec<u8>>()
                .into(),
        };
        let chained = apply_smart_filter_chain(&base, std::slice::from_ref(&filter), true);

        let params = LensCorrectionParams::from_slots(&slots).expect("maps");
        let mut expected = base.clone();
        pictura_filters::apply(&Filter::LensCorrection { params }, &mut expected).expect("applies");
        assert_eq!(chained.data.as_ref(), expected.data.as_ref());
        assert_ne!(chained.data.as_ref(), base.data.as_ref());
    }

    #[test]
    fn unmappable_slots_are_refused() {
        // A profile id past the bundled set makes the filter undecodable.
        let options = pictura_codec::write_descriptor(&pictura_codec::DescValue::Object {
            name: String::new(),
            class_id: b"x".to_vec(),
            items: vec![(b"Prof".to_vec(), pictura_codec::DescValue::Long(999))],
        });
        let filter = SmartFilter {
            filter_id: LENS_CORRECTION_FILTER_ID,
            name: LENS_CORRECTION_FILTER_NAME.to_string(),
            enabled: true,
            options,
        };
        assert!(decode_smart_filter(&filter).is_none());

        // A wrong slot count never reaches the mapping through the codec (decode
        // always returns 16), so check the mapping directly.
        assert!(LensCorrectionParams::from_slots(&[0.0; 15]).is_none());
    }

    fn base_buffer() -> PixelBuffer {
        PixelBuffer {
            width: 4,
            height: 4,
            channels: 4,
            data: (0..64).map(|i| (i * 7) as u8).collect::<Vec<u8>>().into(),
        }
    }

    fn distortion_filter() -> SmartFilter {
        lens_filter(&[
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
        ])
    }

    #[test]
    fn the_chain_renders_through_the_mask() {
        let filter = distortion_filter();
        // The left column is masked out, the rest applies.
        let mut coverage = vec![255u8; 16];
        for y in 0..4 {
            coverage[y * 4] = 0;
        }
        let so = SmartObject {
            smart_filters: vec![filter.clone()],
            smart_filters_enabled: true,
            filter_mask_enabled: true,
            filter_mask_extend_with_white: true,
            filter_mask: Some(FilterMask {
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 4,
                    right: 4,
                },
                data: coverage.into(),
            }),
            ..Default::default()
        };
        let base = base_buffer();
        let out = apply_masked_chain(&base, &so, [0, 0]);
        let filtered = apply_smart_filter_chain(&base, std::slice::from_ref(&filter), true);
        assert_ne!(
            filtered.data.as_ref(),
            base.data.as_ref(),
            "the filter changes pixels"
        );
        for y in 0..4usize {
            let i = y * 4;
            assert_eq!(
                out.data[i], base.data[i],
                "masked-out pixel keeps the source"
            );
            assert_eq!(
                out.data[i + 1],
                filtered.data[i + 1],
                "unmasked pixel is filtered"
            );
        }
    }

    #[test]
    fn a_layer_without_a_mask_is_unchanged() {
        let filter = distortion_filter();
        let so = SmartObject {
            smart_filters: vec![filter.clone()],
            smart_filters_enabled: true,
            filter_mask: None,
            ..Default::default()
        };
        let base = base_buffer();
        let out = apply_masked_chain(&base, &so, [0, 0]);
        let filtered = apply_smart_filter_chain(&base, std::slice::from_ref(&filter), true);
        assert_eq!(out.data.as_ref(), filtered.data.as_ref());
    }
}
