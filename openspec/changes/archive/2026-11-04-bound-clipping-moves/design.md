# Design

## Context

The damage module bounds a structural edit to the rectangles its affected layers
can influence. A clipped member previously reported the union of its own reach
and the clipped run above it. That is correct for a base, but a clipped member
only shows through its base's rendered alpha, so a full-canvas member forced a
full-canvas damage on every reorder inside the run.

The app already unions the before and after reaches of the moved path, so a
clipped member that leaves the run is reported unclipped on its side and a base
that moves keeps the conservative union with its run.

## Goals / Non-Goals

**Goals:**
- Bound a reorder inside a clipping run by the base's reach.
- Keep the released run covered when a base moves or a clipping mask is released.
- Skip the compositor for a move that cannot change the picture, without
  weakening the knockout guard.
- Keep the full recomposite when a live gesture or an uncommitted preview owns
  the display.

**Non-Goals:**
- Changing the compositor's clipping rules or the layer tree's re-basing rules.
- A new damage representation; the existing rectangle union is kept.
- Optimizing history capture for a neutral move.

## Decisions

### D1. Bound a clipped member by its base in `content_influence`

`content_influence` returns the base's `influence_of` when the layer is clipped
and a non-Background base exists. A Background base or an unbased run keeps the
old union. The base's reach is a superset of its rendered alpha, so the bound is
conservative, never smaller than the actual visible area.

`influence` keeps its knockout guard, so a knockout document stays unbounded for
a structural edit. An in-place content edit keeps using `content_influence`,
which is safe because it does not reassign a knockout base.

### D2. Damage the freed set on Release Clipping Mask

`release_clipping_mask` already computes the freed paths. Expose the same set as
`clipping_release_paths` and have the app pass the union of the clicked and
freed paths to `batch_changed`. `batch_changed` computes the before/after
influence over that union; after the release every freed path is unclipped, so
its own reach is included. The release op is idempotent on an already-unclipped
path, so passing the expanded list does not change the changed count.

### D3. Neutral moves are inert or hidden and never touch a base or a knockout

`move_is_content_neutral` reuses `layer_is_inert`, accepts a hidden node, and
requires that the node is not a clipping base before or after. It returns false
when either document contains a knockout, matching `duplicate_is_content_neutral`
and `damage::influence`. The app maps a neutral move to `Edit::Inserted`, which
skips the compositor but still clears the links and records history.

### D4. A pending preview upgrades `Edit::Inserted`

`apply_edit_with` already forces a full rebuild for `Edit::Structural` when a
live session, a pending preview, or a soloed mask is present. Extend the same
condition to `Edit::Inserted`: the display preview stores paths, and a move
shifts sibling indices, so the preview would otherwise point at the wrong layer.
The solo-mask case stays on the `refresh_for` repair path, which re-resolves the
solo source.

## Risks / Trade-offs

- A payload or edit whose base reach is larger than the base's actual alpha
  damages more than necessary. The bound stays conservative.
- A neutral move still records a history state and clears the links; only the
  composite is skipped.
- The clipping-mask release expansion adds a small path walk before the op; the
  op itself is unchanged.
