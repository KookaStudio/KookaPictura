# Proposal

## Why

A Spot Healing Brush click with a brush about the size of a dot leaves a grey
smudge instead of removing the dot (#1023). A drag removes the dot but leaves a
faint trace near detail such as a mane edge. Both come from how Proximity Match
fills the hole:

- The hole is exactly the brush footprint, so the dot's soft rim sits just
  outside it. The fill takes that dark rim as its fixed boundary, and the result
  is grey.
- The fill is a Laplace solve: a smooth interpolation of the boundary. Next to
  a high-contrast edge the interpolation smears the edge into the hole.

`docs/03-tools/healing-brushes.md` ("Spot Healing source search") specifies
that Proximity Match searches the area around the brush for the best-matching
source and then runs the heal solve. The code skips the search.

## What Changes

- Proximity Match and Create Texture rebuild a margin past the brush edge, sized
  from the brush footprint. The fill's boundary therefore lies outside the
  defect's soft rim, and the margin fades out so it leaves no seam. Content-Aware,
  the Healing Brush, and Patch are unchanged.
- Proximity Match searches the area around the hole for the offset whose
  surroundings best match the hole's surroundings, and whose interior is
  closest to the smooth fill. It then transplants that source with the same
  gradient-domain solve as the Healing Brush. When no source fits inside the
  image, it falls back to the smooth fill.
- The healed rectangle grows by the margin.

No new dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/healing-brushes`: the healing engine's Proximity Match searches for a
  source and heals past the brush edge; Create Texture heals past the brush edge;
  a Spot Healing click with a brush the size of a dot removes the dot.

## Impact

- Paint: `pictura-paint::healing` (`mod.rs`, a new `proximity.rs`, tests).
- App: a Qt Test case in `tst_retouch_tools`; no bridge or UI change.
- Behaviour: a Proximity Match or Create Texture heal now changes pixels up to
  the margin outside the brush footprint.
