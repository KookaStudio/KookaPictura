# Design: glass-filter

## Model

Tuned by eye against six CS6 Filter Gallery renders of one photograph:
(Distortion, Smoothness, Texture, Scaling) = (3, 3, Frosted, 66),
(10, 6, Frosted, 66), (10, 11, Frosted, 162), (17, 11, Blocks, 162),
(19, 15, Canvas, 162), and (19, 15, Tiny Lens, 162).

- **Refraction, not Displace.** Displace-style mapping, where one grey value
  shifts x and y equally, streaks the Tiny Lens render sideways. Following
  the map's slope instead turns each dome into a small lens holding its own
  piece of the picture, as CS6 shows. The same model drives Ocean Ripple.
- **Reach.** `1.4·Distortion·relief / rms` px per unit of slope. `rms` is the
  slope's RMS over a fixed 256² square of the same surface at the same
  Smoothness and Scaling, so the strength does not depend on the picture's
  size. `relief` is 0.6 for Frosted, 0.45 for Canvas, and 1 for Blocks and
  Tiny Lens; CS6's Frosted and Canvas bend less at equal Distortion.
- **Smoothness.** A Gaussian blur of σ = `0.4·Smoothness·Scaling/100` px.
- **Surfaces** (sizes at 100 %):
  - Frosted is two octaves of value noise at 6 and 13 px.
  - Tiny Lens is hemispherical domes 16 px apart on a 45° lattice.
  - Blocks is staggered 40 px courses of random height, pre-blurred by a
    quarter of a block. Hard block edges gave a grid of pillows; CS6's Blocks
    swirls.
  - Canvas is a sin² weave with a 24 px pitch, under value noise. An |sin|
    weave creased into stripes.
- Bilinear sampling, clamp-to-edge, alpha untouched. Invert negates the
  reach.

## Limits

Blocks is softer and less fragmented than CS6's, and Frosted at 162 % is a
little stronger. Large Smoothness × Scaling makes the blur long: Blocks at
200 % softens with σ = 20, a 121-tap separable pass.
