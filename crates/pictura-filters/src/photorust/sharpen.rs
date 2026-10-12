//! Sharpen Edges, ported from photorust's `filters/convolve.rs` (#222).

use super::convolve::gaussian_blur_accelerated;
use crate::photorust::pixmap::Pixmap;
use rayon::prelude::*;

/// Filter ▸ Sharpen ▸ Sharpen Edges: sharpen where there is an edge and leave
/// everything else alone.
///
/// Plain Sharpen treats a patch of film grain in a flat sky exactly like the
/// boundary of a petal, and crisping the grain is not what anybody wanted.
/// This weighs each pixel's sharpening by how much of an edge is actually
/// there, so flat areas come through untouched and can therefore take a
/// heavier hand where it counts.
///
/// The edge is measured on the *blurred* copy rather than the original, so
/// that noise — which is by definition what a blur removes — does not read as
/// an edge and invite the sharpening it was meant to escape.
pub fn sharpen_edges(pixmap: &mut Pixmap) {
    if pixmap.is_empty() {
        return;
    }
    const AMOUNT: f32 = 1.0;
    /// Gradients below this are flat ground, above the second are a definite
    /// edge; in between the effect fades in, because a hard cut-off draws a
    /// visible outline round every shape it decides to sharpen.
    const EDGE_FLOOR: f32 = 6.0;
    const EDGE_CEILING: f32 = 36.0;

    let original = pixmap.clone();
    let mut blurred = pixmap.clone();
    gaussian_blur_accelerated(&mut blurred, 1.0);

    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;
    let blur = &blurred;

    // Port adaptation: the brightness is computed once per pixel rather than
    // at each of the Sobel taps that read it; the values are the same.
    let lumas: Vec<f32> = blur
        .as_bytes()
        .par_chunks_exact(4)
        .map(|p| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
        .collect();
    let lumas = &lumas;
    let luma = |x: i32, y: i32| -> f32 {
        lumas[(y.clamp(0, height - 1) * width + x.clamp(0, width - 1)) as usize]
    };

    let stride = pixmap.stride();
    pixmap
        .as_bytes_mut()
        .par_chunks_exact_mut(stride)
        .enumerate()
        .for_each(|(row, out)| {
            let y = row as i32;
            for x in 0..width {
                // Sobel, on brightness: an edge is an edge whatever colour it
                // separates.
                let gx = -luma(x - 1, y - 1) - 2.0 * luma(x - 1, y) - luma(x - 1, y + 1)
                    + luma(x + 1, y - 1)
                    + 2.0 * luma(x + 1, y)
                    + luma(x + 1, y + 1);
                let gy = -luma(x - 1, y - 1) - 2.0 * luma(x, y - 1) - luma(x + 1, y - 1)
                    + luma(x - 1, y + 1)
                    + 2.0 * luma(x, y + 1)
                    + luma(x + 1, y + 1);
                let magnitude = (gx * gx + gy * gy).sqrt() / 4.0;

                let t = ((magnitude - EDGE_FLOOR) / (EDGE_CEILING - EDGE_FLOOR)).clamp(0.0, 1.0);
                // Smoothstep, so the mask has no corner in it to show up as a
                // ring where the sharpening starts.
                let weight = t * t * (3.0 - 2.0 * t) * AMOUNT;
                if weight <= 0.0 {
                    continue;
                }

                let o = original.get(x, y);
                if o.a == 0 {
                    continue;
                }
                let b = blur.get(x, y);
                let i = x as usize * 4;
                // Alpha rides the same weight as the colour: the edge mask
                // comes from the visible image, so the layer edge moves with
                // it (the fourth write is dropped again unless the caller
                // writes the pixmap's alpha back for a moving kernel).
                for (c, (oc, bc)) in [(o.r, b.r), (o.g, b.g), (o.b, b.b), (o.a, b.a)]
                    .into_iter()
                    .enumerate()
                {
                    let diff = oc as f32 - bc as f32;
                    out[i + c] = (oc as f32 + diff * weight).clamp(0.0, 255.0) as u8;
                }
            }
        });
}
