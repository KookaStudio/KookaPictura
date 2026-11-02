# Proposal

## Why

`docs/06-filters/liquify.md` states CS6's rule (acceptance criterion 8): with an
active selection, Liquify previews and processes only the selection's
**bounding rectangle**, and for a rectangular marquee the preview and the
selected area are identical. The implementation ignores the selection when it
opens the session: `liquify_begin` builds a layer-sized mesh and source, and
`liquify_render` shows and distorts the whole layer. A warp drawn anywhere in
the dialog therefore moves pixels outside the selection.

The commit is partially protected today only because the generic filter path
blends its result through the selection's coverage mask. That is the selection's
**shape**, not its bounding rectangle, so a lasso or ellipse leaves pixels
inside the rectangle unprocessed — which contradicts the documented rule and
makes the "Selection" mask mode a no-op only for a rectangular marquee.

## What Changes

- **The mesh carries a processing rectangle.** `LiquifyMesh` gains an optional
  `clip` in layer coordinates. Nodes outside it take and keep no offset, and
  `displacement` is zero there, so every consumer of the mesh (preview,
  resample, Show Mesh, kept meshes) agrees without re-deriving the selection.
- **The session opens on the selection's bounds.** `liquify_begin` intersects
  the selection's bounding rectangle with the active layer and builds the mesh
  with that clip. An empty selection, or one that misses the layer, refuses the
  dialog.
- **The preview shows only the rectangle.** `liquify_geometry` reports the clip
  size and a new `liquify_origin` reports its top-left in layer coordinates; the
  dialog canvas sizes, centres, zooms, traces its mesh, and crops its backdrop
  to that frame. Pixels outside the clip render clear.
- **The commit processes the whole rectangle.** A clipped mesh is self-confining,
  so `apply_filter_region` does not also blend through the selection's coverage
  mask. The selection's shape only matters through Mask Options, as CS6
  documents.
- **Kept meshes keep their rectangle.** `scaled_to` scales the clip with the
  mesh; loading a kept mesh into a session replaces its clip with that
  session's, so the current selection always wins.

Not in this change: freezing from a selection or channel (the Mask Options
source feature, tracked separately), and smart-filter support for Liquify
(Liquify is not smart-capable).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `imaging/liquify-filter`: a new requirement confines the session, preview,
  commit, and kept meshes to an active selection's bounding rectangle, and
  refuses a selection that misses the layer. The existing mesh, tool, and
  freeze-mask requirements are unchanged.

## Impact

- `crates/pictura-filters/src/distort/liquify.rs` and `liquify/{resample,view}.rs`.
- `crates/pictura-render/src/filter.rs` (a clipped Liquify mesh is
  self-confining; no coverage mask).
- `crates/pictura-app/src/cxxqt_object/filter_tools/liquify.rs` and
  `liquify/imp.rs`, `cpp/liquify_canvas.{h,cpp}`.
- Tests: `crates/pictura-filters/src/distort/liquify/tests.rs`,
  `crates/pictura-app/src/cxxqt_object/filter_tools/liquify/imp.rs`,
  `crates/pictura-app/cpp/tests/tst_liquify_dialog.cpp`.
- Docs: `docs/dev/STATE.md` only; the behavioural contract already states the
  rule.
- No new dependencies, no file-format change (meshes are process-local; Liquify
  is not a smart filter).
