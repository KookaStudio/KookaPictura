# Proposal

## Why

A live preview of a canvas-sized change recomposites the whole document at full
resolution on the GUI thread. Every Properties-panel slider or curve tick on an
unmasked adjustment layer takes the `Edit::Full` path. Every canvas-sized opacity
or blend change on a pixel layer goes through `mutate_layer`/`changed_property_region`,
and every canvas-sized visibility change goes through `set_visible_paths_union`;
both composite the whole canvas through `Edit::regions`. Groups and unmasked
adjustment layers take `Edit::Full`. Either way the whole document is composited
at full resolution: the full path runs `composite_rgba` over every layer, a
whole-doc `buffer_to_srgb`, a whole-doc composite store, and a whole-document
`ViewPyramid::rebuild` (`impl_core.rs:882-941`); the region path composites the
whole canvas and updates every stored level (`impl_core.rs:1034-1061`). The
choice is made in `edit.rs:258-317`. On a 4000² eight-layer document that is
roughly 128 million blended pixels per tick for a change the user is watching at
25% zoom.
The compositor already has a regional path, a reduced-document proxy, and a view
pyramid that the canvas presents; the preview path does not use them at the
presented level.

## What Changes

- Add a render session that presents a live preview from the smallest frame the
  canvas actually shows: the visible document rectangle composited at the
  view-pyramid level the canvas presents, with the document below the edited
  layer reused as a cached backdrop.
- Present that preview into the display levels only: it patches only the
  presented rectangle of the level-0 display frame (and the presented stored
  level above 0). It does not update `doc.composite` or any display pixels
  outside that rectangle; the exact full composite is produced by the commit (or
  an explicit refine) and replaces the approximate frame.
- Coalesce preview requests with the single-shot timer pattern the Layers panel
  already uses, so at most one preview composite runs per presented frame,
  however many slider events arrive.
- Cache the backdrop below the edited layer at the preview's resolution, keyed
  by a digest of that layer prefix, and invalidate it by the key rather than by
  edit notifications. Resume only at a structurally valid boundary; fall back to
  the ordinary full composite otherwise.
- Modify the current-picture contract so a display-only preview is one of the
  states where the maintained composite cannot stand in, and modify the damage
  contract so a display-only preview is not treated as a committed pixel edit.

Non-goals, recorded so the review surface stays bounded: a document-sized
full-resolution backdrop (16 bytes per pixel, 256 MB at 4000²), a stroke-frame
backdrop, and a `RenderSession` abstraction that owns display state. Whether the
commit's exact composite runs on a worker is decided by the profile task, not
assumed here. The stroke-frame backdrop and the off-thread refine remain tracked
by issue #1015; this change does not close it.

## Capabilities

### New Capabilities

- `compositing/render-session`: a live preview composites only the visible
  region at the presented level from a cached backdrop below the edited layer,
  is presented into the display levels only, is coalesced per frame, invalidates
  by prefix digest, and is replaced by an exact full composite at the commit.

### Modified Capabilities

- `compositing/current-picture`: a display-only preview does not leave the
  maintained composite current, so the current picture is composited while one
  is live.
- `compositing/described-damage`: the regional contract applies to a live
  preview that updates the document composite and to the commit; a display-only
  preview is outside it.

## Impact

- `crates/pictura-render`: a new render-session module (backdrop slot, prefix
  digest, viewport/level preview, exact refine entry point); the region
  compositor gains a seeded "resume at layer k" entry point. No Qt dependency.
- `crates/pictura-app`: `PictureViewRust` owns the session; `adjustment_edit.rs`
  presents a preview instead of `Edit::Full`; `edit.rs` gains the display-only
  preview outcome; `impl_core.rs` runs the exact commit/refine;
  `helpers_composite.rs` patches a presented level; `filter_proxy.rs` is reused
  for the reduced document.
- `crates/pictura-app/cpp`: `ImageView` reports the presented level and the
  visible document rectangle; the Properties panel coalesces preview requests
  with the single-shot timer pattern the Layers panel uses.
- Tests: the render oracle suites gain backdrop/refine equivalence cases, the
  app gains preview/invalidation/commit tests, and `tests_profiles.rs` gains
  ignored Curves-drag and opacity-drag profiles at fit and at 100%.
- No new dependencies. No PSD/codec/GPU surface change.
