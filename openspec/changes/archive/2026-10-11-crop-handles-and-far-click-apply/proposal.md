# Proposal

## Why

The Crop tool's on-canvas chrome and its outside-the-box interaction diverge
from Photoshop CS6. The crop box is drawn with eight centred 6x6 squares
instead of CS6's L-bracket corners and flat edge bars, and the rotate zone
extends unbounded from the box edge, so a click far from the box never applies
the crop the way CS6 does.

## What Changes

- Draw the crop box handles as CS6 does: four L-shaped corner brackets and four
  thin edge bars, all **inside** the box outline. Hit-testing and cursors keep
  their current border-centred grab zones.
- Bound the outside-the-box rotate ring to **8 px (inner margin) to 100 px
  (outer margin)** in screen pixels, at any zoom, in both the Modern preview and
  the active crop states.
- Outside the 100 px ring, show the workspace default arrow and let a left press
  **apply the crop** — including when the point falls on the workspace past the
  canvas edge. A far press in the preview state remains a no-op.
- The perspective-crop quad and slice overlays are unchanged; their handles and
  cursors are out of scope.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `tools/crop-straighten`: the crop rotation gesture's outside zone gains a
  100 px outer bound and a far press that commits; the pointer-cursor zones gain
  a far default-arrow zone; a new requirement fixes the crop box handle
  rendering (L-bracket corners, thin edge bars, drawn inside the box).

## Impact

- `crates/pictura-app/cpp/crop_grip.h` — bounded rotate zone and a far-zone
  predicate.
- `crates/pictura-app/cpp/tool_crop.cpp` — far-zone press commits; a new
  default-arrow cursor kind.
- `crates/pictura-app/cpp/image_view_overlays.cpp` — `paintCropGroupOverlays`
  handle drawing.
- Tests: `cpp/tests/tst_crop_grip.cpp`, `cpp/tests/tst_crop_cursor.cpp`, plus a
  new handle-glyph unit test.
- `docs/03-tools/crop-tool.md` — handle description and the commit triggers.
- No new dependencies.
