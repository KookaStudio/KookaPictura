//! Row-parallel helpers for the CPU compositor.
//!
//! A per-pixel content loop touches only its own pixel, Dissolve is a pure
//! `(x, y)` hash, and `blend_if` reads the same pixel, so rows are independent
//! and may run on rayon. Effect kernels and the knockout `cover` canvas stay
//! sequential, so the output is byte-identical to the serial oracle. See D10 in
//! `openspec/changes/canvas-view-performance/design.md`.

#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use pictura_core::{BlendMode, ColorMode, Document, Layer, SampleRow};
use rayon::prelude::*;

use crate::composite::{blend, channel_plane, Canvas, Px};
use crate::composite_blend::{blend_into_row_ctx, BlendCtx, SourcePlanes};

/// Test-only switch that routes [`for_each_row`] and the chunked kernels
/// through their serial branch, so a test can compare the parallel and
/// sequential outputs on the same stack.
#[cfg(test)]
pub(crate) static FORCE_SERIAL: AtomicBool = AtomicBool::new(false);
/// Test-only count of parallel branch entries, so a test can prove the parallel
/// path actually runs when [`FORCE_SERIAL`] is off.
#[cfg(test)]
pub(crate) static PARALLEL_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Whether the test-only switch forces the serial branch. Always `false` in a
/// production build.
#[cfg(test)]
pub(crate) fn serial() -> bool {
    let forced = FORCE_SERIAL.load(Ordering::Relaxed);
    if !forced {
        PARALLEL_RUNS.fetch_add(1, Ordering::Relaxed);
    }
    forced
}

/// Serializes the tests that toggle [`FORCE_SERIAL`], so a single-process
/// `cargo test` run does not race the process-global switch.
#[cfg(test)]
pub(crate) fn serial_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(not(test))]
#[inline]
pub(crate) fn serial() -> bool {
    false
}

/// splitmix64 noise for Dissolve and the effects' Noise; pure in `(x, y)`.
pub(crate) fn dissolve_noise(x: usize, y: usize) -> f32 {
    let mut z = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ 0xD1B5_4A32_D192_ED03;
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    ((z >> 56) as f32) / 256.0
}

/// Run `f` for each canvas row in `[y0, y1)`, handing it the row's pixels, the
/// row's `cover` flags when present, and the row's document `y`. Rows run in
/// parallel unless the canvas carries a `cover` mask (a knockout composite) or
/// the test-only serial switch is set.
pub(crate) fn for_each_row<F>(canvas: &mut Canvas, y0: i32, y1: i32, f: F)
where
    F: Fn(i32, &mut [Px], Option<&mut [bool]>) + Sync + Send,
{
    if y1 <= y0 {
        return;
    }
    let w = canvas.w;
    let start = (y0 - canvas.oy) as usize * w;
    let end = (y1 - canvas.oy) as usize * w;
    let Canvas { px, cover, .. } = canvas;
    let rows = &mut px[start..end];

    #[cfg(test)]
    let serial = cover.is_some() || serial();
    #[cfg(not(test))]
    let serial = cover.is_some();

    match cover.as_mut() {
        Some(cov) => {
            let cov = &mut cov[start..end];
            for (r, (row, cov)) in rows.chunks_mut(w).zip(cov.chunks_mut(w)).enumerate() {
                f(y0 + r as i32, row, Some(cov));
            }
        }
        None if serial => {
            for (r, row) in rows.chunks_mut(w).enumerate() {
                f(y0 + r as i32, row, None);
            }
        }
        None => {
            rows.par_chunks_mut(w).enumerate().for_each(|(r, row)| {
                f(y0 + r as i32, row, None);
            });
        }
    }
}

/// Composite an opacity-weighted source sample over the backdrop at
/// `row[x - ox]` with a blend mode.
#[allow(clippy::too_many_arguments)]
pub(crate) fn blend_parts_row(
    row: &mut [Px],
    cover: Option<&mut [bool]>,
    ox: i32,
    x: i32,
    y: i32,
    cs: [f32; 3],
    alpha: f32,
    mode: BlendMode,
) {
    let mut as_ = alpha;
    if as_ <= 0.0 {
        return;
    }
    let i = (x - ox) as usize;
    let cb = row[i];

    // Dissolve is stochastic: the effective alpha is a threshold on a fixed
    // per-pixel noise field, and passing pixels go fully opaque.
    // ponytail: deterministic splitmix hash, not the unpublished noise tile;
    // swap when a CS6 dither baseline exists.
    if matches!(mode, BlendMode::Dissolve) {
        if dissolve_noise(x as usize, y as usize) >= as_ {
            return;
        }
        as_ = 1.0;
    }
    if let Some(cover) = cover {
        cover[i] = true;
    }

    let ab = cb.a;
    let b = blend(mode, [cb.r, cb.g, cb.b], cs);
    let ao = as_ + ab * (1.0 - as_);
    if ao <= 0.0 {
        row[i] = Px::default();
        return;
    }
    let co = |idx: usize, c: f32| {
        ((1.0 - ab) * as_ * cs[idx] + as_ * ab * b[idx] + (1.0 - as_) * ab * c) / ao
    };
    row[i] = Px {
        r: co(0, cb.r),
        g: co(1, cb.g),
        b: co(2, cb.b),
        a: ao,
    };
}

/// A source sample from the resolved native row, or the 8-bit plane.
#[inline]
fn source_unit(native: Option<SampleRow<'_>>, bytes: Option<&[u8]>, lx: usize, default: u8) -> f32 {
    if let Some(row) = native {
        if let Some(v) = row.unit(lx) {
            return v;
        }
    }
    bytes
        .and_then(|row| row.get(lx))
        .copied()
        .unwrap_or(default) as f32
        / 255.0
}

/// Paint a pixel layer's own channels over the canvas, row-parallel.
pub(crate) fn composite_pixels(canvas: &mut Canvas, layer: &Layer, doc: &Document) {
    // A smart-object layer with no raster proxy has no channel to draw from. The
    // embedded-source branch in `composite_layer_inner` handles it; if that
    // source is unusable, leave the backdrop unchanged instead of painting the
    // channel-less rect black.
    if layer.smart_object.is_some() && channel_plane(layer, 0).is_none() {
        return;
    }
    #[cfg(test)]
    if crate::composite_blend::legacy_source() {
        composite_pixels_legacy(canvas, layer, doc);
        return;
    }
    let lw = layer.rect.width();
    let lh = layer.rect.height();
    if lw <= 0 || lh <= 0 {
        return;
    }
    let lw = lw as usize;
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let gray = matches!(
        doc.mode,
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone
    );
    let ch0 = channel_plane(layer, 0);
    let ch1 = channel_plane(layer, 1).or(ch0);
    let ch2 = channel_plane(layer, 2).or(ch0);
    let alpha = channel_plane(layer, -1);
    let ox = canvas.ox;
    let rect = layer.rect;
    let clip = canvas.clip();
    let ctx = BlendCtx::new(layer, doc, clip);
    let source = SourcePlanes::resolve(layer, doc);

    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        // A row lies inside one band, so reading rows keeps a banded plane
        // shared instead of materializing it into one contiguous slice.
        let ly = (y - rect.top) as usize;
        let r0 = ch0.and_then(|p| p.row(ly, lw));
        let r1 = ch1.and_then(|p| p.row(ly, lw));
        let r2 = ch2.and_then(|p| p.row(ly, lw));
        let ra = alpha.and_then(|p| p.row(ly, lw));
        let native = source.row(ly, lw);
        for x in x0..x1 {
            let lx = (x - rect.left) as usize;
            // A high-depth Grayscale/RGB layer reads its native samples; a
            // channel missing from the store falls back to its 8-bit plane.
            let a = source_unit(native[3], ra, lx, 255);
            if a <= 0.0 {
                continue;
            }
            let (r, g, b) = if gray {
                let v = source_unit(native[0], r0, lx, 0);
                (v, v, v)
            } else {
                (
                    source_unit(native[0], r0, lx, 0),
                    source_unit(native[1], r1, lx, 0),
                    source_unit(native[2], r2, lx, 0),
                )
            };
            blend_into_row_ctx(
                row,
                cover.as_deref_mut(),
                ox,
                &ctx,
                layer,
                doc,
                clip,
                x,
                y,
                [r, g, b],
                a,
            );
        }
    });
}

/// The pre-change pixel loop, kept as the source-sampling oracle.
#[cfg(test)]
fn composite_pixels_legacy(canvas: &mut Canvas, layer: &Layer, doc: &Document) {
    if layer.smart_object.is_some() && channel_plane(layer, 0).is_none() {
        return;
    }
    let lw = layer.rect.width();
    let lh = layer.rect.height();
    if lw <= 0 || lh <= 0 {
        return;
    }
    let lw = lw as usize;
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let gray = matches!(
        doc.mode,
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone
    );
    let ch0 = channel_plane(layer, 0);
    let ch1 = channel_plane(layer, 1).or(ch0);
    let ch2 = channel_plane(layer, 2).or(ch0);
    let alpha = channel_plane(layer, -1);
    let ox = canvas.ox;
    let rect = layer.rect;
    let clip = canvas.clip();
    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        let ly = (y - rect.top) as usize;
        let r0 = ch0.and_then(|p| p.row(ly, lw));
        let r1 = ch1.and_then(|p| p.row(ly, lw));
        let r2 = ch2.and_then(|p| p.row(ly, lw));
        let ra = alpha.and_then(|p| p.row(ly, lw));
        for x in x0..x1 {
            let lx = (x - rect.left) as usize;
            let li = ly * lw + lx;
            let unit = |id: i16, r: Option<&[u8]>| {
                crate::composite_native::native_unit(layer, doc, id, li).unwrap_or_else(|| {
                    r.and_then(|r| r.get(lx)).copied().unwrap_or(0) as f32 / 255.0
                })
            };
            let (r, g, b) = if gray {
                let v = unit(0, r0);
                (v, v, v)
            } else {
                (unit(0, r0), unit(1, r1), unit(2, r2))
            };
            let a = crate::composite_native::native_unit(layer, doc, -1, li).unwrap_or_else(|| {
                ra.and_then(|r| r.get(lx)).copied().unwrap_or(255) as f32 / 255.0
            });
            crate::composite_blend::blend_into_row(
                row,
                cover.as_deref_mut(),
                ox,
                layer,
                doc,
                clip,
                x,
                y,
                [r, g, b],
                a,
            );
        }
    });
}

/// Blend an isolated group's rendered canvas onto the running canvas,
/// row-parallel.
pub(crate) fn composite_canvas(canvas: &mut Canvas, layer: &Layer, doc: &Document, inner: &Canvas) {
    let ox = canvas.ox;
    let x0 = canvas.x0();
    let x1 = canvas.x1();
    let y0 = canvas.y0();
    let y1 = canvas.y1();
    let clip = canvas.clip();
    let ctx = BlendCtx::new(layer, doc, clip);
    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        for x in x0..x1 {
            let p = inner.px[inner.idx(x as usize, y as usize)];
            if p.a > 0.0 {
                blend_into_row_ctx(
                    row,
                    cover.as_deref_mut(),
                    ox,
                    &ctx,
                    layer,
                    doc,
                    clip,
                    x,
                    y,
                    [p.r, p.g, p.b],
                    p.a,
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_row_equals_sequential_row() {
        let _guard = serial_test_lock();
        let mut parallel = Canvas::new(8, 5);
        let mut sequential = Canvas::new(8, 5);
        for (i, p) in parallel.px.iter_mut().enumerate() {
            *p = Px {
                r: i as f32,
                g: 1.0,
                b: 0.5,
                a: 1.0,
            };
        }
        sequential.px = parallel.px.clone();

        for_each_row(&mut parallel, 0, 5, |_, row, _| {
            for p in row.iter_mut() {
                p.r += 0.25;
            }
        });
        FORCE_SERIAL.store(true, Ordering::Relaxed);
        for_each_row(&mut sequential, 0, 5, |_, row, _| {
            for p in row.iter_mut() {
                p.r += 0.25;
            }
        });
        FORCE_SERIAL.store(false, Ordering::Relaxed);

        for (a, b) in parallel.px.iter().zip(sequential.px.iter()) {
            assert_eq!(a.r.to_bits(), b.r.to_bits());
            assert_eq!(a.a.to_bits(), b.a.to_bits());
        }
    }
}
