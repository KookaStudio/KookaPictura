# Design

## Shared bounds

`AdjustmentDialog` already samples a downscaled canvas histogram when a
hand-laid dialog opens (`takeHistograms`): 0 luminosity, 1 red, 2 green, 3 blue.
The new helper walks those bins with the same rule the engine's
`pictura_adjust::auto::percentile_bounds` uses:

```
total = sum(bins)
cut   = floor(total * 0.001)          # 0.1% at each end
lo    = first bin whose cumulative count exceeds cut
hi    = last bin whose reverse cumulative count exceeds cut
```

It returns `false` when the histogram is empty or `hi - lo < 2`, so a flat or
single-level image is a no-op rather than a division by zero. This mirrors the
existing Levels guard.

## Button wiring

`AdjustmentDialog` gains a protected `virtual void applyAuto()`. The two places
that build CS6's action column (`buttonColumn`) and bottom row
(`bottomButtons`) connect a button labelled `Auto` to it with a lambda, so the
virtual override is the one that runs. Levels keeps its right-hand column;
Curves puts Auto in its bottom row; the stacked slider layout used by
Brightness/Contrast passes `Auto` as an extra button instead of a disabled one.

## Per dialog

- **Levels.** Unchanged behaviour: reset input black to 0, set input white to
  the white bound, then set input black to the black bound. The temporary reset
  keeps the engine's black-below-white rule satisfied. The preset drops to
  `Custom`.
- **Curves.** Replace the selected channel's points with
  `"<lo>,0 <hi>,255"`. Two points satisfy the engine's 2..14 rule, and the
  Hermite kernel clamps below `lo` and above `hi`, so this is the Auto levels
  stretch expressed as a curve. An identity result is still written; it is
  visually a no-op.
- **Brightness/Contrast.** Use the composite (luminosity) histogram because the
  adjustment applies one map to all channels. Let `mid = (lo + hi) / 510` and
  `gamma = ln(mid) / ln(0.5)`, so the modern `y = x^(1/gamma)` curve moves the
  occupied midpoint to 0.5. Clamp `brightness = 150 * log2(gamma)` to the
  engine range and recompute the effective gamma from the clamped value. Map
  the endpoints through that gamma, then choose the S-curve central slope
  `s = 0.5 / max(0.5 - lo', hi' - 0.5)` and `contrast = 100 * (s - 1)`, clamped
  to `-50..100`. The S-curve fixes `(0,0)`, `(0.5,0.5)`, `(1,1)`, so this
  expands the range without promising that the endpoints land exactly on 0 and
  255 — Adobe's fit is closed and the docs mark Brightness/Contrast Auto as
  behavioural parity only.

## Parity note

No golden oracle exists for Adobe's closed Auto fit. The tests assert the
observable contract: the shared bounds are the 0.1% clip, flat images are
no-ops, and each dialog moves its controls in the documented direction.
