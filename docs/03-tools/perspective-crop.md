# Perspective Crop

- **Spec ID:** `TOOL-012`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Yes` — the Perspective Crop tool is a separate tool nested under the Crop tool. In CS5 perspective correction was a "Perspective" checkbox on the Crop tool.
- **Depends on:** `TOOL-011` (Crop Tool), `TOOL-017` (Ruler straighten), `01-architecture/document-model.md` (`ARCH-002`), `01-architecture/undo-history.md`.

## CS6 behavior

Perspective Crop transforms the perspective of an image while cropping. It is
used on images with **keystone distortion** — an object photographed from an
angle rather than straight on (for example, a tall building shot from ground
level, whose edges converge toward the top). Source: CS6 reference,
"Transform perspective while cropping | CS6".

Documented workflow (CS6):

1. Hold down the Crop tool and select the **Perspective Crop** tool.
2. Click the four corners of the distorted object in turn (top-left,
   top-right, bottom-right, bottom-left); the quad closes on the fourth click.
   Match the corners to the rectangular edges of the object. The object **must
   have been rectangular in the original scene**, or the transform will not be
   what the user expects. Each corner snaps to the canvas edges/corners within a
   small threshold, and the corners can be dragged afterwards; dragging inside
   moves the whole quad, and dragging outside it rotates the quad about its
   centre.
3. `Enter`/`Return`, or the options bar's apply button, completes the
   perspective crop; the cancel button or `Esc` discards the quad.

Kooka draws only the staged corners while the quad is incomplete, and shades
**only the canvas** (never the workspace) once the quad is complete, so a
partially staged quad never dims anything.

Additional CS6 facts:

- The perspective **Show Grid** option in the options bar displays the
  perspective grid used to align the vertical/horizontal lines of the subject.
  Source: Photoshop Essentials (perspective crop), CS6 options bar screenshot.
- `Front Image` is available; its shortcut changed from `F` (CS5) to `I` in
  CS6 for both Crop and Perspective Crop.
- A **Clear** button clears the W/H fields (and resolution field if shown).
- In CS5 the crop marquee center point had to stay in place to perform the
  correction ("Do not move the center point … The center point needs to be in
  its original position in order to perform perspective correction"). Whether
  this constraint carries into the CS6 separate tool is not stated in the CS6
  section — see Open questions.
- The CS5 procedure used a marquee with a **Perspective** checkbox and required
  extending side handles while preserving the perspective; the CS6 tool is a
  distinct tool but retains the same edge-matching interaction.
- Widely reported (community): Perspective Crop has no non-destructive option
  and always discards cropped pixels; auto-resampling is not exact, so users
  commonly re-run the crop or stretch the result vertically with Free Transform
  (a PSD round-trip could otherwise be used). Marked inferred.

### Interaction with the Crop tool

| Aspect | Crop tool (`TOOL-011`) | Perspective Crop (`TOOL-012`) |
|---|---|---|
| Toolbox slot | Crop | Nested under Crop; `Shift+C` cycles the group |
| Initial box | Placed automatically on select | Drawn by the user |
| Grid | Composition overlays (Thirds, Grid, …) | Perspective grid (`Show Grid`) |
| Transform | Rotation / straighten only | Projective (four-corner) correction |
| Non-destructive | Available (Delete Cropped Pixels off) | Not offered (inferred) |
| Commit | `Enter`, double-click, or button | `Enter`/`Return` |

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox (under Crop) | Tool | hold Crop / `Shift+C` | Separate CS6 tool |
| Options bar | W / H | — | Final output pixels; blank = auto from the quad |
| Options bar | Swap | — | Swap the output width and height |
| Options bar | Resolution / unit | — | Output resolution (ppi or ppcm) |
| Options bar | Show Grid | — | Perspective grid visibility |
| Options bar | Front Image | `I` | Sets the resolution from the front document; shortcut changed from F |
| Options bar | Clear | — | Clears the W/H output fields |
| Options bar | Apply / Cancel | `Enter` / `Esc` | Shown only while the quad is complete |
| Canvas | Four clicked corners | — | Click TL, TR, BR, BL; drag to match object edges |
| Canvas | Rotate outside | — | Drag outside the complete quad rotates it about its centre |
| Context | Commit | `Enter`/`Return` | Double-click inside also commits |
| Context | Cancel | `Esc` | |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Show Grid | Bool | On | — | Perspective grid overlay |
| Quad corners | 4 point pairs | From drawn marquee | Within image bounds | Define the homography |
| Crop W / H | Number | Empty | > 0 | Optional resample size |
| Resolution | Number | Empty | > 0 ppi | Optional |
| Front Image | Action | — | — | Adopts another document's W/H/resolution |
| Clear | Action | — | — | Clears W/H/resolution |
| Commit / Cancel | Action | — | `Enter` / `Esc` | |

## Algorithms & pipeline

Behavioral description (the exact Adobe implementation is closed; the following
is the standard, well-known approach and is **inferred** unless noted):

- The user's four crop corners define a quadrilateral in source-image space. A
  rectangular object photographed at an angle projects onto that quad under a
  **projective transform** (homography). Recovering the rectangle is the inverse
  problem.
- Estimate the 2-D projective transformation `H` (an invertible 3×3 matrix,
  8 degrees of freedom after scale normalization) that maps the unit rectangle
  to the user's quad, then invert it to map source pixels back to a rectangle.
  A standard direct linear transform (DLT) from the four point
  correspondences, or a closed-form quad-to-rectangle solve, suffices.
- Because `H` is projective (not affine), it preserves straight lines but not
  parallelism or ratios: parallel lines may converge and cross-ratios along a
  line are invariant. This is exactly what removes keystone distortion.
- Output size is the crop box's pixel dimensions; each destination pixel is
  sampled from the source via the inverse homography plus interpolation
  (bicubic by default, consistent with General preferences).
- Only four corner correspondences are observable, so a quad can be mapped to
  any rectangle; the tool's result is therefore user-dependent and "not an
  exact science." Residual error produces the commonly reported vertical
  "squish," because the transform cannot recover information the camera did not
  record (depth/true proportions).
- **Show Grid** draws the homography's line family so the user can align grid
  lines with edges that should be vertical/horizontal. Kooka spaces the grid at
  one inch (the document resolution in pixels), clipped to the quad, so a larger
  quad shows more lines at the same physical spacing.

Implementation notes:

- Solve the homography once per commit; validate non-degenerate quads (no
  three collinear corners).
- Mark as "behavioral parity only, algorithm TBD" for the exact interpolation
  and the precise grid construction.

## Rust module mapping

Proposed:

- `pictura-core::geometry::homography::Homography` — 3×3 projective transform,
  `from_quad_to_rect(quad, w, h)`, `inverse()`, `map_point`.
- `pictura-core::tools::perspective_crop::PerspectiveCropState` — quad, grid
  visibility, output size, `commit()`.
- `pictura-core::render::warp::warp_projective(src, dst, H, sampler)` — tile-
  friendly inverse mapping sampler with CPU/GPU backends.
- Reuses `pictura-core::geometry::transform` and the Crop tool's commit
  plumbing (`TOOL-011`).
- Boundary types: `Quad`, `Homography`, `WarpKernel`, `CropRect`.

## Qt6 component mapping

- `PerspectiveCropTool` — draws the four-corner marquee, edge handles, and
  perspective grid; corner drags recompute the homography preview live.
- `PerspectiveCropOptionsBar` (`QWidget`) — Show Grid toggle, Front Image,
  Clear, W/H/resolution.
- `PerspectiveGridItem` — GPU-accelerated grid overlay (CPU fallback) drawn from
  the current homography.
- Live preview renders through the canvas's existing scene; commit sends the
  quad and output size to the Rust core.

## Data-model impact

- Perspective crop is destructive in CS6 (inferred): commit replaces the
  raster with the warped result and updates canvas dimensions. It should still
  be a single history state.
- If a future non-destructive variant is chosen, store a `PerspectiveQuad` on
  the layer/document alongside `CropRegion` (see `TOOL-011`); not required for
  CS6 parity.
- Undo record: before/after raster region plus transform parameters.

## Edge cases

- **Degenerate quads**: three collinear corners or zero-area quads must be
  rejected without corrupting the document.
- **Corners outside the canvas**: clamp or reject; behavior unverified in CS6.
- **Bitmap/Indexed modes**: color-mode restrictions may apply (inferred).
- **16/32-bit HDR and CMYK/Lab**: warp in the working space; preserve bit depth.
- **Very large (PSB) documents**: homography solve is O(1); the cost is the
  warp over up to 300,000 px per side — tile and cache.
- **GPU-unavailable**: CPU warp fallback with progress and cancellation.
- **Non-rectangular source object**: result is undefined by design; document
  the precondition in UI copy/tooltip.
- **Resampling error**: no automatic vertical correction; matches reported CS6
  behavior.

## Parity acceptance criteria

1. Given a frontal photo of a rectangular target with known corners, applying
   Perspective Crop to those corners yields a rectangle whose sides are
   parallel within 1 px at the measured scale.
2. Given a quad that is exactly a rectangle and axis-aligned, the committed
   output equals the plain crop of that rectangle (no resampling drift beyond
   1 LSB).
3. Given a quadrilateral whose corners are collinear (degenerate), the tool
   refuses commit and leaves the document unchanged.
4. Given a quad, straight lines in the source that were straight remain straight
   in the destination (projective invariant) — verified by edge-straightness
   measurement on a checkerboard test image.
5. Given the same input quad and output size, the homography and resampled
   output are deterministic byte-for-byte across runs.
6. Given `Esc`, no history state is added.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  "Transform perspective while cropping | CS6" (separate tool, keystone
  distortion, marquee matching, Enter to complete); "Transform perspective
  while cropping | CS5" (Perspective checkbox, center-point constraint,
  rectangular-object precondition); "Crop tool changes and enhancements"
  (`Front Image` shortcut F→I).
- `https://www.photoshopessentials.com/basics/perspective-crop-tool-photoshop` —
  Steve Patterson: Perspective Crop introduced in CS6, draw-the-marquee
  workflow, **Show Grid** option, aligning grid to subject edges, commit via
  checkmark/Enter, vertical "squish" and Free Transform workaround.
- `https://pixotter.com/blog/how-to-crop-image-in-photoshop` — "The Perspective
  Crop Tool always deletes cropped pixels -- there is no non-destructive
  option" (community blog; surfaced via search).
- `https://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/perspective_croptool.html`
  — "Perspective Crop tool … crop and correct converging verticals with a single
  crop" (search snippet; page not directly fetched).

## Open questions

- **Does the CS6 center-point constraint still apply** to the separate
  Perspective Crop tool? *Resolves with:* CS6 observation.
- **Exact arbitrary-quad-to-rectangle solve** Adobe uses. *Resolves with:*
  measuring behavior in tests; not documented. Kooka sizes the output from the
  quad's longer opposite edges unless the user types a W/H, which forces that
  dimension.
- **Is Perspective Crop strictly destructive in CS6**, and is Delete Cropped
  Pixels ever exposed? *Resolves with:* CS6 options-bar capture.
- **Grid construction** (1-point/2-point grid, line spacing) used by Show Grid.
  *Resolves with:* CS6 observation.
- **Whether perspective crop is recorded as an action step** and its serialized
  parameters. *Resolves with:* CS6 action/scripting docs.
