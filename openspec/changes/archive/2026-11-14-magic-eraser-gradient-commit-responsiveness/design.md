# Design

## Magic Eraser's edit

`magic_erase` already returns the document rectangle it changed. The app threw
that away and recorded `Edit::Full`. The click now returns
`Edit::regions(vec![dirty], true)` through the same helper the fills use
(`fill_edit`): a document with a layer style still returns `Edit::Full` because
the style can spread past the erased pixels; every other document refreshes and
captures only the dirty rectangle. The history hint is the same rectangle, so a
partial erase does not compare the rest of the layer.

Unlocking a Background adds an alpha channel, which changes the tracked-plane
geometry. The history's `diff_into` already starts a new segment for that
change, so the bounded edit does not lose the metadata.

## Magic Eraser as a job

`runDocumentCommand` arms the view and shows the cancellable progress window,
exactly as it does for the four fills (#1300). `magic_erase_at` checks
`document_job_wanted()` after its layer checks and, when the document is large
enough, starts a `document_op`. The worker's clone is the document the engine
edits: it unlocks the Background, floods from the layer or the composite, builds
the anti-aliased mask and calls `magic_erase`. The outcome carries the dirty
rectangle. A cancel drops the worker's document.

Edit ▸ Fade is the one thing that cannot be recorded inside `land`: the edit
transaction clears the fade record before it applies the pixels. `document_op`
therefore gains an optional post-land callback, run after the edit has been
applied and captured, which installs the pre-effect layer that was captured on
the GUI thread before the job started. The synchronous path runs the same
callback after its edit.

## Uniform-tile erase

`magic_erase` iterates the tiles the mask reaches. For each tile it can prove is
fully inside the canvas and uniformly covered, and whose target tile is
`Empty` or `Uniform`, it computes the one result value and calls
`set_tile_uniform` instead of adding the tile to the dense list. A tile that is
dense, partly outside the canvas, or partly covered still takes the existing
per-pixel path, so the result is unchanged and a no-op still reports no damage.
A whole-canvas erase to transparency therefore touches tile metadata only.

## Uniform alpha

`release_background` and `ensure_alpha` used `Plane::build`, which allocates a
flat plane. They now use `Plane::filled`, whose every tile is `Uniform(255)`
with no allocation. That is the same bytes with a representation the erase fast
path can collapse.

## Direct opaque Normal fill

`Pass::blend_one` runs for every fill pixel. When the mode is Normal, the
effective strength is at least 1 and the source alpha is 255, the Normal blend
is exactly the source colour: the source's alpha is 1, so the backdrop cannot
contribute. The pass writes that colour directly and keeps the destination's
alpha for `Alpha::Keep`, or the source's for `Alpha::Replace` and `Blend`. The
generic blend still handles every other combination.

## Measured

`scripts/profile-tools.py --size 17000x17000 --scenarios magic-eraser,gradient`
on the optimized `dev-app` build, same host, before and after this change:

| Scenario | Before | After | Speed-up |
|---|---:|---:|---:|
| Magic Eraser total | 286.9 ms | 58.6 ms | 4.9× |
| Magic Eraser undo | 13.4 ms | 18.1 ms | — |
| Gradient total | 2 363.3 ms | 2 140.5 ms | 1.1× |
| Gradient undo | 11.4 ms | 16.8 ms | — |

The issue's 4.7–5.1 s Gradient baseline predates the tiled-fill and job work
already on master; current master commits the Gradient in about 2.36 s, and
this change does not regress it. The remaining Gradient time is the first
17000² canvas frame, not the fill. The Magic Eraser's 4.9× is the uniform-tile
erase and the bounded history capture. The undo numbers are both under 20 ms;
the small increase is history-hint bookkeeping, not a regression in wall time.
