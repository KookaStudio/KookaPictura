//! Row-parallel helpers for the CPU compositor.
//!
//! A per-pixel content loop touches only its own pixel, Dissolve is a pure
//! `(x, y)` hash, and `blend_if` reads the same pixel, so rows are independent
//! and may run on rayon. Effect kernels and the knockout `cover` canvas stay
//! sequential, so the output is byte-identical to the serial oracle. See D10 in
//! `openspec/changes/canvas-view-performance/design.md`.

#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use pictura_core::{BlendMode, ColorMode, Document, Layer, PsdRect, SampleRow};
use rayon::prelude::*;

use crate::composite::{blend, channel_plane, unit_u8, Canvas, Px};
use crate::composite_blend::{blend_into_row_ctx, BlendCtx, BlendOverride, Kernel};
use crate::composite_plan::PlanNode;

/// Test-only switch that routes [`for_each_row`] and the chunked kernels
/// through their serial branch, so a test can compare the parallel and
/// sequential outputs on the same stack.
#[cfg(test)]
pub(crate) static FORCE_SERIAL: AtomicBool = AtomicBool::new(false);
/// Test-only count of parallel branch entries, so a test can prove the parallel
/// path actually runs when [`FORCE_SERIAL`] is off.
#[cfg(test)]
pub(crate) static PARALLEL_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Test-only switch that routes `composite_pixels` through the general kernel.
#[cfg(test)]
pub(crate) static FORCE_GENERAL: AtomicBool = AtomicBool::new(false);

thread_local! {
    static MASK_ROW: std::cell::RefCell<Vec<f32>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

// Fast row-span evaluations `[Masked, Alpha, Opaque]` and covered passes for
// this thread, so concurrent tests cannot pollute each other's counters.
#[cfg(test)]
thread_local! {
    static KERNEL_RUNS: std::cell::Cell<[usize; 3]> =
        const { std::cell::Cell::new([0; 3]) };
    static COVERED_RUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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

#[cfg(test)]
pub(crate) fn set_force_general(on: bool) {
    FORCE_GENERAL.store(on, Ordering::Relaxed);
}

#[cfg(test)]
fn forced_general() -> bool {
    FORCE_GENERAL.load(Ordering::Relaxed)
}

#[cfg(test)]
fn kernel_slot(kernel: Kernel) -> usize {
    match kernel {
        Kernel::Masked => 0,
        Kernel::Alpha => 1,
        Kernel::Opaque => 2,
    }
}

#[cfg(test)]
fn note_kernel(kernel: Kernel) {
    KERNEL_RUNS.with(|runs| {
        let mut counts = runs.get();
        counts[kernel_slot(kernel)] += 1;
        runs.set(counts);
    });
}

#[cfg(test)]
fn note_covered() {
    COVERED_RUNS.with(|runs| runs.set(runs.get() + 1));
}

#[cfg(test)]
pub(crate) fn kernel_runs(kernel: Kernel) -> usize {
    KERNEL_RUNS.with(|runs| runs.get()[kernel_slot(kernel)])
}

#[cfg(test)]
pub(crate) fn covered_runs() -> usize {
    COVERED_RUNS.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(crate) fn reset_kernel_runs() {
    KERNEL_RUNS.with(|runs| runs.set([0; 3]));
    COVERED_RUNS.with(|runs| runs.set(0));
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
#[inline]
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

/// The general formula for Normal mode over the backdrop `cb`. Keeps
/// `blend_parts_row`'s operation order and division exactly.
#[inline]
fn blend_normal_px(cb: Px, as_: f32, cs: [f32; 3]) -> Px {
    let ab = cb.a;
    let ao = as_ + ab * (1.0 - as_);
    if ao <= 0.0 {
        return Px::default();
    }
    let b = cs;
    let co = |idx: usize, c: f32| {
        ((1.0 - ab) * as_ * cs[idx] + as_ * ab * b[idx] + (1.0 - as_) * ab * c) / ao
    };
    Px {
        r: co(0, cb.r),
        g: co(1, cb.g),
        b: co(2, cb.b),
        a: ao,
    }
}

/// The opaque shape: alpha 1, so `ao == 1` and the division folds away.
fn blend_row_opaque(row: &mut [Px], x0: i32, x1: i32, ox: i32, cs: [&[u8]; 3]) {
    for (i, x) in (x0..x1).enumerate() {
        let at = (x - ox) as usize;
        let cb = row[at];
        let ab = cb.a;
        let one_minus = 1.0 - ab;
        let c = [unit_u8(cs[0][i]), unit_u8(cs[1][i]), unit_u8(cs[2][i])];
        row[at] = Px {
            r: one_minus * c[0] + ab * c[0],
            g: one_minus * c[1] + ab * c[1],
            b: one_minus * c[2] + ab * c[2],
            a: 1.0,
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_row_alpha(
    row: &mut [Px],
    x0: i32,
    x1: i32,
    ox: i32,
    ctx: &BlendCtx<'_>,
    alpha: Option<&[u8]>,
    cs: [&[u8]; 3],
) {
    let opacity = ctx.opacity();
    let fill = ctx.fill();
    for (i, x) in (x0..x1).enumerate() {
        let a = alpha.map_or(1.0, |line| unit_u8(line[i]));
        if a <= 0.0 {
            continue;
        }
        let content = (a * opacity) * fill;
        if content <= 0.0 {
            continue;
        }
        let at = (x - ox) as usize;
        let c = [unit_u8(cs[0][i]), unit_u8(cs[1][i]), unit_u8(cs[2][i])];
        row[at] = blend_normal_px(row[at], content, c);
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_row_masked(
    row: &mut [Px],
    x0: i32,
    x1: i32,
    ox: i32,
    ctx: &BlendCtx<'_>,
    alpha: Option<&[u8]>,
    cs: [&[u8]; 3],
    mask: &[f32],
) {
    let opacity = ctx.opacity();
    let fill = ctx.fill();
    for (i, x) in (x0..x1).enumerate() {
        let a = alpha.map_or(1.0, |line| unit_u8(line[i]));
        if a <= 0.0 {
            continue;
        }
        let content = (a * opacity) * fill;
        if content <= 0.0 {
            continue;
        }
        let as_ = content * mask[i];
        if as_ <= 0.0 {
            continue;
        }
        let at = (x - ox) as usize;
        let c = [unit_u8(cs[0][i]), unit_u8(cs[1][i]), unit_u8(cs[2][i])];
        row[at] = blend_normal_px(row[at], as_, c);
    }
}

/// The per-pixel body for a layer that cannot use a row-span kernel.
#[allow(clippy::too_many_arguments)]
fn blend_row_general(
    row: &mut [Px],
    mut cover: Option<&mut [bool]>,
    x0: i32,
    x1: i32,
    y: i32,
    ox: i32,
    rect: PsdRect,
    ctx: &BlendCtx<'_>,
    layer: &Layer,
    doc: &Document,
    clip: PsdRect,
    over: Option<&BlendOverride>,
    native: [Option<SampleRow<'_>>; 4],
    rows: [Option<&[u8]>; 4],
    from: usize,
    gray: bool,
) {
    for x in x0..x1 {
        let lx = (x - rect.left) as usize;
        let a = source_unit(native[3], rows[3], lx, from, 255);
        if a <= 0.0 {
            continue;
        }
        let (r, g, b) = if gray {
            let v = source_color(native[0], rows[0], lx, from);
            (v, v, v)
        } else {
            (
                source_color(native[0], rows[0], lx, from),
                source_color(native[1], rows[1], lx, from),
                source_color(native[2], rows[2], lx, from),
            )
        };
        blend_into_row_ctx(
            row,
            cover.as_deref_mut(),
            ox,
            ctx,
            layer,
            doc,
            clip,
            over,
            x,
            y,
            [r, g, b],
            a,
        );
    }
}

fn select_kernel(node: &PlanNode<'_>, ctx: &BlendCtx<'_>, alpha_plane: bool) -> Option<Kernel> {
    #[cfg(test)]
    if forced_general()
        || crate::composite_blend::legacy_blend()
        || crate::composite_blend::legacy_source()
    {
        return None;
    }
    let native = node.sources.as_ref().is_some_and(|s| s.any_native());
    ctx.kernel(native, alpha_plane)
}

/// A source sample from the resolved native row, or the 8-bit plane.
#[inline]
/// `bytes` holds the 8-bit row from column `from`, so sample `lx` is at
/// `lx - from`; `native` holds the whole row.
fn source_unit(
    native: Option<SampleRow<'_>>,
    bytes: Option<&[u8]>,
    lx: usize,
    from: usize,
    default: u8,
) -> f32 {
    if let Some(row) = native {
        if let Some(v) = row.unit(lx) {
            return v;
        }
    }
    unit_u8(
        bytes
            .and_then(|row| row.get(lx - from))
            .copied()
            .unwrap_or(default),
    )
}

fn source_color(
    native: Option<SampleRow<'_>>,
    bytes: Option<&[u8]>,
    lx: usize,
    from: usize,
) -> f32 {
    if let Some(SampleRow::F32(row)) = native {
        if let Some(&sample) = row.get(lx) {
            return if sample.is_finite() { sample } else { 0.0 };
        }
    }
    source_unit(native, bytes, lx, from, 0)
}

/// Paint a pixel layer's own channels over the canvas, row-parallel.
pub(crate) fn composite_pixels(
    canvas: &mut Canvas,
    node: &PlanNode<'_>,
    doc: &Document,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    let layer = node.layer;
    // A smart-object layer with no raster proxy has no channel to draw from. The
    // embedded-source branch in `composite_layer_inner` handles it; if that
    // source is unusable, leave the backdrop unchanged instead of painting the
    // channel-less rect black.
    if layer.smart_object().is_some() && channel_plane(layer, 0).is_none() {
        return;
    }
    #[cfg(test)]
    if crate::composite_blend::legacy_source() {
        composite_pixels_legacy(canvas, node, doc, ctx, over);
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
    let source = node
        .sources
        .as_ref()
        .expect("a visible layer resolves its native planes");
    let span = (x0 - rect.left) as usize..(x1 - rect.left) as usize;
    let from = span.start;

    // A knockout canvas carries a cover mask; those layers take the general
    // kernel so the cover flag is recorded.
    let mut kernel = select_kernel(node, ctx, alpha.is_some());
    if canvas.cover.is_some() {
        #[cfg(test)]
        if kernel.is_some() {
            note_covered();
        }
        kernel = None;
    }

    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        // A row is copied out of a tiled plane (borrowed from a flat one), so
        // the plane is never materialized into one contiguous slice.
        let ly = (y - rect.top) as usize;
        // Only the columns the canvas shows are copied out of a tiled plane.
        let rows = [ch0, ch1, ch2, alpha].map(|p| p.and_then(|p| p.row_span(ly, span.clone(), lw)));
        let [r0, r1, r2, ra] = rows.each_ref().map(Option::as_deref);
        let native = source.row(ly, lw);
        if let (Some(kernel), None) = (kernel, cover.as_deref()) {
            if let (Some(r0), Some(r1), Some(r2)) = (r0, r1, r2) {
                let cs = if gray { [r0, r0, r0] } else { [r0, r1, r2] };
                match kernel {
                    Kernel::Opaque => blend_row_opaque(row, x0, x1, ox, cs),
                    Kernel::Alpha => blend_row_alpha(row, x0, x1, ox, ctx, ra, cs),
                    Kernel::Masked => MASK_ROW.with(|cell| {
                        let mut mask = cell.borrow_mut();
                        ctx.mask_row(x0, x1, y, ra, &mut mask);
                        blend_row_masked(row, x0, x1, ox, ctx, ra, cs, &mask[..]);
                    }),
                }
                #[cfg(test)]
                note_kernel(kernel);
                return;
            }
        }
        // A high-depth Grayscale/RGB layer reads its native samples; a
        // channel missing from the store falls back to its 8-bit plane.
        blend_row_general(
            row,
            cover.as_deref_mut(),
            x0,
            x1,
            y,
            ox,
            rect,
            ctx,
            layer,
            doc,
            clip,
            over,
            native,
            [r0, r1, r2, ra],
            from,
            gray,
        );
    });
}

/// The pre-change pixel loop, kept as the source-sampling oracle.
#[cfg(test)]
fn composite_pixels_legacy(
    canvas: &mut Canvas,
    node: &PlanNode<'_>,
    doc: &Document,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    let layer = node.layer;
    if layer.smart_object().is_some() && channel_plane(layer, 0).is_none() {
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
        let rows = [ch0, ch1, ch2, alpha].map(|p| p.and_then(|p| p.row(ly, lw)));
        let [r0, r1, r2, ra] = rows.each_ref().map(Option::as_deref);
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
            crate::composite_blend::blend_into_row_ctx(
                row,
                cover.as_deref_mut(),
                ox,
                ctx,
                layer,
                doc,
                clip,
                over,
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
pub(crate) fn composite_canvas(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    inner: &Canvas,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    let ox = canvas.ox;
    let x0 = canvas.x0();
    let x1 = canvas.x1();
    let y0 = canvas.y0();
    let y1 = canvas.y1();
    let clip = canvas.clip();
    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        for x in x0..x1 {
            let p = inner.px[inner.idx(x as usize, y as usize)];
            if p.a > 0.0 {
                blend_into_row_ctx(
                    row,
                    cover.as_deref_mut(),
                    ox,
                    ctx,
                    layer,
                    doc,
                    clip,
                    over,
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

    #[test]
    fn source_unit_uses_the_unit_table() {
        for value in 0..=255u8 {
            let bytes = [value];
            let got = source_unit(None, Some(&bytes), 0, 0, 0).to_bits();
            assert_eq!(
                got,
                crate::composite::unit_u8(value).to_bits(),
                "sample {value}"
            );
        }
    }
}
