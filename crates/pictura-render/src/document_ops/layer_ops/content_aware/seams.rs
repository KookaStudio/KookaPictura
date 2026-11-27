use pictura_core::memory::try_buffer;
use std::sync::atomic::{AtomicBool, Ordering};

const PURPOSE: &str = "content-aware scale scratch";

pub(super) fn buffer<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    try_buffer(len, value, PURPOSE).ok()
}

pub(super) fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// Forward-energy dynamic programming. Equal paths prefer the leftmost predecessor.
pub(in crate::document_ops::layer_ops) fn minimum_vertical_seam(
    intensity: &[f32],
    protect: Option<&[f32]>,
    width: usize,
    height: usize,
    cancel: &AtomicBool,
) -> Option<Vec<usize>> {
    let area = width.checked_mul(height)?;
    if width == 0
        || height == 0
        || intensity.len() != area
        || protect.is_some_and(|p| p.len() != area)
        || cancelled(cancel)
    {
        return None;
    }
    let mut predecessor = buffer(area, 0i8)?;
    let mut previous = buffer(width, 0f64)?;
    let mut current = buffer(width, 0f64)?;
    for x in 0..width {
        let left = x.saturating_sub(1);
        let right = (x + 1).min(width - 1);
        previous[x] = f64::from((intensity[right] - intensity[left]).abs())
            + protect.map_or(0.0, |p| f64::from(p[x]) * 1_000_000.0);
    }
    for y in 1..height {
        if cancelled(cancel) {
            return None;
        }
        for x in 0..width {
            let left = x.saturating_sub(1);
            let right = (x + 1).min(width - 1);
            let i = y * width + x;
            let l = f64::from(intensity[y * width + left]);
            let r = f64::from(intensity[y * width + right]);
            let above = f64::from(intensity[(y - 1) * width + x]);
            let base = (r - l).abs();
            let mut best = f64::INFINITY;
            let mut best_x = x;
            for px in left..=right {
                let extra = if px < x {
                    (above - l).abs()
                } else if px > x {
                    (above - r).abs()
                } else {
                    0.0
                };
                let candidate = previous[px] + base + extra;
                if candidate < best {
                    best = candidate;
                    best_x = px;
                }
            }
            predecessor[i] = (best_x as isize - x as isize) as i8;
            current[x] = best + protect.map_or(0.0, |p| f64::from(p[i]) * 1_000_000.0);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    let mut endpoint = 0;
    for x in 1..width {
        if previous[x] < previous[endpoint] {
            endpoint = x;
        }
    }
    let mut seam = buffer(height, 0usize)?;
    for y in (0..height).rev() {
        seam[y] = endpoint;
        endpoint = (endpoint as isize + isize::from(predecessor[y * width + endpoint])) as usize;
    }
    Some(seam)
}

pub(super) fn resize_width(
    planes: &mut [Vec<f32>],
    protection: &mut Option<Vec<f32>>,
    colors: &[usize],
    width: &mut usize,
    height: usize,
    target: usize,
    cancel: &AtomicBool,
) -> Option<()> {
    if colors.is_empty() {
        return None;
    }
    while *width != target {
        if cancelled(cancel) {
            return None;
        }
        let area = width.checked_mul(height)?;
        let mut intensity = buffer(area, 0.0f32)?;
        for &index in colors {
            for (dst, &sample) in intensity.iter_mut().zip(&planes[index]) {
                *dst += sample / colors.len() as f32;
            }
        }
        for value in &mut intensity {
            *value = if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        let seam =
            minimum_vertical_seam(&intensity, protection.as_deref(), *width, height, cancel)?;
        let grow = *width < target;
        for plane in planes.iter_mut() {
            *plane = change_plane(plane, *width, height, &seam, grow)?;
        }
        if let Some(plane) = protection.as_mut() {
            *plane = change_plane(plane, *width, height, &seam, grow)?;
        }
        *width = if grow { *width + 1 } else { *width - 1 };
    }
    Some(())
}

fn change_plane(
    src: &[f32],
    width: usize,
    height: usize,
    seam: &[usize],
    grow: bool,
) -> Option<Vec<f32>> {
    let new_width = if grow {
        width.checked_add(1)?
    } else {
        width.checked_sub(1)?
    };
    if new_width == 0 {
        return None;
    }
    let mut dst = buffer(new_width.checked_mul(height)?, 0.0f32)?;
    for y in 0..height {
        let x = seam[y];
        let old = &src[y * width..(y + 1) * width];
        let row = &mut dst[y * new_width..(y + 1) * new_width];
        if grow {
            row[..=x].copy_from_slice(&old[..=x]);
            let neighbor = if x + 1 < width {
                x + 1
            } else {
                x.saturating_sub(1)
            };
            row[x + 1] = (old[x] + old[neighbor]) * 0.5;
            row[x + 2..].copy_from_slice(&old[x + 1..]);
        } else {
            row[..x].copy_from_slice(&old[..x]);
            row[x..].copy_from_slice(&old[x + 1..]);
        }
    }
    Some(dst)
}

pub(super) fn transpose(plane: &mut Vec<f32>, width: usize, height: usize) -> Option<()> {
    let mut out = buffer(width.checked_mul(height)?, 0.0f32)?;
    for y in 0..height {
        for x in 0..width {
            out[x * height + y] = plane[y * width + x];
        }
    }
    *plane = out;
    Some(())
}
