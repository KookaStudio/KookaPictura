//! Scanline four-connected flood.

/// A 0 / 255 mask of the pixels four-connected to the non-zero samples of
/// `seeds` and accepted by `matches(from, to)`.
///
/// The seeds are sources and are included in the returned mask. `matches`
/// receives the linear index of an accepted pixel and of a neighbour, so a
/// fixed-reference wand and a per-pixel grow share the kernel. The frontier is
/// a stack of row spans; `None` when no seed lies in the plane.
pub fn flood(
    w: usize,
    h: usize,
    seeds: &[u8],
    matches: impl Fn(usize, usize) -> bool,
) -> Option<Vec<u8>> {
    flood_with_peak(w, h, seeds, matches).map(|(mask, _, _)| mask)
}

/// As [`flood`], also returning the peak span-stack length and the number of
/// first marks (seeds and expansions).
#[cfg(test)]
pub(crate) fn flood_peak(
    w: usize,
    h: usize,
    seeds: &[u8],
    matches: impl Fn(usize, usize) -> bool,
) -> Option<(Vec<u8>, usize, usize)> {
    flood_with_peak(w, h, seeds, matches)
}

/// As [`flood`] from the single pixel at linear index `seed`, without a
/// seed plane: a one-pixel source is a pixel, not a plane the size of the
/// mask. `None` when `seed` lies outside the plane.
pub fn flood_from(
    w: usize,
    h: usize,
    seed: usize,
    matches: impl Fn(usize, usize) -> bool,
) -> Option<Vec<u8>> {
    let n = w.saturating_mul(h);
    if seed >= n {
        return None;
    }
    let mut mask = vec![0u8; n];
    let mut visited = 0;
    let (y, x) = (seed / w, seed % w);
    mask[seed] = 255;
    let (left, right) = widen(&mut mask, w, y * w, x, x + 1, &matches, &mut visited);
    let mut stack = vec![(y, left, right)];
    spread(&mut mask, w, h, &mut stack, &matches, &mut visited);
    Some(mask)
}

fn flood_with_peak(
    w: usize,
    h: usize,
    seeds: &[u8],
    matches: impl Fn(usize, usize) -> bool,
) -> Option<(Vec<u8>, usize, usize)> {
    let n = w.saturating_mul(h);
    if n == 0 || seeds.len() < n {
        return None;
    }
    let mut mask = vec![0u8; n];
    let mut stack: Vec<(usize, usize, usize)> = Vec::new();
    let mut visited = 0usize;
    for y in 0..h {
        let base = y * w;
        let mut x = 0;
        while x < w {
            if seeds[base + x] == 0 || mask[base + x] != 0 {
                x += 1;
                continue;
            }
            let start = x;
            while x < w && seeds[base + x] != 0 {
                if mask[base + x] == 0 {
                    mask[base + x] = 255;
                    visited += 1;
                }
                x += 1;
            }
            let (left, right) = widen(&mut mask, w, base, start, x, &matches, &mut visited);
            stack.push((y, left, right));
            x = right;
        }
    }
    if stack.is_empty() {
        return None;
    }
    let peak = spread(&mut mask, w, h, &mut stack, &matches, &mut visited);
    Some((mask, peak, visited))
}

/// Grow the marked span `start..end` of the row at `base` left and right over
/// the pixels `matches` accepts; the span it reached.
fn widen(
    mask: &mut [u8],
    w: usize,
    base: usize,
    start: usize,
    end: usize,
    matches: &impl Fn(usize, usize) -> bool,
    visited: &mut usize,
) -> (usize, usize) {
    let mut left = start;
    while left > 0 && mask[base + left - 1] == 0 && matches(base + left, base + left - 1) {
        mask[base + left - 1] = 255;
        *visited += 1;
        left -= 1;
    }
    let mut right = end;
    while right < w && mask[base + right] == 0 && matches(base + right - 1, base + right) {
        mask[base + right] = 255;
        *visited += 1;
        right += 1;
    }
    (left, right)
}

/// Flood from the spans on `stack` into the rows above and below them until
/// none is left; the stack's peak length.
fn spread(
    mask: &mut [u8],
    w: usize,
    h: usize,
    stack: &mut Vec<(usize, usize, usize)>,
    matches: &impl Fn(usize, usize) -> bool,
    visited: &mut usize,
) -> usize {
    let mut peak = stack.len();
    while let Some((y, x0, x1)) = stack.pop() {
        for ny in [y.checked_sub(1), (y + 1 < h).then_some(y + 1)] {
            let Some(ny) = ny else {
                continue;
            };
            let nbase = ny * w;
            let mut x = x0;
            while x < x1 {
                let to = nbase + x;
                if mask[to] == 0 && matches(y * w + x, to) {
                    mask[to] = 255;
                    *visited += 1;
                    let mut left = x;
                    while left > 0 {
                        let candidate = nbase + left - 1;
                        if mask[candidate] != 0 || !matches(nbase + left, candidate) {
                            break;
                        }
                        mask[candidate] = 255;
                        *visited += 1;
                        left -= 1;
                    }
                    let mut right = x;
                    while right + 1 < w {
                        let candidate = nbase + right + 1;
                        if mask[candidate] != 0 || !matches(nbase + right, candidate) {
                            break;
                        }
                        mask[candidate] = 255;
                        *visited += 1;
                        right += 1;
                    }
                    stack.push((ny, left, right + 1));
                    peak = peak.max(stack.len());
                    x = right + 1;
                } else {
                    x += 1;
                }
            }
        }
    }
    peak
}
