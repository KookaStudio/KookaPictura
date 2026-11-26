//! Bounded translation registration on exposure-invariant luminance correlation.

const LIMIT: isize = 32;

pub(super) fn offset(
    base: &[[u8; 4]],
    moving: &[[u8; 4]],
    width: usize,
    height: usize,
) -> Option<(isize, isize)> {
    let stride = (width.min(height) / 96).max(1);
    let image = Registration {
        base,
        moving,
        width,
        height,
        stride,
    };
    // Enumerate the full bounded window at sparse resolution to avoid losing
    // highly textured shifts between coarse translation steps.
    let stride = stride.max(3);
    let coarse = Registration { stride, ..image };
    let mut candidates = Vec::new();
    for dy in -LIMIT..=LIMIT {
        for dx in -LIMIT..=LIMIT {
            if let Some(score) = coarse.score(dx, dy) {
                candidates.push((score, dx, dy));
            }
        }
    }
    candidates.sort_unstable_by(|a, b| b.0.total_cmp(&a.0).then_with(|| {
        (a.1.abs() + a.2.abs(), a.2, a.1).cmp(&(b.1.abs() + b.2.abs(), b.2, b.1))
    }));
    let mut best = None::<(f64, isize, isize)>;
    for &(_, dx, dy) in candidates.iter().take(12) {
        if let Some(score) = image.score(dx, dy)
            && best.is_none_or(|current| score > current.0)
        {
            best = Some((score, dx, dy));
        }
    }
    let (score, dx, dy) = best?;
    if score < 0.72 || dx.abs() == LIMIT || dy.abs() == LIMIT {
        return None;
    }
    // Rescore only a bounded number of spatially separated coarse peaks.
    let mut peaks = vec![(dx, dy)];
    let mut competing = f64::NEG_INFINITY;
    for &(_, x, y) in &candidates {
        if peaks.iter().any(|&(px, py)| (x - px).abs() <= 2 && (y - py).abs() <= 2) {
            continue;
        }
        peaks.push((x, y));
        if let Some(candidate) = image.score(x, y) {
            competing = competing.max(candidate);
        }
        if peaks.len() == 13 {
            break;
        }
    }
    (score - competing >= 0.03).then_some((dx, dy))
}

#[derive(Clone, Copy)]
struct Registration<'a> {
    base: &'a [[u8; 4]],
    moving: &'a [[u8; 4]],
    width: usize,
    height: usize,
    stride: usize,
}

impl Registration<'_> {
    fn score(&self, dx: isize, dy: isize) -> Option<f64> {
        let x0 = (-dx).max(0) as usize;
        let y0 = (-dy).max(0) as usize;
        let x1 = self.width.saturating_sub(dx.max(0) as usize);
        let y1 = self.height.saturating_sub(dy.max(0) as usize);
        if x1.saturating_sub(x0) * y1.saturating_sub(y0) < self.width * self.height / 2 {
            return None;
        }
        let (mut n, mut sa, mut sb, mut aa, mut bb, mut ab) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for y in (y0..y1).step_by(self.stride) {
            for x in (x0..x1).step_by(self.stride) {
                let (mx, my) = (x.wrapping_add_signed(dx), y.wrapping_add_signed(dy));
                let a = self.base[y * self.width + x];
                let b = self.moving[my * self.width + mx];
                if a[3] == 0 || b[3] == 0 {
                    continue;
                }
                let a = luma(a);
                let b = luma(b);
                n += 1.0;
                sa += a;
                sb += b;
                aa += a * a;
                bb += b * b;
                ab += a * b;
            }
        }
        if n < 16.0 {
            return None;
        }
        let va: f64 = aa - sa * sa / n;
        let vb: f64 = bb - sb * sb / n;
        if va < n * 4.0 || vb < n * 4.0 {
            return None;
        }
        Some((ab - sa * sb / n) / (va * vb).sqrt())
    }
}

fn luma([r, g, b, _]: [u8; 4]) -> f64 {
    0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)
}
