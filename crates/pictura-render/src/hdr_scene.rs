//! Scene-referred region reads for the 32-bit preview and Info panel.

use pictura_core::{BitDepth, ColorMode, Document, PsdRect, Samples};

pub fn supports_hdr_scene(doc: &Document) -> bool {
    doc.source.depth() == Some(BitDepth::ThirtyTwo)
        && doc.source.mode().is_none()
        && matches!(doc.mode, ColorMode::Rgb | ColorMode::Grayscale)
}

/// Return straight-alpha RGBA samples for an in-bounds region.
pub fn hdr_scene_region(doc: &Document, rect: PsdRect, use_merged: bool) -> Option<Vec<[f32; 4]>> {
    if !supports_hdr_scene(doc)
        || rect.top < 0
        || rect.left < 0
        || rect.right as u32 > doc.width
        || rect.bottom as u32 > doc.height
        || rect.width() <= 0
        || rect.height() <= 0
    {
        return None;
    }
    let w = rect.width() as usize;
    let h = rect.height() as usize;
    if !doc.layers.is_empty() && !use_merged {
        let canvas = crate::composite_region::region_canvas(
            doc, rect.left as u32, rect.top as u32, w as u32, h as u32,
        );
        return Some(canvas.px.into_iter().map(|p| [p.r, p.g, p.b, p.a]).collect());
    }

    let retained = doc.source.planes().as_ref()?;
    let Samples::F32(samples) = &retained.samples else {
        return None;
    };
    if retained.width != doc.width || retained.height != doc.height {
        return None;
    }
    let doc_w = doc.width as usize;
    let plane = doc_w * doc.height as usize;
    let colors = doc.mode.color_channels() as usize;
    if samples.len() < colors * plane {
        return None;
    }
    let merged_alpha = doc.composite.signed_merged_alpha() && samples.len() >= (colors + 1) * plane;
    let mut out = vec![[0.0, 0.0, 0.0, 1.0]; w * h];
    let mut line = vec![0.0; w];
    for (offset, row) in out.chunks_exact_mut(w).enumerate() {
        let at = (rect.top as usize + offset) * doc_w + rect.left as usize;
        for channel in 0..colors + usize::from(merged_alpha) {
            samples.read_into(channel * plane + at, &mut line);
            for (pixel, value) in row.iter_mut().zip(line.iter().copied()) {
                if channel == colors {
                    pixel[3] = value.clamp(0.0, 1.0);
                } else if colors == 1 {
                    pixel[..3].fill(value);
                } else {
                    pixel[channel] = value;
                }
            }
        }
    }
    Some(out)
}
