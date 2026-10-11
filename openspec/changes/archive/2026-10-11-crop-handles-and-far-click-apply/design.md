# Design

## Context

See proposal.md for motivation. Today the crop box geometry lives in
`crates/pictura-app/cpp/crop_grip.h` (`cropRotateZone` has only an inner resize
margin of 8 px and no outer bound), the press/cursor routing in
`crates/pictura-app/cpp/tool_crop.cpp` (`onPress`, `cursorStateAt`, `hoverCursor`),
and the drawing in `crates/pictura-app/cpp/image_view_overlays.cpp`
(`paintCropGroupOverlays`, the loop that paints eight centred 6x6 squares).
`cropRotateZone` is shared with the Slice Select header but only called by the
Crop tool and its tests.

## Goals / Non-Goals

**Goals:**
- Bound the outside-the-box rotate ring to a 100 screen-pixel outer margin.
- Apply the crop on a press beyond the ring, from canvas or workspace.
- Draw CS6-style L-bracket corners and thin edge bars inside the box.
- Keep the change testable with the existing Qt Test harness.

**Non-Goals:**
- Perspective Crop quad handles and cursor (separate capability).
- Slice Select handles.
- Free Transform's outside behaviour (#310) — a different tool.
- A draggable crop pivot (#309).

## Decisions

- **Two zone predicates, not a new classifier type.** Keep `cropRotateZone`
  but bound it to `(inner, outer]`, and add `cropFarZone(box, p, zoom)` for
  `> outer`. Both reuse the existing clamp-to-box distance math. A single
  `enum` classifier was considered but would churn every call site for no
  behaviour gain. `kCropFarMarginPx = 100.0` sits beside `kCropRotateMarginPx`.
  Both are screen pixels divided by zoom, so the band is constant on screen.

- **Apply on press, in the tool.** In `onPress` Active, the branch order becomes
  handle (`boxGripAt`) → body `Move` → rotate ring → `commitCrop()`. Committing
  on press (not release) matches Photoshop and avoids a drag from a no-gesture
  zone doing nothing then acting on release. The preview state keeps its current
  behaviour except that a far press returns without starting a rotate.

- **A distinct `CursorKind` for the far zone.** Add `CursorDefault` rather than
  reusing `CursorBox`, so the far-zone tests pin the exact zone and the two
  states stay independently changeable. `hoverCursor` maps it to the same
  workspace arrow already used for `CursorBox`.

- **Extract handle glyph geometry for testing.** Painting is a loop with
  branches, so per the repo's definition-of-done it needs a runnable check. A
  pure helper (widget-space box → corner bracket lines + edge bar rects) is
  unit-tested; `paintCropGroupOverlays` only strokes it. This also keeps the
  draw loop short and `tool_crop.cpp` under its 800-LOC target by leaving zone
  math in the shared header.

## Risks / Trade-offs

- [Existing spec and tests assert an unbounded rotate zone] → The delta
  rewrites `lcs_rotate_past_canvas` and updates `tst_crop_grip.cpp` /
  `tst_crop_cursor.cpp`; the far points in those tests sit under 100 px, so only
  the explicit `(200, 200)` case flips.
- [A 100 px ring is large relative to a small box on screen] → Acceptable: the
  ring is measured from the nearest box edge and matches the requested CS6
  behaviour; no aspect-ratio or snapping interaction changes.
- [Rendered brackets are hard to assert pixel-exactly] → Test the geometry
  helper, not the painted pixels; visual regressions stay a review concern.
- [Far press on a box equal to the canvas] → `commitCrop()` no-ops (no pending
  crop), the same as an Enter with nothing to do; no special-casing needed.

## Migration Plan

No migration: single app, no persisted format or API change.

## Open Questions

None.
