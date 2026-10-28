# Design

## Context

See `proposal.md` for motivation and `specs/` for the behavioral contract.

The current edit transaction has three relevant outcomes: `Edit::Regions`
(composite the given rects, one blit), `Edit::Full` (full recomposite), and
`Edit::Structural` (clear positional links, full recomposite). Structural
edits, mask-refine previews, type and shape edits, smart-filter toggles, and
`batch_changed` all route through `Full`/`Structural`.

Reach logic exists in `layer_visibility_region`, `changed_property_region`,
`layer_paint_bounds`, `layer_has_effects`, and the GPU mask-coverage assembly.
`layer_paint_bounds` already unions a group's descendants and grows a layer by
its effect reach; it does not model the clipped run above a layer, the mask
extent, mask density, feather, or vector masks on adjustments.

History capture (`History::capture_live` → `planes::diff_into`) compares every
tracked plane tile by tile. Stamps skip untouched planes. A region edit leaves
the composite and the edited layer's planes unstamped, so those are compared in
full even though the edit already declared its rectangles.

A structural edit can create a path, remove a path, or move one. Its before and
after path sets therefore differ; a path missing because the edit created or
removed it must not be treated as an unprovable reach. Any *unexpected* missing
path is different and stays unbounded. The compositor also has two global base
dependencies the layer-local reach does not see: a knockout layer's base is the
first root layer or its group's entry/inner backdrop, and a regional refresh can
leave a soloed-mask view stale because only the full rebuild re-resolves it.

## Goals / Non-Goals

**Goals:**

- One reach primitive used by visibility, property changes, filters, Layer Style
  previews, and structural edits.
- Structural edits composite only their influence; history is hinted only where
  tracked-plane geometry is stable. Other structural edits keep the existing
  full history segment.
- History capture compares only the tiles a bounded edit declares when the hint
  covers every change.
- No rendered pixel changes.

**Non-Goals:**

- Changing the compositor's output or the GPU path.
- Bounding operations whose reach is the document or a global base by
  construction: flatten, Merge Visible, stamp visible/selected, canvas-sized
  fill creation, duplicate-to-document, document geometry, mode/HDR conversion,
  unmasked adjustments, a knockout base change, and a soloed mask.
- Per-plane hints (a caller names document rectangles, not tracked planes).
- Splitting the display blit into several signals; a multi-rect commit still
  emits one blit over the bounding box.

## Decisions

### 1. `pictura_render::damage` owns reach

`influence(doc, path) -> Option<PsdRect>` returns `None` when the path does not
resolve on that side or when the reach cannot be proven, and a document
rectangle clipped to the canvas otherwise. It composes:

- `layer_paint_bounds` for content, group descendants, and effect reach;
- `mask.rect` and the vector-mask box, because a mask plane can extend past the
  layer's content box and a history hint must cover plane bytes as well as the
  composite;
- the contiguous run of `clipping` siblings directly above the path.

The clipped run is not an optimization detail. `LayerStack::remove` and
`move_node` call `release_run_above`: deleting or moving a non-clipped layer
unclips every clipped layer above it. Group and splice paths can make the
inserted node a new clip base. Without the run, those siblings change outside
the reported damage.

`influence_paths(doc, paths) -> Option<Vec<PsdRect>>` returns one rectangle per
path, or `None` if any path in that list does not resolve or is unbounded.
Overlapping rectangles are merged. Keeping rectangles separate matters for
align/distribute and reorders of distant layers: their union can be most of the
canvas while each rectangle is small.

Before/after path sets are one-sided:

- A path the edit creates is omitted from the before set and included in the
  after set.
- A path the edit removes is included in the before set and omitted from the
  after set.
- A path that moves or is regrouped is listed under its old path before and its
  new path after.
- A path listed on a side where it should resolve but does not is an unexpected
  missing path and is unbounded.

This lets delete, duplicate, group, ungroup, move, and insertion be bounded
without inventing a stable layer identity.

A knockout layer's base is global within its container: the first root layer at
the document root, or the group's entry/inner backdrop inside a group. If a
structural edit can change that base while a knockout layer exists in the same
stack, `influence` returns `None` (conservative). This covers root, pass-through,
and isolated-group knockout stacks.

Alternatives considered: a single union rectangle (simpler, but defeats
disjoint edits); computing reach from the operation's parameters inside the
transaction (needs a before-document snapshot and stable layer identity, which
the tree does not have).

### 2. Fix the existing under-bounds in the same primitive

`layer_paint_bounds` is only safe for an adjustment when the mask is enabled,
has data, has `default_color == 0`, has `density == 255`, and has no vector
mask. A density below 255 lifts an outside-mask sample of 0 to a nonzero value,
so the adjustment applies outside `mask.rect`; feather spreads it by
`mask_refine::reach`; an inverted vector mask can apply everywhere. The same
density hole exists in `layer_visibility_region`.

`damage::influence` implements the safe cases (bounded raster mask, density 255,
default 0, no vector mask, grown by feather reach) and returns `None` for the
rest. `mask_refine::reach` becomes crate-visible instead of duplicating the
formula. `layer_paint_bounds` is corrected in place so the Layer Style preview
also stops under-covering feathered masks.

`layer_visibility_region` is removed once its callers have a document and path.
The one exception is adjustment creation in `impl_filters.rs`, which currently
computes a region from a detached layer: insert the layer first, then compute
`damage::influence` for the created path. No layer-only influence helper is
needed.

Alternative considered: leave the old bounds and only add the run; rejected
because the property test would fail on feathered or low-density masks, and the
existing preview is wrong for them.

### 3. `Edit::Structural { damage: Option<Vec<PsdRect>> }`

`None` means full. `Some` means refresh only those rectangles through the
existing `refresh_regions` path, emit one blit, and clear links as today. The
transaction still bumps the content revision, captures one history state, and
emits one change notification.

Policy details:

- If the sum of the clipped disjoint damage rectangle areas is at least three
  quarters of the canvas area, use the full composite instead; the regional
  path has per-rect overhead and no benefit. Do not use the bounding box of
  disjoint rectangles for this measure.
- If a live gesture (stroke, placer, transform, or preview) is active, or
  `pending_previews.any()` reports an uncommitted opacity/fill/mask preview,
  force the full path and pass no history hint. A region refresh would skip the
  session cleanup and would let a hint miss the preview's pixels.
- If `mask_solo.is_some()`, force the full path. `sync_mask_solo_region` patches
  only the refreshed region and cannot re-resolve a mask whose path moved or
  clear a deleted solo target.
- An empty damage list is treated as `None` (full), not as a zero-rectangle
  region.
- `effect_of` returns `Blit { notify: true }` for `Some` and `Rebuild` for
  `None`, so the shell refreshes the layer panel and command registry exactly
  as it does for a region edit.

Alternative considered: a separate `StructuralDamage` variant. Rejected because
it duplicates the link-clearing meaning and future call sites would have to
choose between two near-identical variants.

### 4. Call sites compute before/after influence

Each bounded caller resolves the affected paths that exist on each side,
computes their influence before the mutation, runs the mutation, computes the
influence after, and passes the union. This is the pattern `layer_style.rs`
already uses for paint bounds. It needs no document snapshot and makes the
affected set explicit.

- Stack ops: moved/duplicated/grouped/deleted/arranged/reversed paths. The
  before list excludes a created path; the after list excludes a removed path.
  Batch duplicate includes the copies only after; batch delete includes the
  deleted paths only before.
- Type, shape, smart-object, rasterize, and warp: the same path before and
  after (the rectangle may change). `rasterize_all_layers` unions every
  fill/type path before and after.
- `batch_changed`: `influence_paths(doc, refs)` before and after; returns
  `Edit::Regions { notify: true }` because those operations change panel state
  but do not invalidate positional links today.
- Mask refine: capture the target layer's influence **before** the setter, run
  the setter, capture it **after**, and refresh the union on every preview and
  commit. A shrinking feather must repair the previous, larger reach. The
  commit notifies (`notify: true`); the preview does not. A commit that finishes
  an earlier preview passes no history hint unless it also unions the
  accumulated preview damage; the final influence alone does not cover a
  previously larger feather.
- Smart-filter toggles: the layer's influence; `Edit::Regions { notify: true }`.
- Merge: the pre-merge influence of the scope's resolved paths for Merge Down,
  Merge Selected, and Merge Clipping; Merge Visible and Flatten stay full.
- Non-neutral insertion: only the inserted path exists after the mutation; the
  content-neutral `Edit::Inserted` branch is unchanged.
- The following sites stay full in this change: stamp visible/selected,
  canvas-sized fill creation (`add_solid_fill`, `add_gradient_fill`,
  `add_fill_layer`), duplicate-to-document, document geometry, mode/HDR
  conversion, Flatten, Merge Visible, Pictura Raw, free transform, and other
  Structural sites whose path set or global base is not proven here.

If any affected path is unexpectedly missing or unbounded, the caller passes
`None`.

### 5. History hints are document rectangles

`capture_live_with(..., hint: Option<&[PsdRect]>)` and `record_hinted` sit
beside the existing `capture_live`/`record`, so the 34 existing callers do not
change. `record_move_with` is the move-commit form; `apply_edit_with` forwards
the hint on both the revision-bumping and move paths. `Edit::Regions` gains a
`hint` field via constructors, defaulting to its refresh rects. An empty refresh
list becomes `hint: None` and is treated as no bounded rectangle, so the
transaction uses the full path. `end_paint` overrides it with the whole stroke's
tile rectangles: for a non-previewed stroke the refresh rects are only the
pending region, while history must cover every region the stroke already
presented. `Edit::Structural` forwards its damage as the hint; empty damage is
no hint.

This is safe for structural ops because:

- `influence` ignores visibility and occlusion, so a layer plane rewritten by a
  type, rasterize, or filter-like edit stays inside the reported layer rect;
- operations that change plane count, stride, height, or order make `diff_into`
  return `None`, which starts a full segment before any hint is used;
- a bounded edit is never hinted while `pending_previews.any()` is true. A
  structural edit uses the full path (decision 3); a region edit still uses its
  region refresh but passes no history hint;
- the per-op and randomized tests compare hinted and unhinted deltas.

`planes::diff_into` gets an optional hint. To apply it, the plane visitor must
report a placement, not just `(plane, width, height)`:

- composite: origin 0, rows are channel blocks stacked vertically; reuse the
  existing `Damage::rows` mapping;
- canvas: document channels, origin 0; the hint applies to their canvas tiles
  too;
- layer channel: origin `layer.rect.left/top`;
- raster mask: origin `mask.rect.left/top`.

`changed_tiles` tests each tile's document rectangle against the hint and
compares only intersecting tiles. A partially intersecting tile is compared in
full. Tiles outside every hint are treated as equal and copied into the
materialized history state as if unchanged. `stamp_agreed` then marks them
equal, so a hint that misses a change is permanent corruption until a geometry
change. A hint is therefore a caller contract: it must cover every change since
the last captured state. A non-covering hint is a contract violation, not a
correctness claim; the randomized hinted-vs-unhinted test is an acceptance test,
not a nice-to-have.

## Risks / Trade-offs

- **Under-bounded damage leaves stale pixels** → every unprovable case returns
  `None`; the random structural property test compares full composites before
  and after, including clipping and knockout cases.
- **A bad history hint corrupts the delta silently** → only bounded edits are
  hinted; the per-op and randomized hinted-vs-unhinted tests compare the stored
  tiles; an operation without a proven plane bound stays unhinted; pending
  previews force no hint.
- **A clipped run changes outside the edited path** → influence includes the
  run; the property test generates clipping stacks.
- **A knockout base changes outside the edited path** → influence returns
  `None`; the property test generates a root-level knockout case.
- **Canvas-sized "bounded" damage is slower than full** → the area threshold
  falls back to `composite_full`.
- **A structural region refresh during a live gesture, pending preview, or
  soloed mask skips cleanup** → force full in those cases.
- **Disjoint rectangles emit one blit over their bounding box** → compositing
  stays per-rectangle; only the display copy covers the union. Accepted for this
  change; split the emit later if a profile shows it matters.
- **Most structural history captures still start a full segment** → the goal is
  limited to compositing and to hints whose tracked-plane geometry is stable;
  history plane sharing for structural edits remains issue #973.

## Migration Plan

No data migration. The change ships behind the existing edit transaction. If the
bounded path regresses a document, the call site can pass `None` to restore the
full path without reverting the primitive, and a call site can pass no hint to
restore full history capture. The OpenSpec change is archived in the same pull
request once the property, randomized hinted-vs-unhinted, and profile checks
pass.
