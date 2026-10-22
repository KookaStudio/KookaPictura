//! M27: apply `pictura_filters::Filter` kernels as wgpu compute passes.
//!
//! [`apply_filter_active`] runs a pointwise / separable-convolution filter on
//! the GPU when `gpu_enabled` is set and a usable adapter exists; otherwise it
//! delegates to the CPU oracle [`pictura_filters::apply`]. The accelerated set
//! is Gaussian Blur, Box Blur, Motion Blur, Blur, Blur More, Sharpen, Sharpen
//! More, Unsharp Mask, High Pass, Surface Blur, Maximum, Minimum, Median,
//! Custom (5×5) and Oil Paint. Any other filter (or any GPU failure) falls back
//! to the CPU with a byte-identical result and [`Backend::Cpu`].
//!
//! One WGSL compute shader carries every kernel behind a mode selector and a
//! normalized weight buffer:
//!
//! - `KERNEL` — a KxK convolution (Custom) with one rounded 8-bit write;
//! - `SEP_H`/`SEP_V` — a separable 1-D convolution keeping an unrounded `f32`
//!   intermediate between the row and column passes, exactly like
//!   `pictura_filters::kernel::gaussian_blur_planes` (Gaussian, Box, and Blur
//!   and Blur More at their fixed radii);
//! - `MOTION` — the directional 1-D sum (Motion Blur);
//! - `RADIAL_ZOOM` — bilinear samples along a pixel's ray (Radial Blur, Zoom);
//! - `COMBINE` — a pointwise mix of the original and a blurred copy (Unsharp
//!   Mask, Sharpen and Sharpen More at their fixed amounts, High Pass);
//! - `SURFACE` — the per-channel thresholded window mean (Surface Blur);
//! - `MEDIAN` — an exact per-channel order statistic over the clamped
//!   `(2r+1)²` window, selected by binary search on the byte value (the window
//!   count is always odd, so the middle element is unique and matches the CPU
//!   sort bit-for-bit).
//!
//! Samples are uploaded planar 8-bit, packed four bytes per `u32` word with the
//! same byte helpers as [`crate::gpu`]; one invocation owns a whole word, so
//! the read-modify-write of a shared word never races. A 4-channel buffer
//! uploads its alpha plane as a fourth plane only when the kernel filters alpha
//! (see [`pictura_filters::Filter::moves_alpha`]); otherwise alpha is never
//! uploaded and never written back, so the buffer keeps it bit-for-bit.
//!
//! This module reuses the compositor's cached [`crate::gpu::shared_device`]
//! rather than opening a second Vulkan device, so filters and compositing share
//! one queue and one set of cached pipelines. The multi-pass stages each submit
//! separately (queue order is the barrier), which is fine while per-call setup
//! dominates; batch the encoder if filter dispatch ever shows up in a profile.

use pictura_core::PixelBuffer;
use pictura_filters::{Filter, FilterError};

use crate::gpu::Backend;

mod plan;
mod resources;
#[cfg(test)]
mod tests;

use plan::plan;
use resources::run;

const MODE_KERNEL: u32 = 0;
const MODE_SEP_H: u32 = 1;
const MODE_SEP_V: u32 = 2;
const MODE_MOTION: u32 = 3;
const MODE_COMBINE: u32 = 4;
const MODE_SURFACE: u32 = 5;
const MODE_MORPH_H: u32 = 6;
const MODE_MORPH_V: u32 = 7;
const MODE_MEDIAN: u32 = 8;
const MODE_RADIAL_ZOOM: u32 = 9;
const MODE_OIL_LUMA: u32 = 20;
const MODE_OIL_AGG: u32 = 21;
const MODE_OIL_HEIGHT: u32 = 22;
const MODE_OIL_SHADE: u32 = 23;

const COMBINE_HIGH_PASS: u32 = 0;
const COMBINE_UNSHARP: u32 = 1;

/// Largest 1-D support the weight buffer accepts; anything larger falls back to
/// the CPU rather than allocating an unbounded kernel.
const MAX_WEIGHTS: usize = 512;

/// Whether a usable adapter exists for the GPU filter kernels.
///
/// Reuses the compositor's shared device ([`crate::gpu::shared_device`]) and
/// requires the four storage buffers the kernel binds; cheap to call
/// repeatedly and never panics.
pub fn filter_gpu_available() -> bool {
    crate::gpu::shared_device()
        .is_some_and(|(d, _)| d.limits().max_storage_buffers_per_shader_stage >= 4)
}

/// Apply `filter` in place; return which backend produced the buffer.
///
/// Uses the GPU when `gpu_enabled && filter_gpu_available()` and a kernel
/// exists for this filter, otherwise the CPU oracle [`pictura_filters::apply`].
/// Any GPU failure or kernel-less filter silently degrades to the CPU. Invalid
/// parameters surface as `Err(FilterError)` from the CPU oracle; never panics.
pub fn apply_filter_active(
    filter: &Filter,
    buf: &mut PixelBuffer,
    gpu_enabled: bool,
) -> Result<Backend, FilterError> {
    if gpu_enabled && filter_gpu_available() && apply_filter_gpu(filter, buf).is_some() {
        return Ok(Backend::Gpu);
    }
    pictura_filters::apply(filter, buf)?;
    Ok(Backend::Cpu)
}

fn apply_filter_gpu(filter: &Filter, buf: &mut PixelBuffer) -> Option<()> {
    let plan = plan(filter)?;
    if buf.channels != 3 && buf.channels != 4 {
        return None;
    }
    let n = buf.pixel_count();
    if n == 0 || buf.data.len() != n * buf.channels as usize {
        return None;
    }
    // A moving kernel takes the fourth plane through the shader; a value
    // kernel runs colour only and the buffer's alpha is left untouched.
    let planes = if buf.channels == 4 && filter.moves_alpha() {
        4
    } else {
        3
    };
    let mut input = Vec::with_capacity(n * planes);
    for c in 0..planes {
        input.extend_from_slice(&buf.data[c * n..c * n + n]);
    }
    let (device, queue) = crate::gpu::shared_device()?;
    let out = run(device, queue, &plan, &input, buf.width, buf.height)?;
    for c in 0..planes {
        buf.data[c * n..c * n + n].copy_from_slice(&out[c * n..c * n + n]);
    }
    Some(())
}
