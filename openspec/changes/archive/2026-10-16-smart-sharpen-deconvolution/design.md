# Design: smart-sharpen-deconvolution

## Measurement

The source JPEG and two Photoshop screenshots of the Smart Sharpen result
(Gaussian r 30.3, amount 378 %, noise 27 %; Lens r 9.9, amount 405 %, noise
36 %, both with Shadow/Highlight fades) were registered with an affine fit.
The screenshots are scaled by 0.975, and the residual is 1.2 px. The radial
transfer function was then estimated from the cross-spectrum against the
warped source.

Clipping hides much of the gain, so a non-parametric radial gain was fitted
through the same `clip → warp → measure` pipeline. The Gaussian shot gives
about 1.9 from f = 0.014 to 0.09 cycles/px, rising to 10.4 at f = 0.46. The
Lens shot shows a bump near f = 0.085, the first Airy minimum of a disc of
about the radius, and the same rise.

A plateau of `1 + a·0.24` followed by a rise is `1/H − 1` for
`H = W·core + (1 − W)·halo` with W ≈ 0.6. Above `1/radius` the halo term
vanishes, leaving `1/W`, and the core's inverse supplies the rise. A grid fit
of W, the core spread, and the halo scale against both screenshots gives
W = 0.62, c = 0.17, and scale 0.8. The fit includes the tonal fades and
clipping, and uses the same factorized inverse the code runs.

## Algorithm

`H⁻¹ ≈ core⁻¹ · (W + (1 − W)·halo)⁻¹`. The two factors commute away from
the borders.

- **Core:** `[c, 1−2c, c]` equals `(1 − p z⁻¹)(1 − p z)/(1 − p)²`, with `p`
  the root of `z² + ((1−2c)/c) z + 1` inside the unit circle (p ≈ −0.28).
  The exact inverse is a causal pass, an anticausal pass, and a `(1 − p)²`
  scale, run on rows and then on columns. Seeding each pass with the
  steady state of a constant extension matches clamp-to-edge.
- **Halo:** `x ← x + β(y − W x − (1 − W)·halo(x))` from `x₀ = y`, with
  β = 1.25. The halo response lies in about `[0.54, 1]` (the disc's and the
  line's negative lobes set the low end), so each iteration contracts every
  frequency by at least 3×. DC has zero residual from the start, so a flat
  image stays bit-exact.
- **Noise and fades:** unchanged, except the floor scale (10, not 26). With
  26 the floor would suppress the fine detail Photoshop keeps at 27–36 %;
  10 matched best while Reduce Noise still visibly holds back noise.

## Alternatives rejected

- **Unsharp mask at σ = radius:** the best σ still scored 21.6 against the
  Gaussian shot, versus 18.0. It lifts every scale equally, so it can't make
  the plateau-then-rise shape.
- **Wiener deconvolution of a pure Gaussian:** the regularized inverse falls
  to zero at high frequency, the opposite of the measured rise. It scored
  57–119.
- **FFT:** there is no FFT crate in the workspace, and padding a large
  document to a power of two costs gigabytes. The spatial core IIR is exact
  and `O(1)` per pixel. The halo iterations are `O(1)` for the separable
  Gaussian, and `O(radius)` for the disc and the line.

## Limits

The constants come from two extreme settings on one image. Adobe's
algorithm is closed, so this is behavioral parity from those screenshots
only. A controlled sweep (radius 1/5/20 × each removal, amount 100 %, noise
0, no fades) would pin the radius scale and the noise floor.
