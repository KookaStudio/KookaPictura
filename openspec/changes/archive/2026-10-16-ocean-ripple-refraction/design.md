# Design: ocean-ripple-refraction

## Model

Tuned by eye against CS6 Filter Gallery renders of one photograph at
(Size, Magnitude) = (9, 9) at 43 %, and (2, 12), (14, 2) and (15, 20) at about
91 %.

- **Surface.** Lattice values come from `ChaCha8Rng(seed)` and lie in −1..1.
  They are interpolated with smoothstep on a `cell = 3 + 0.5·size` px grid.
  The displacement is the surface's analytic slope `(∂H/∂u, ∂H/∂v)` in
  lattice units, so each bump acts like a small lens.
- **Reach.** `0.15·magnitude^1.5·√(cell/4)` px per unit of slope. CS6 barely
  moves the picture at Magnitude 2 (about 2 px), frosts it at 9–12, and
  scatters fragments about 30 px at 20. A linear reach could not match both
  ends.
- **Rejected.** The slope of *bilinear* noise gives diamond and rectangle
  artefacts that CS6 lacks. Larger cells (`4 + size`) made blobs about twice
  CS6's size at Size 15.

## Limits

At Size 15 / Magnitude 20, fold-overs inside each lens show thin ring
contours, while CS6's blob interiors are softer and its fragments fly a
little farther. No pixel parity is claimed.
