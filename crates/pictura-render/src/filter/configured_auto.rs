use pictura_adjust::AutoHistogram;
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, AutoOptions, Coverage, Layer, Sample, Samples,
};
use pictura_filters::FilterError;

use super::{channel_mut, coverage_row};

pub fn apply_configured_auto_native(
    layer: &mut Layer,
    options: &AutoOptions,
    coverage: Option<&Coverage>,
) -> Result<(), FilterError> {
    if layer_pixel_locked(layer) {
        return Err(FilterError::Locked);
    }
    let rect = layer.rect;
    if rect.is_empty() {
        return Ok(());
    }
    let mut store = layer.source_channels.clone().ok_or_else(missing)?;
    let depth = store.depth;
    let color_ids = if crate::channel(layer, 1).is_some() { &[0, 1, 2][..] } else { &[0][..] };
    let indices: Vec<usize> = color_ids
        .iter()
        .map(|id| {
            store.planes.iter().position(|(channel, _)| channel == id).ok_or_else(missing)
        })
        .collect::<Result<_, _>>()?;
    let width = rect.width() as usize;
    let height = rect.height() as usize;
    let n = width * height;
    // Validate every working plane before publishing any native result.
    for &id in color_ids {
        if crate::channel(layer, id).is_none_or(|plane| plane.len() != n) {
            return Err(missing());
        }
    }
    let mut included = vec![255u8; n];
    for (y, row) in included.chunks_exact_mut(width).enumerate() {
        coverage_row(coverage, rect.left, rect.top + y as i32, row);
    }
    let alpha = store.planes.iter().find_map(|(id, samples)| {
        (*id == -1 && samples.len() == n && samples.depth() == depth).then_some(samples)
    });
    let clear = |i| match alpha {
        Some(Samples::U16(data)) => data[i] == 0,
        Some(Samples::F32(data)) => data[i] <= 0.0,
        _ => crate::channel(layer, -1).is_some_and(|a| a.get(i) == Some(&0)),
    };
    let keep_clear = layer_transparency_locked(layer);
    // Selection belongs to the fit as well as the write. Alpha participates
    // only when it owns the transparency lock; the alpha plane is never edited.
    if keep_clear {
        for (i, amount) in included.iter_mut().enumerate() {
            if clear(i) {
                *amount = 0;
            }
        }
    }

    let indices_rgb = if let &[r, g, b] = indices.as_slice() {
        [r, g, b]
    } else {
        [indices[0]; 3]
    };
    match depth {
        pictura_core::BitDepth::Sixteen => {
            apply::<u16>(&mut store.planes, indices_rgb, indices.len(), &included, options, Sample::to_unit)?;
        }
        pictura_core::BitDepth::ThirtyTwo => {
            apply::<f32>(&mut store.planes, indices_rgb, indices.len(), &included, options, f64::from)?;
        }
        _ => return Err(missing()),
    }

    for (&id, &index) in color_ids.iter().zip(&indices) {
        let narrowed = store.planes[index].1.narrow_to_u8();
        let out = channel_mut(layer, id).expect("color channels confirmed before publishing");
        for ((working, &value), &amount) in out.iter_mut().zip(narrowed.iter()).zip(&included) {
            if amount != 0 {
                *working = value;
            }
        }
    }
    layer.source_channels = Some(store);
    Ok(())
}

fn missing() -> FilterError {
    FilterError::InvalidParams("the layer's native Auto store is incomplete".into())
}

fn apply<T: Sample>(
    store: &mut [(i16, Samples)],
    rgb: [usize; 3],
    planes: usize,
    coverage: &[u8],
    options: &AutoOptions,
    raw: impl Fn(T) -> f64,
) -> Result<(), FilterError> {
    let mut histogram = AutoHistogram::new(u16::MAX);
    let [r, g, b] = rgb.map(|index| {
        T::as_slice_mut(&mut store[index].1)
            .filter(|samples| samples.len() == coverage.len())
            .map(|samples| samples.to_vec())
            .ok_or_else(missing)
    });
    let [r, g, b] = [r?, g?, b?];
    histogram.add_planes([&r, &g, &b], |i| coverage[i] != 0);
    for (index, transfer) in rgb.into_iter().zip(histogram.fit(options)).take(planes) {
        let Some(transfer) = transfer else { continue };
        let plane = T::as_slice_mut(&mut store[index].1).ok_or_else(missing)?;
        for (sample, &amount) in plane.iter_mut().zip(coverage) {
            if amount == 0 {
                continue;
            }
            let original = raw(*sample);
            // Extended HDR samples are preserved exactly; the fitted transfer
            // describes only the unit interval represented by its histogram.
            if !original.is_finite() || !(0.0..=1.0).contains(&original) {
                continue;
            }
            let target = transfer.map(original);
            *sample = T::from_unit(if amount == 255 {
                target
            } else {
                original + (target - original) * f64::from(amount) / 255.0
            });
        }
    }
    Ok(())
}
