# Design: paintbrush-oval-wet-edges

## Context

Every Paint dab adds to a per-stroke coverage buffer `a`:
`a' = 1 − (1 − a)(1 − flow·tip)`. The stencil then composites `a·opacity`
from the saved stroke-start pixels. Because it starts from those saved pixels,
the painted alpha may go down as `a` rises.

## Measurements (owner's Photoshop screenshot)

The screenshot is at ~770 % zoom with the Pixel Grid on: one document pixel
spans 7.71 screen pixels. Sampling every cell's centre gives document pixels.
Coverage is `255 − grey` on a white canvas.

| Feature | Value |
|---|---|
| Single dab (isolated clicks) | flat 26, 1–2 px anti-aliased edge |
| Dab shape | ellipse 24 × 12 px, major axis +50° (up-right) |
| Stroke interior | flat 20 |
| Stroke rim | 1–3 px at 21–26 |
| Overlap | none above 27 |

At Opacity 35 % with `#47484a` (184 below white), 26 and 20 are alphas
0.141 and 0.109, or 40 % and 31 % of the opacity. With Flow 50 %, a single
dab leaves `a = 0.5`, and a dense stroke drives `a` toward 1.

## Decisions

- **Wet-edge transfer.** `f(a) = 1.3 · a · (1 − 0.76 · a)`, clamped to
  0..=1. It passes through both measured points (f(0.5) = 0.40,
  f(1) = 0.31) and peaks at a ≈ 0.66. The partially covered pixels at a
  stroke's edge sit near that peak, which produces the darker rim. The curve
  runs on the accumulated buffer before grain and the stencil.
- **Multiply.** Per W3C compositing:
  `Cs' = (1 − αb)·Cs + αb·Cs·Cb`, then source-over at the dab's strength. On
  an opaque backdrop this is `Cb + a·(Cs·Cb − Cb)`; on a transparent one it
  paints the colour as Normal does.
- **Preset.** 45 px, as the preset's name says. The reference dab measures
  24 px; the owner chose to keep the name's size. Hardness 100 matches the
  1–2 px edge. Roundness 50. Angle −50: in this engine a positive angle tilts
  the major axis down-right on screen (y down, θ = −angle), so −50 gives the
  reference's up-right tilt. Spacing 5 % (2.25 px), because at 15 % a 50 %
  flow leaves the interior uneven (21–25); at 5 % a simulated stroke paints
  `24 21 20 … 20 21 24` across, against the reference's `26 21 20 … 20 21 26`.
- **GPU and preview.** The GPU dab shader has no wet-edge curve, so a
  wet-edge stroke stays on the CPU path, as grain does. Multiply already has
  no GPU mode. The large-dab preview applies the curve to its coverage before
  blending, and presents Multiply as Normal, as it already does for Dissolve
  and Behind.
- **Scope.** Wet Edges is an engine flag that only presets set; there is no
  Brush-panel toggle. Only Multiply is added among the unmapped Brush modes.

`ponytail:` the curve is fitted to two tones from one screenshot, and the
angle sign convention is left as it is.

## Risks

- The angle sign differs from Photoshop's counter-clockwise convention. A
  later fix of the sign must flip this preset's −50 to +50.
- Persisted options gain a `wetEdges` key. Older files without it load as off.
