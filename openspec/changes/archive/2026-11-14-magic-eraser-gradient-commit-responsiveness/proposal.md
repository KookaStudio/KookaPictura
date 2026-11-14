# Magic Eraser and Gradient commits scale with their work

## Why

Issue #1191 measured two tools on a new 17000×17000 white document
(`scripts/profile-tools.py --size 17000x17000`): the Magic Eraser click took
**9 193 ms** and the Gradient release **4 720–5 115 ms**, with undo restores of
**2 453 ms** and **2 266 ms**. Photocraft took 4 795 ms and 2 866 ms on the same
gestures.

Both gestures cover the whole canvas by design, so bounding them to a box does
not help. The cost is per pixel in the fill/erase pass and in the history
restore. Three specific costs are visible:

- The Magic Eraser records `Edit::Full`, so every click captures and recomposites
  the whole document even when the flood erased one small region.
- `magic_erase` forks and visits every tile the mask touches, even when the mask
  and the target tile are uniform; a whole-canvas erase to transparency spends
  its time allocating and walking tiles that end up `Uniform(0)`.
- Unlocking a Background, or giving a layer an alpha channel, allocates a dense
  flat plane, so the first erase of a new white document materializes the whole
  alpha plane before it can write anything.
- The Gradient's per-pixel path runs the full Normal blend arithmetic for the
  common opaque, full-opacity case, where the result is simply the source
  colour.

## What Changes

- **Magic Eraser is a bounded edit.** It records the rectangle its flood erased
  instead of `Edit::Full`, so a partial erase captures and refreshes only that
  region. A styled document still recomposites whole, because a layer style can
  spread the change.
- **Magic Eraser is a job.** On an armed view and a document at or above the
  document-job threshold, the flood, anti-alias and erase run on a worker as a
  `document_op` with the shell's cancellable progress window. Cancel leaves the
  document and history unchanged, and Edit ▸ Fade is recorded only when the job
  lands.
- **Uniform tiles erase without pixels.** When a target tile and the mask over
  it are uniform, `magic_erase` writes the tile's new value with
  `set_tile_uniform` instead of forking and visiting its samples. The result is
  byte-identical to the per-pixel erase.
- **Uniform alpha planes.** The alpha channel added by `release_background` and
  `ensure_alpha` is a tiled uniform plane, not a dense allocation, so the erase's
  uniform fast path applies from the first click.
- **Direct opaque Normal fill.** The shared fill pass writes the source colour
  directly when the mode is Normal, the strength is at least 1 and the source is
  opaque, instead of running the full blend. Gradient, Paint Bucket, Fill and
  Stroke all share the pass; the result MUST equal the blend.

## Capabilities

### New Capabilities

- `tools/magic-eraser-gradient-commit-responsiveness`

### Modified Capabilities

None. The existing Magic Eraser and Gradient behaviour is unchanged; this
change adds the cost contract those capabilities rely on.

## Impact

- `pictura-paint`: `eraser.rs` (uniform-tile erase and uniform alpha),
  `fill.rs` (opaque full-strength Normal fast path).
- `pictura-render`: `document_ops/layer_ops/create.rs` (`release_background`).
- `pictura-app`: `paint_tools/imp.rs` (Magic Eraser region edit and job),
  `impl_core/document_job.rs` (a post-land hook for Fade), and
  `cpp/tool_eraser.cpp` (run the click through `runDocumentCommand`).
- Tests: `pictura-paint` unit tests, `pictura-render` layer tests, and
  `tst_tool_commit`.
