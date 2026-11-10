# Proposal: auto-align-layers

## Why

Issue #582: `Edit > Auto-Align Layers` is the last missing command in the
`LAY-030` align/distribute family. Align, Distribute, and Auto-Blend Layers
shipped, but Auto-Align has no command, engine, dialog, or bridge; the menu
entry is absent and the parity register records `LAY-4` as `MISSING`. The
feature is what makes hand-held panoramas, focus stacks, and scanned offsets
usable before blending or stacking.

## What Changes

- `pictura-render` `layer_ops/auto_align/` (new): a deterministic, dependency-free
  matcher. Harris corners, rotation-invariant SIFT-lite descriptors, ratio-test
  matching, RANSAC translation/similarity/homography estimation, and
  cylindrical/spherical projection spaces. All seven CS6 projections are
  implemented: Auto, Perspective, Cylindrical, Spherical, Scene Collage,
  Reposition Only, and Lens Correction. Auto runs Perspective and Cylindrical
  and keeps the better overlap score.
- Reference selection follows `LAY-030`: a locked layer is the reference,
  otherwise the layer whose content centre is nearest the union centre. Groups,
  adjustment/fill/vector/Smart Object layers, and the Background are skipped;
  position-locked layers never move. One invocation is one history state.
- Lens Correction reuses the existing lens-correction model and installed
  profiles. EXIF identifies the camera/lens; the correction is baked into each
  layer before matching. With no matching profile the pixels are left unchanged
  and the run reports why.
- `cxxqt_object/auto_align.rs` (new bridge): a cancellable worker over a document
  clone, progress 0–1000, a stale-document guard, and one `edit` commit named
  "Auto-Align Layers".
- `AutoAlignDialog` (new C++ dialog): seven projection radios and the
  Lens Correction sub-options (Vignette Removal, Geometric Distortion). The
  choice is remembered for the session. `Edit > Auto-Align Layers…` is wired in
  `frame_menus_align.cpp` beside Auto-Blend.
- Tests: engine unit tests for the matcher, reference rule, projections, lens
  correction, cancellation, and determinism; Qt Test `tst_auto_align` for menu
  enablement, dialog defaults/restoration, one history state with undo,
  cancellation, stale runs, and selection after commit.
- Docs: tick `ROADMAP.md`, mark `LAY-4` shipped in the parity register, update
  the Edit menu checklist and `STATE.md`.

## Capabilities

### New Capabilities

- `document/auto-align-layers`: content-based alignment of selected layers with
  the seven CS6 projections, reference selection, Lens Correction, and the
  dialog/worker surface.

## Impact

- `pictura-render` (new `auto_align` module, one transform-map hook),
  `pictura-app` (bridge, C++ dialog, menu wiring, Qt test), command catalogue,
  generated command ids, CMake source lists, and docs.
- No new dependency. The matcher uses `std`, `rayon` (already present), and the
  existing blur/resample/lens-correction code.

## Provenance

Behavior from `docs/05-layers/align-and-distribute.md` (`LAY-030`) and the CS6
Help. Adobe's feature detector, matcher, thresholds, and "better composite"
metric are unpublished, so this is **behavioral parity only**: the tests assert
recovery within a pixel tolerance, never byte identity. The projection maths
are standard panoramic formulas; the focal length is inferred from the selected
content because the document model carries no lens calibration.

Ceiling (`ponytail:`): destructive resampling (the document model has no live
layer transform), so the result is baked on apply; lens profiles come from the
same installed set as Filter ▸ Lens Correction; very large documents are
downsampled to a bounded work grid for matching.
