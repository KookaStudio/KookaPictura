# Design

## Context

See `proposal.md` for motivation. The current model:

- `Layer::clipping` is the PSD clipping byte: `false` is a base, `true` is a
  non-base ("clipped") layer. PSD stores no base reference; the base is the
  nearest non-clipped sibling below the run.
- The CPU compositor already treats a clipped layer at the bottom of a container
  as unclipped (`composite_clipping.rs`), but the flag and the Layers panel glyph
  remain set.
- `create_clipping_mask` / `release_clipping_mask` only flip the byte. No reorder,
  delete, Reverse, ungroup, or Background path touches it.

### Behavioral evidence

- Adobe help, *Create and manage clipping masks*: multiple layers must be
  consecutive; a layer created or dragged **between** layers in a mask joins it;
  Alt-click removes the upper layer; Release removes the selected layer and all
  above it.
  <https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/create-and-manage-clipping-masks.html>
- Pretty Presets, *How to Use a Clipping Mask in Photoshop* (2022): dragging a
  clipped layer **below the base** or **above a layer that does not belong**
  releases it without affecting the rest; dragging the **base** out releases all
  subordinate layers, and moving them back does not re-clip. *(inferred from a
  practitioner walkthrough; no CS6 oracle render.)*
  <https://www.lightroompresets.com/blogs/pretty-presets-blog/how-to-use-clipping-masks-photoshop>
- Photoshop feedback, *Clipping Mask base layer removal releases the Clipping
  Mask chain* (2021): deleting the base breaks the chain; the requested
  "promote the layer above" is a feature request, confirming release is current
  behavior.
  <https://community.adobe.com/feature-requests-713/p-clipping-mask-base-layer-removal-releases-the-clipping-mask-chain-1228870>
- psd-tools: `Clipping` is `0 = base, 1 = non-base`; "Clipping is positional: the
  layers clipped to this one are not carried with it."
  <https://psd-tools.readthedocs.io/en/latest/reference/psd_tools.api.layers.html>
- Photoshop scripting reference: `ArtLayer.grouped` is "is the layer grouped with
  the layer below?" — a per-layer positional property, not a stored base link.
  *(The quote is from the Photoshop scripting reference; the current API exposes
  the same property.)*

Sketchbook documents the opposite ("move a clipping mask ... it will adjust to
reference the new base layer"). That is a different application and is not the
parity target here.

## Goals / Non-Goals

**Goals:**

- Make clipping membership follow stack position after moves, reparenting,
  deletion, Reverse, ungroup, and Background conversion.
- Release a run when its base leaves; do not silently re-base it.
- Keep release persistent when layers are reordered back.
- Clear a requested clipping flag when a node is inserted at the bottom of a
  container, so insertion cannot create a baseless clip.
- Keep the Layers panel glyph, indent, and base underline correct by fixing the
  underlying flag only.

**Non-Goals:**

- Duplicating a layer inside a clipped group. Photoshop has a reported quirk that
  releases the clip above the source; psd-tools' `copy_to` keeps the copy's own
  flag. Our duplicate inserts above the source and cannot create a baseless clip;
  leave it unchanged.
- Making a *newly created* layer join a run when it is inserted between a base and
  its clipped layers. The docs say it joins, but this change is reorder-driven;
  only the bottom-insertion guard is in scope. Track full insertion membership
  separately if parity is wanted.
- Grouping a clipped layer. Wrapping it puts the child at the bottom of a new
  container; this is a containment change, not a reorder, and Photoshop's
  group-clipping behavior is not established by the sources above.
- Merge commands, which have their own clipping handling.
- Normalizing on PSD import. Files are preserved as authored; a stale flag is
  corrected on the next covered edit.
- Reproducing Sketchbook's re-base-on-move behavior.

## Decisions

### Normalize in the render layer, not the panel

The flag is document state: it affects compositing, undo, and PSD write. Fixing it
in the Layers panel would leave the model wrong for saves and other consumers. The
panel already derives the glyph from `Layer::clipping`, so no UI change is needed.

### Two small helpers in `layer_ops/clipping.rs`

1. `release_run_above(container, index, was_clipped)` — called **before** removing a
   node. If the departing node is unclipped and the layer directly above it is
   clipped, clear the maximal contiguous clipped run above it. Those layers were
   clipped to it; Photoshop releases them rather than promoting the next layer.
   The call is a no-op when the layer above is not clipped. An unclipped node is
   not necessarily a base; the run check is what makes it one.
2. `normalize_clipping(container, moved: Option<Moved>)` — called **after** a
   container edit. `Moved` carries the final `index`, `was_clipped`, and
   `same_base_below` (the node landed directly above the same base it was
   clipped to before the move):
   - return without touching anything when the container is empty;
   - clear `container[0].clipping`;
   - if `moved` is present and in range, set its flag from its final neighbors:

     ```
     above = container.get(index + 1).is_some_and(|l| l.clipping)
     below = index > 0 && container[index - 1].clipping
     clipped = index > 0
            && (above || (was_clipped && (below || same_base_below)))
     ```

   The first clause means "dropped under a clipped layer -> joins that run"; the
   second means "already clipped and still inside a run"; `same_base_below` keeps
   a clipped layer that is the top of a one-layer run, e.g. `[A, B(clip)]` moved
   to the top, where neither neighbor is clipped but B is still directly above
   its former base A. Otherwise the node is released. `index == 0` always
   releases. Because `above` wins, a clipped layer dropped under another run's
   clipped layer joins that run even when the layer directly below it is a
   non-member.

**Alternative considered:** recompute every flag in the container from scratch.
Rejected: an unclipped layer inside a run may be a deliberate base, and a flag
represents intent for layers that did not move. The design only rewrites the moved
node and the container bottom.

`was_clipped` and the former base MUST be read before removal. For a
same-container move, derive the base with `base_below(container, index)` and map
its index across removal/insertion:

```
base_after = if at <= base { base + 1 } else { base }
same_base_below = at > 0 && at - 1 == base_after
```

A cross-container move passes `same_base_below = false`; the former base is in a
different container. `move_path_to` MUST pass the final clamped insertion index
(`at`), not the pre-removal `dest_index` or `None`.

### Call sites

| Site | Normalization |
|---|---|
| `properties.rs::move_path` | `release_run_above` before removal; `normalize_clipping(Some)` after insert |
| `properties.rs::move_path_to` | `release_run_above` on the source before removal; `normalize_clipping(None)` on the source after removal when the parent changes; `normalize_clipping(Some)` on the destination after insert |
| `properties.rs::delete_paths` | `release_run_above` before removal; `normalize_clipping(None)` after removal |
| `properties.rs::reverse_paths` | `normalize_clipping(None)` after reversing |
| `properties.rs::ungroup_paths`, `create.rs::ungroup_layer` | `release_run_above` on the parent before removing the group; `normalize_clipping(None)` after splicing |
| `create.rs::background_from_layer` | `release_run_above` on the source; `normalize_clipping(None)` on the source after removal; clear the moved node's own flag; insert at root 0; `normalize_clipping(None)` on the root |
| `create.rs::insert_node` | clear `node.clipping` when the chosen insertion index is 0 |
| `impl_layers.rs::move_layer` (legacy top-level bridge) | route through `move_path` so it cannot bypass the invariant |

`delete_hidden_layers` and `remove_layer` route through `delete_paths`, so they are
covered. The source container must be normalized from the handle obtained after
removal, before insertion, because insertion can shift path indices.

### Release, not promote, on a departing base

The Photoshop feedback thread shows users asking for the layer above to become the
new base, which means current Photoshop releases instead. This also matches
"clipping is positional: the layers clipped to this one are not carried with it."
Ungrouping a group that was a base is the same event: the base node disappears, so
the run above releases before the children are spliced.

### Release persists because no rule re-flags an unclipped moved node

After a run is released, its layers are unclipped. Moving them back above the base
does not set their flags: the moved-node rule only joins when the layer above is
clipped, and after release none are. A layer re-joins only by being dropped
directly under a still-clipped layer (inside a run). `release_run_above` is the
other writer, and it only clears flags.

### Reverse only clears a baseless bottom

No authoritative Photoshop rule describes Reverse for a clip run. Flags travel
with their layers; the only invariant enforced is that the new bottom is not
clipped. A reversed run may therefore re-base to the nearest non-clipped sibling.
This is documented rather than guessed.

### The legacy `move_layer` bridge now shifts

`PictureView::move_layer(i, delta)` currently swaps `i` with `i + delta`. Routing
it through `move_path` makes it remove-and-insert (shift) and clamps at the
container bounds, so a delta that used to be refused can now succeed by clamping.
The control server's `layer_op move` is the only caller. With `|delta| == 1` the
result is identical; with `|delta| > 1` the shift is the intended "move by delta"
result. The bridge test pins both cases and the locked/Background refusal.

### Insertion at the bottom clears the requested clip

`insert_node` can choose index 0 when the selection is an empty group.
`add_layer_full` applies the dialog's "Use Previous Layer to Create Clipping
Mask" flag, which would otherwise create a clipped bottom layer. Clearing it at
insertion is the same no-base rule and closes the only insertion path in scope.

## Risks / Trade-offs

- **`move_path_to` is also used by the stamp path** → a stamp dropped inside a run
  will now join it. This is the documented membership rule; `stamp_scope` gets a
  regression test that pins the landing position and clipping state.
- **Dragging a base out now releases its run** → users who expected the run to
  re-base will see the clip released. This matches Photoshop and is the point of
  the change.
- **Reverse can re-base** → accepted; no evidence for a stronger rule, and
  releasing the whole reversed run would be a second semantic change.
- **The legacy `move_layer` delta semantics change for `|delta| > 1`** → accepted;
  the shift is the method's documented intent and the common ±1 path is
  unchanged. The bridge test covers both.
- **Empty containers and top-of-container moves** → both helpers use `first_mut`
  / `get`, and edge tests cover moving the only child out of a group and moving a
  clipped layer to the last index.
- **Grouping a clipped layer and creating a new layer *inside* a run remain
  outside this change** → grouping can still make a baseless child, and a new
  layer between a base and its run still re-bases the run. Recorded as
  follow-ups, not hidden.
- **Multi-delete ordering** → `release_run_above` changes flags only, never
  indices, so the selected-path list remains valid.

## Migration Plan

No data migration and no PSD format change. Existing documents with a stale
clipped bottom are corrected by the next covered edit; import does not normalize
them. Rollback is a normal revert of the change; no persisted state depends on it.
