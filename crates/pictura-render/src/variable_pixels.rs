use pictura_core::variables::{Placement, ScaleMethod};
use pictura_core::{BitDepth, ColorMode, Document, PsdRect};

pub fn place_pixels(
    rgba: &[u8],
    width: u32,
    height: u32,
    frame: PsdRect,
    placement: Placement,
) -> Result<(PsdRect, Vec<u8>), String> {
    let w = i64::from(frame.right) - i64::from(frame.left);
    let h = i64::from(frame.bottom) - i64::from(frame.top);
    if w <= 0 || h <= 0 {
        return Err("The pixel replacement frame is empty".into());
    }
    if width == 0
        || height == 0
        || u64::from(width)
            .checked_mul(height.into())
            .and_then(|n| n.checked_mul(4))
            != Some(rgba.len() as u64)
        || !placement.valid()
    {
        return Err("Invalid replacement image or placement".into());
    }
    let (input_w, input_h) = (f64::from(width), f64::from(height));
    let (dw, dh) = match placement.method {
        ScaleMethod::AsIs => (i64::from(width), i64::from(height)),
        ScaleMethod::Conform => (w, h),
        method => {
            let (sx, sy) = (w as f64 / input_w, h as f64 / input_h);
            let scale = if method == ScaleMethod::Fit {
                sx.min(sy)
            } else {
                sx.max(sy)
            };
            (
                (input_w * scale).round().max(1.0) as i64,
                (input_h * scale).round().max(1.0) as i64,
            )
        }
    };
    if dw > i64::from(i32::MAX) || dh > i64::from(i32::MAX) {
        return Err("Pixel replacement exceeds layer dimensions".into());
    }
    let left = i64::from(frame.left) + (w - dw) * i64::from(placement.horizontal) / 2;
    let top = i64::from(frame.top) + (h - dh) * i64::from(placement.vertical) / 2;
    let (mut l, mut t, mut r, mut b) = (left, top, left + dw, top + dh);
    if placement.clip {
        l = l.max(frame.left.into());
        t = t.max(frame.top.into());
        r = r.min(frame.right.into());
        b = b.min(frame.bottom.into());
    }
    let to_coord =
        |coord| i32::try_from(coord).map_err(|_| "Pixel replacement exceeds layer coordinates");
    let rect = PsdRect {
        left: to_coord(l)?,
        top: to_coord(t)?,
        right: to_coord(r)?,
        bottom: to_coord(b)?,
    };
    let count = u64::try_from(r - l)
        .ok()
        .zip(u64::try_from(b - t).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .filter(|n| *n <= pictura_codec::ImageBudget::default().max_alloc_bytes / 32)
        .and_then(|n| usize::try_from(n * 4).ok())
        .ok_or("Pixel replacement exceeds the image memory budget")?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|_| "Pixel replacement allocation failed")?;
    for y in t..b {
        for x in l..r {
            if dw == i64::from(width) && dh == i64::from(height) {
                let index = (((y - top) as usize * width as usize) + (x - left) as usize) * 4;
                pixels.extend_from_slice(&rgba[index..index + 4]);
            } else {
                let sx = (((x - left) as f64 + 0.5) * input_w / dw as f64 - 0.5)
                    .clamp(0.0, input_w - 1.0);
                let sy = (((y - top) as f64 + 0.5) * input_h / dh as f64 - 0.5)
                    .clamp(0.0, input_h - 1.0);
                pixels.extend_from_slice(&sample(rgba, width as usize, height as usize, sx, sy));
            }
        }
    }
    Ok((rect, pixels))
}

fn sample(rgba: &[u8], width: usize, height: usize, x: f64, y: f64) -> [u8; 4] {
    let (left, top) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x.fract(), y.fract());
    let mut result = [0.0; 4];
    for (row, wy) in [(top, 1.0 - fy), ((top + 1).min(height - 1), fy)] {
        for (col, wx) in [(left, 1.0 - fx), ((left + 1).min(width - 1), fx)] {
            let start = (row * width + col) * 4;
            let alpha = f64::from(rgba[start + 3]) * wx * wy;
            for channel in 0..3 {
                result[channel] += f64::from(rgba[start + channel]) * alpha;
            }
            result[3] += alpha;
        }
    }
    let a = result[3].round() as u8;
    if a == 0 {
        return [0; 4];
    }
    [
        (result[0] / result[3]).round() as u8,
        (result[1] / result[3]).round() as u8,
        (result[2] / result[3]).round() as u8,
        a,
    ]
}

pub fn replace_variable_pixels(
    doc: &mut Document,
    path: &str,
    rect: PsdRect,
    pixels: &[u8],
) -> Result<bool, String> {
    use crate::{convert_bit_depth, convert_mode_with_profiles};
    use pictura_codec::{ConversionOptions, Profile};

    let mode = doc.document_color_mode();
    let depth = doc.document_bit_depth();
    if !matches!(
        mode,
        ColorMode::Rgb | ColorMode::Grayscale | ColorMode::Cmyk | ColorMode::Lab
    ) {
        return Err(format!("Pixel replacement does not support {mode:?}"));
    }
    let profile = doc
        .source
        .working_profile()
        .as_ref()
        .and_then(|profile| Profile::from_icc(profile.as_slice()).ok());
    let cmyk = pictura_codec::document_cmyk_profile(doc);
    let mut source = Document::from_rgba(
        "Replacement",
        rect.width() as u32,
        rect.height() as u32,
        pixels,
    );
    if mode == ColorMode::Rgb {
        if let Some(profile) = profile {
            if !pictura_codec::convert_document(&mut source, &profile) {
                return Err("The replacement cannot use the document profile".into());
            }
        }
    } else {
        convert_mode_with_profiles(
            &mut source,
            mode,
            cmyk.as_ref(),
            ConversionOptions::default(),
        )
        .map_err(|e| e.to_string())?;
    }
    if depth != BitDepth::Eight {
        convert_bit_depth(&mut source, depth).map_err(|e| e.to_string())?;
    }
    let mut replacement = source.layers.pop().ok_or("Missing replacement pixels")?;
    let layer = crate::resolve_path_mut(doc, path).ok_or("Missing target layer")?;
    if layer.rect == rect
        && layer.channels() == replacement.channels()
        && layer.source_channels == replacement.source_channels
    {
        return Ok(false);
    }
    let channels = std::mem::take(replacement.channels_mut());
    pictura_codec::edit_layer(layer, |parts| {
        parts.rect = rect;
        parts.channels = channels;
        parts.source_channels = replacement.source_channels;
        parts.raw_channels.retain(|channel| channel.id < -2);
    });
    Ok(true)
}

#[cfg(test)]
mod tests;
