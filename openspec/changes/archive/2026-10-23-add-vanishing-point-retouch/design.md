# Design: add-vanishing-point-retouch

Adobe's retouch internals are closed. What follows is built on part 1's
plane geometry and is an approximation, not verified parity.

## Flat editing space

- **Root planes.** A root plane's flat coordinates are its frame
  coordinates divided by the family's metric unit, so one flat unit is about
  one pixel on a square-on plane.
- **Related planes.** A related plane's frame coordinates `(s, t)` map
  affinely into its parent's flat space: `s` along the hinge in the parent's
  own coordinate, and `t` along the parent's in-plane outward direction. The
  plane unfolds flat, keeping distances from the hinge.
- **Image ↔ flat.** Each plane carries a homography from its frame rect to
  its image quad, and the affine map into flat space.
  - **Image → flat:** use the plane containing the pixel (the edit's own
    plane first, then the newest).
  - **Flat → image:** use the plane whose unfolded rect contains the point,
    or extrapolate through the edit's own plane.

## Working copy and undo

- **The working copy.** The session holds the active layer's pixels as
  straight RGBA in the layer's frame. Every pixel edit writes into it.
- **Undo steps.** A step records the dialog's non-pixel state (the planes,
  the floating selection and the marquee) plus, for a pixel edit, the
  changed rect's previous bytes. Undo and redo swap those bytes with the
  working copy, so one swap does both.
- **The preview.** The preview composes three layers in document space: the
  document with the active layer hidden, then the working copy (Normal, at
  full opacity), then the floating selection. Only dirty rects are recomposed
  and handed to the canvas.
  - ponytail: the layer's blend mode and opacity are ignored in the preview.
    The commit honours them, since it edits the layer.

## Floating selection

- **Representation.** A texture, its family, a centre in flat space, a
  scale (flat units per texel), an angle, and Flip / Flop flags.
- **Rendering.** For each pixel of the footprint: image → flat → texel, a
  bilinear sample, times Opacity and the Feather ramp, all inside the
  family's planes.
- **Dropping.** Dropping composites it into the working copy through the
  Heal blend.
- **Paste placement.** One texel per flat unit, shrunk to 80 % of the
  plane.

## Heal

The blend runs over the drop or stroke footprint, where the mask is the
coverage:

- **Off:** `dest·(1−m) + src·m`.
- **On:** Gauss-Seidel on `Δf = Δsrc` inside the footprint, with `f = dest`
  on its border. This keeps the source's texture while matching the
  destination's colour and lighting, with no seam.
- **Luminance:** the same solve on luminance only. The source's chroma is
  kept and re-lit to the solved luminance.

Painting a mask below 1 blends the healed result in by the mask.

## Strokes

- **Dabs.** Dabs are spaced at a quarter of Diameter along the stroke in
  flat space. A dab covers the image pixels whose flat point lies within its
  radius, with Hardness setting the inner fully-opaque radius.
- **Brush.** It paints Brush Color. In a Grayscale document the colour is
  reduced to its luminance.
- **Stamp.** It samples the working copy as it was at the start of the
  stroke, at the dab's flat point plus the source offset, mapped back to
  the image.
- **Opacity and heal.** Opacity caps a stroke's coverage. With Heal on,
  the stroke's coverage and sampled pixels are kept and blended when the
  stroke ends.

## Commit

- **The edit.** OK drops any floating selection, then compares the working
  copy with the layer's original pixels. The bounding rect of the changes
  becomes the edit's patch, beside Render Grids' lines.
- **The kernel.** `Filter::VanishingPoint` writes the patch, then draws the
  lines.
  - ponytail: kept edits stay in memory (eight of them).
  - ponytail: Last Filter on a layer of another size replays the patch at
    the same layer offset.

## Ceilings (`ponytail:`)

- The CS6 defaults for Diameter, Feather and Heal are not tabulated. The
  ranges copy the main brush tools.
- Heal applies when a selection is dropped or a stroke ends, not live while
  it is dragged.
- The preview ignores the selection. The commit clips to it.
