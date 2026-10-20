/// The turn at each corner of a quad (cross products of consecutive edges):
/// all the same sign for a convex quad, mixed for a concave or self-intersecting
/// one.
pub(super) fn quad_turns(quad: &[(f64, f64); 4]) -> [f64; 4] {
    std::array::from_fn(|k| {
        let (a, b, c) = (quad[k], quad[(k + 1) % 4], quad[(k + 2) % 4]);
        (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)
    })
}

/// Whether `quad` is convex with orientation `sign`, every corner turning by
/// more than `min_turn`.
pub(super) fn quad_is_convex(quad: &[(f64, f64); 4], sign: f64, min_turn: f64) -> bool {
    quad_turns(quad).iter().all(|turn| turn * sign > min_turn)
}

/// Move from `from` towards `to` as far as the quad stays convex. A box that is
/// not convex to begin with is left free.
pub(super) fn keep_convex(from: [(f64, f64); 4], to: [(f64, f64); 4]) -> [(f64, f64); 4] {
    let turns = quad_turns(&from);
    let sign = turns[0].signum();
    let min_turn = turns
        .iter()
        .map(|turn| turn.abs())
        .fold(f64::INFINITY, f64::min)
        * 1e-3;
    if !(min_turn.is_finite() && min_turn > 0.0 && quad_is_convex(&from, sign, 0.0))
        || quad_is_convex(&to, sign, min_turn)
    {
        return to;
    }
    let lerp = |t: f64| -> [(f64, f64); 4] {
        std::array::from_fn(|k| {
            (
                from[k].0 + (to[k].0 - from[k].0) * t,
                from[k].1 + (to[k].1 - from[k].1) * t,
            )
        })
    };
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if quad_is_convex(&lerp(mid), sign, min_turn) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lerp(lo)
}
