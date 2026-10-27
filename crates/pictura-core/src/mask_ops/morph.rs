//! Separable square-footprint dilation and erosion.
//!
//! Each axis is a monotonic-deque sliding window, so a line costs `O(n)` and
//! the plane costs `O(1)` per sample per axis regardless of the radius.

use std::collections::VecDeque;

use rayon::prelude::*;

/// What a window reads past the plane edge.
#[derive(Clone, Copy, Debug)]
pub enum Border<T> {
    /// Out-of-plane taps read this sample.
    Zero(T),
    /// Out-of-plane taps read the nearest edge sample.
    Clamp,
}

/// Dilate over a `(2 * radius + 1)²` square footprint.
pub fn dilate<T: Copy + PartialOrd + Send + Sync>(
    src: &[T],
    w: usize,
    h: usize,
    radius: usize,
    border: Border<T>,
) -> Vec<T> {
    morph(src, w, h, radius, border, true)
}

/// Erode over a `(2 * radius + 1)²` square footprint.
pub fn erode<T: Copy + PartialOrd + Send + Sync>(
    src: &[T],
    w: usize,
    h: usize,
    radius: usize,
    border: Border<T>,
) -> Vec<T> {
    morph(src, w, h, radius, border, false)
}

fn morph<T: Copy + PartialOrd + Send + Sync>(
    src: &[T],
    w: usize,
    h: usize,
    radius: usize,
    border: Border<T>,
    want_max: bool,
) -> Vec<T> {
    let n = w.saturating_mul(h);
    if radius == 0 || n == 0 || src.len() != n {
        return src.to_vec();
    }
    let zero = match border {
        Border::Zero(value) => Some(value),
        Border::Clamp => None,
    };
    let mut tmp = src.to_vec();
    tmp.par_chunks_mut(w).enumerate().for_each(|(y, out)| {
        slide(&src[y * w..(y + 1) * w], radius, zero, want_max, out);
    });
    let parts: Vec<(usize, usize, Vec<T>)> = (0..w)
        .step_by(COLUMN_BLOCK)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|x0| {
            let x1 = (x0 + COLUMN_BLOCK).min(w);
            (x0, x1, vertical(&tmp, w, h, x0, x1, radius, zero, want_max))
        })
        .collect();
    drop(tmp);
    let mut out = src.to_vec();
    for (x0, x1, part) in parts {
        let bw = x1 - x0;
        for y in 0..h {
            out[y * w + x0..y * w + x1].copy_from_slice(&part[y * bw..(y + 1) * bw]);
        }
    }
    out
}

/// Columns per parallel block. One cache line of `u8` samples; wide enough for
/// row-contiguous reads and writes, narrow enough for small planes.
const COLUMN_BLOCK: usize = 64;

fn slide<T: Copy + PartialOrd + Send + Sync>(
    line: &[T],
    radius: usize,
    zero: Option<T>,
    want_max: bool,
    out: &mut [T],
) {
    let n = line.len();
    // Cap at the line length so the padded walk stays O(n) when the radius
    // exceeds the line; duplicate and zero taps cannot change a max or min.
    let radius = radius.min(n);
    let window = 2 * radius + 1;
    let padded = n + 2 * radius;
    let mut deque: VecDeque<(usize, T)> = VecDeque::new();
    for p in 0..padded {
        let value = sample(line, p as isize - radius as isize, zero);
        while let Some(&(_, back)) = deque.back() {
            if (want_max && back <= value) || (!want_max && back >= value) {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back((p, value));
        while let Some(&(front, _)) = deque.front() {
            if front + window <= p {
                deque.pop_front();
            } else {
                break;
            }
        }
        if p + 1 >= window {
            out[p + 1 - window] = deque.front().expect("non-empty window").1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn vertical<T: Copy + PartialOrd + Send + Sync>(
    src: &[T],
    w: usize,
    h: usize,
    x0: usize,
    x1: usize,
    radius: usize,
    zero: Option<T>,
    want_max: bool,
) -> Vec<T> {
    let bw = x1 - x0;
    // Cap at the column height so the padded walk stays O(h) when the radius
    // exceeds it; duplicate and zero taps cannot change a max or min.
    let radius = radius.min(h);
    let window = 2 * radius + 1;
    let padded = h + 2 * radius;
    let mut deque: Vec<VecDeque<(usize, T)>> = (0..bw).map(|_| VecDeque::new()).collect();
    let mut out = vec![src[0]; h * bw];
    for p in 0..padded {
        for (j, deque) in deque.iter_mut().enumerate() {
            let x = x0 + j;
            let value = sample_column(src, w, x, h, p as isize - radius as isize, zero);
            while let Some(&(_, back)) = deque.back() {
                if (want_max && back <= value) || (!want_max && back >= value) {
                    deque.pop_back();
                } else {
                    break;
                }
            }
            deque.push_back((p, value));
            while let Some(&(front, _)) = deque.front() {
                if front + window <= p {
                    deque.pop_front();
                } else {
                    break;
                }
            }
            if p + 1 >= window {
                out[(p + 1 - window) * bw + j] = deque.front().expect("non-empty window").1;
            }
        }
    }
    out
}

#[inline]
fn sample<T: Copy>(line: &[T], index: isize, zero: Option<T>) -> T {
    let n = line.len() as isize;
    if index >= 0 && index < n {
        line[index as usize]
    } else {
        match zero {
            Some(value) => value,
            None => line[index.clamp(0, n - 1) as usize],
        }
    }
}

#[inline]
fn sample_column<T: Copy>(
    src: &[T],
    w: usize,
    x: usize,
    h: usize,
    index: isize,
    zero: Option<T>,
) -> T {
    let n = h as isize;
    if index >= 0 && index < n {
        src[index as usize * w + x]
    } else {
        match zero {
            Some(value) => value,
            None => src[index.clamp(0, n - 1) as usize * w + x],
        }
    }
}
