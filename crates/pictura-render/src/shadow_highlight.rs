use pictura_adjust::{Adjustment, ColorBalanceMode};
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, BitDepth, Coverage, Layer, PsdRect, Samples,
};
use pictura_filters::FilterError;

use crate::{channel, filter::coverage_row};

pub(crate) fn apply_native_region(
    layer: &mut Layer,
    adjustment: &Adjustment,
    mask: Option<&Coverage>,
    region: PsdRect,
    mode: ColorBalanceMode,
) -> Result<(), FilterError> {
    if layer_pixel_locked(layer) {
        return Err(FilterError::Locked);
    }
    let rect = layer.rect;
    let Some(clip) = region.intersect(rect) else {
        return Ok(());
    };
    let width = rect.width() as usize;
    let height = rect.height() as usize;
    let n = width * height;
    let invalid = || FilterError::InvalidParams("incomplete 16-bit Shadows/Highlights layer".into());
    let clear_locked = layer_transparency_locked(layer);
    let grayscale = channel(layer, 1).is_none() && channel(layer, 2).is_none();
    let ids: &[i16] = if grayscale { &[0] } else { &[0, 1, 2] };
    let (channels, source) = layer.channels_and_source_mut();
    let store = source.as_mut().ok_or_else(invalid)?;
    if store.depth != BitDepth::Sixteen || store.rect != rect {
        return Err(invalid());
    }
    let mut packed = Vec::with_capacity(3 * n);
    let mut targets = Vec::with_capacity(ids.len());
    for &id in ids {
        let bytes = channels.iter().position(|c| c.id == id && c.data.len() == n)
            .ok_or_else(invalid)?;
        let plane = store.planes.iter().position(|(key, data)| {
            *key == id && matches!(data, Samples::U16(values) if values.len() == n)
        }).ok_or_else(invalid)?;
        let Samples::U16(values) = &store.planes[plane].1 else {
            return Err(invalid());
        };
        packed.extend_from_slice(values.as_slice());
        targets.push((bytes, plane));
    }
    if grayscale {
        packed.extend_from_within(..n);
        packed.extend_from_within(..n);
    }
    let mut samples = Samples::U16(packed.into());
    pictura_adjust::apply_native_in_mode(
        adjustment, &mut samples, width, height, 3,
        if grayscale { ColorBalanceMode::Grayscale } else { mode },
    )
    .map_err(|error| FilterError::InvalidParams(error.to_string()))?;
    let Samples::U16(output) = samples else {
        return Err(invalid());
    };

    // Resolve the transparency lock from the authoritative alpha, with the
    // same byte-plane fallback used by compositing.
    let clear: Option<Vec<bool>> = if clear_locked {
        if let Some((_, Samples::U16(alpha))) = store.planes.iter().find(|(id, _)| *id == -1)
            .filter(|(_, plane)| plane.len() == n)
        {
            Some(alpha.iter().map(|&sample| sample == 0).collect())
        } else {
            channels.iter().find(|c| c.id == -1 && c.data.len() == n)
                .map(|alpha| alpha.data.iter().map(|&sample| sample == 0).collect())
        }
    } else {
        None
    };
    let x0 = (clip.left - rect.left) as usize;
    let y0 = (clip.top - rect.top) as usize;
    let mut coverage = vec![255; clip.width() as usize];
    for (index, (bytes, plane)) in targets.into_iter().enumerate() {
        let Samples::U16(values) = &mut store.planes[plane].1 else {
            unreachable!("validated 16-bit color plane");
        };
        let values = values.as_mut_slice();
        let bytes = channels[bytes].data.as_mut_slice();
        for y in 0..clip.height() as usize {
            coverage_row(mask, clip.left, clip.top + y as i32, &mut coverage);
            let from = (y + y0) * width + x0;
            for (x, &coverage) in coverage.iter().enumerate() {
                let i = from + x;
                if coverage == 0 || clear.as_ref().is_some_and(|alpha| alpha[i]) {
                    continue;
                }
                let corrected = u32::from(output[index * n + i]);
                let old = u32::from(values[i]);
                let coverage = u32::from(coverage);
                values[i] = ((old * (255 - coverage) + corrected * coverage + 127) / 255) as u16;
                bytes[i] = (values[i] >> 8) as u8;
            }
        }
    }
    Ok(())
}
