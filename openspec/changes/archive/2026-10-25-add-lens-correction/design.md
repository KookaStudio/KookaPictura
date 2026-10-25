# Design: add-lens-correction

Adobe's lens models, profile format, and resampler are closed; everything
below is an approximation with behavioural parity only.

## The map

Each output pixel centre `p` is mapped back to the source, with `c` the layer
centre and `R` the half-diagonal:

1. **Scale.** `q = c + (p − c) / scale`.
2. **Angle and perspective.** The forward homography is
   `H = Rot(angle) · T · K·Rx(a)·Ry(b)·K⁻¹`, `K` a pinhole camera with focal
   length equal to the diagonal, `a = −30° · vertical / 100`,
   `b = −30° · horizontal / 100`, and `T` the translation that puts the tilted
   centre back on `c`. The inverse is applied; a point on or behind the
   horizon is blank. Positive Vertical narrows the top, positive Horizontal
   narrows the right side, and a positive Angle turns clockwise. Rotation is
   applied last, in output space, so Straighten's measured deviation is
   exactly the angle to undo.
3. **Remove Distortion.** `s = c + d·(1 + k1·(|d|/R)²)`, `k1 = −0.3 ·
   value / 100`: positive values sample nearer the centre at the edges, which
   removes barrel distortion. `|k1| ≤ 0.3` keeps the map monotone to the
   corners.
4. **Auto Scale and the profile.** With a profile whose distortion is not
   zero and Auto Scale on, the point is divided by the smallest zoom (found
   by bisection over frame-edge samples, 0.25–4) at which the profile's map
   keeps the whole frame inside the layer; the profile's `k1 k2 k3` are then
   applied the same way. Auto Scale answers the profile only, as in CS6
   captures: a Custom rotation still leaves blank corners.
5. **Fringe.** Each colour channel samples `c + (s − c)·k_ch`, `k_ch = (1 +
   0.005 · slider / 100) · profile_ch`: Red/Cyan scales red, Green/Magenta
   green, Blue/Yellow blue. Alpha samples at the green point; red and blue
   are re-weighted to it.

## Colour

- **Vignette.** Gain `1 + amount/100 · t²`, `t = clamp((r − 0.8·mid/100) /
  (1 − 0.8·mid/100), 0, 1)` on the source radius over `R`; a lower midpoint
  starts the falloff nearer the centre. A profile's falloff
  `1 + v1 r² + v2 r⁴ + v3 r⁶` is divided out.
- **Sampling.** Premultiplied Catmull-Rom with the taps clamped to the layer;
  results are clamped so colour never exceeds alpha.

## Edge

Coverage is how far the geometric source point is inside the layer, plus
half a pixel, clamped to 0–1; the result blends with the Edge fill by it.
Transparency clears (white on a buffer without alpha, as Adaptive Wide Angle
does; the preview always shows it clear); Black and White fill opaque; Edge
Extension clamps the point to the layer. All-default settings return before
resampling, so they are bit-identical.

## Profiles

A text format, std-only (`[profile]` blocks of `key = value`: `name`,
`make`, `model`, `lens`, `distortion`, `ca`, `vignette`), parsed once from
`profiles.txt`. Matching is case-insensitive: the make must match, then a
matching model scores 2 and a matching lens 4; an empty criterion matches
all. The bundled profile is synthetic and matches the fictitious camera in
the codec's `metadata.psd` fixture, so real photos report "No matching
profiles found". Adobe's `.lcp` files are licensed and stay out of the tree.

## Slots

`lens-correction` carries 16 slots: Geometric Distortion, Chromatic
Aberration, Vignette (the profile checkboxes; defaults on, off, off), Auto
Scale Image (on), Edge (0 Extension, 1 Transparency default, 2 Black, 3
White), profile id (0 none, n the n-th bundled), Remove Distortion, the
three fringes, Vignette Amount, Midpoint (50), Vertical, Horizontal, Angle,
Scale % (50–150, 100). A profile id that is not installed refuses the slots.
The grid's show/size/colour/offset are view state, not slots.

## Settings

Default Correction resets, and Previous Conversion restores from the last
OK, the distortion, fringe, and vignette sliders; the transform is per image
and is never restored by Settings (CS6 does not save it). Previous
Conversion lives for the process.

## Depth gate

`depth_refusal(kind, doc)` replaces the 8-bit-only check: Adaptive Wide
Angle and Vanishing Point need 8-bit RGB or Grayscale; Lens Correction 8- or
16-bit. The menu row's enabled state and every commit path ask the same
function. The working planes are 8-bit; a 16-bit read keeps its depth note.
