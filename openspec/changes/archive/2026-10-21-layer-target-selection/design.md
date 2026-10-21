# Design

## Context

See `proposal.md` for the motivation. The current state that shapes this design:

- `PictureViewRust` holds `active_layer: Option<String>`
  (`cxxqt_object/state.rs`), written by `LayersPanel::syncControls` only when
  exactly one row is selected, and written ad hoc by about eight bridge commands.
- `LayersPanel::refresh()` captures the tree selection and current path, resets
  the model, re-selects those paths, and pushes a single path back to Rust. The
  model reset and the filter proxy make the tree unable to hold a
  selected-but-hidden layer.
- `history::Snapshot` carries `doc` and the pixel `selection` only. Undo restores
  the document and the panel then re-selects stale path strings.
- The engine already has the set algebra: `selected_paths()` in
  `pictura-render/src/document_ops/layer_ops/paths.rs` dedups, resolves, drops
  selected descendants, and orders paths for safe removal.
- `#739` (edit transaction) and `#740` (plain-Rust document session) have no
  commits on their branches. There is no session type to move into yet.

## Goals / Non-Goals

**Goals:**

- One per-document layer target owned by Rust, with one write path and one read
  path for commands, tools, history, and the panel.
- The panel becomes a projection; filters and model resets cannot mutate the
  target.
- Structural operations define an explicit target outcome, and history restores
  it with the document.
- Keep the migration compiler-driven: existing invokables either keep their
  semantics or become thin shims.

**Non-Goals:**

- Extracting the `DocumentSession` (#740). This design leaves a `LayerTarget`
  value that #740 can move unchanged.
- Retiring the index-addressed invokables or changing the control-server JSON
  contract. That is a follow-up proposal for `interop/agentic-control`.
- Moving several layers with the Move tool in one gesture. The target exposes
  the set; the tool still consumes one layer.
- Stable layer identities. Paths remain the addressing scheme.

## Decisions

### D1. Store the target in `PictureViewRust` now

A new Qt-free module `crates/pictura-app/src/layer_target.rs` defines:

```rust
pub struct LayerTarget {
    selected: Vec<String>,
    active: Option<String>,
}
```

with `single()`, `anchor()`, `selected()`, `select_only()`, `select()`,
`clear()`, and `retain_resolving(doc)`.

`PictureViewRust.active_layer` is replaced by `layer_target: LayerTarget`.

**Why:** #740 does not exist, and this change unblocks #742 and #277. The type
has no Qt or QObject dependency, so #740 can move it into the session verbatim.

**Alternative rejected:** introducing a minimal `DocumentSession` now. It would
be a second seam that #740 immediately reshapes, and it adds migration cost
without a current consumer.

### D2. Paths, not stable IDs

The target stores panel path strings (`"2/1"`), sanitized against the live
document on write and after restore.

**Why:** paths are already the engine's addressing scheme, and the issue scopes
the change to paths. Adding `LayerId` would touch 301 `Layer { .. }` literals,
the codec, and every construction path. The target's transitions are explicit,
so IDs are not needed for correctness in the common flows.

**Alternative rejected:** a stable `u64` id on `Layer`. It would make undo and
remapping free, but it is a much larger data-model change and PSD carries no id.
Recorded as a future option if the positional-path ceiling below starts to
matter.

### D3. One target type, three derived roles

- **Edit target** = `single()`; non-empty only when exactly one path is selected.
- **Command target** = `selected()`; used by group, merge, delete, property
  batches, align/distribute.
- **Anchor** = `anchor()`; the panel's current row, used for insertion points,
  rename, mask thumbnails, and mask solo.

Invariant: `active ∈ selected ∪ {None}`; empty `selected` implies no `active`.
When sanitizing drops the anchor but other selected paths survive, the anchor
coerces to the first surviving selected path in document order.

### D4. Explicit per-operation transitions in the app bridge

Each structural bridge command sets the target immediately before `record()`.
The transitions are the ones in the `document/layer-target` spec. The engine
supplies the result paths where it already computes them (`MergeOutcome.path`,
`duplicate_paths`, `group_paths`, the `add_*` return value, and
`move_path_to_result` / `arrange_path_index` / `reverse_paths_result`).

Transition writes stay inline in the operation, before `record()`, rather than
using a shared closure: `batch_changed`, `merge_scope_changed`, and `flatten`
have different mutation shapes, and a generic post-mutation hook would add a
callback parameter to every call site for no shared behavior.

Ungroup needs a remap, not just a new set: selected layers outside the ungrouped
groups keep their selection while their indices shift, and the spliced children
take the group's former slot. The transition computes both sides from the
post-operation document. `ungroup_paths` may return the resulting paths (or the
bridge recomputes them from the ungrouped groups) rather than only a count.

**Why:** a generic remap needs identity. Op-specific outcomes are unambiguous and
match CS6 behavior (duplicate selects the copy, merge selects the result, delete
selects the neighbour). The engine remains about pixels and tree structure.

**Alternative rejected:** teaching `pictura-render` every op a target type. That
couples the engine to session state and changes dozens of signatures for no
reuse outside the app.

### D5. History carries the target; restore is conditional

Each history `Entry` gains `target_before` and `target_after`; the stored
`Snapshot`/`Delta` stay document-only. `capture_live`, `undo_live`, `redo_live`,
and `jump_live` take the live target so the transition can compare it with the
state being left.

On a walk from state `c` to `k`, using the captured post-operation targets:

```
if live_target == state[c].target_after -> live_target = state[k].target_after
else                                    -> live_target.retain_resolving(restored_doc)
```

`history_restore_snapshot` bypasses `restore_live`; it restores the snapshot's
target explicitly (same conditional rule is unnecessary there: a named snapshot
restore is an explicit restore of that state). `history_add_snapshot` and the
`snapshot()` helper capture the live target.

The existing `document/edit-history` requirement for the pixel `selection` is
unchanged; the new `document/layer-target` capability owns the layer-target
restore behavior.

**Why:** restoring unconditionally would revert a layer selection the user made
after the captured edit, which Photoshop does not undo. Keeping the later
selection avoids that. When the target was untouched, the destination state's
target is exact, so a structural undo lands on the restored layer rather than on
whatever now occupies the old path.

**Ceiling:** if the user changes the selection after a structural edit and then
undoes that edit, the kept paths are only sanitized, not remapped. Mark with a
`ponytail:` comment naming stable IDs as the upgrade path. The spec's guarantee is
limited to the unchanged-target case.

### D6. The panel writes on user intent and applies on refresh

Split the two directions:

- User selection/current-row change -> `set_layer_target(paths, anchor)`.
- Refresh, open/switch, and a filter change -> `applyTargetSelection()`, which
  only sets the tree selection from the target; it never writes back.

`refreshing_` (or a dedicated `applyingTarget_` guard) suppresses the write while
any of these run: `refresh()`, `applyFilter()`, `enterIsolation()`,
`releaseIsolation()`, `setView()`, and `applyTargetSelection()`.

`applyFilter()` calls `applyTargetSelection()` after `proxy_->setFilter()` so a
hidden selection is represented as "no visible selected row" while the target
survives, and clearing the filter reselects the visible target rows.

`refresh()` loses both its capture block (`selected` / `selectedPath`) and its
re-select block (`selectPaths(selected, selectedPath)`); it keeps no fallback to
the first displayed row. The rename editor's target comes from
`layer_selection_anchor()`.

`set_layer_target` no-ops when the value is unchanged. Target writes that change
the layer selection emit the view's existing `changed` signal (or rely on the
panel's own refresh); there is no second target-specific signal. `setView`
replaces the panel's `changed` connection, so a previous document's target can
never be applied.

The public `selectPaths()` remains for tests and simulated clicks and writes the
target under the same `applyingTarget_` guard as a refresh; the tree-only path is
private.

**Why:** this is the loop that currently reverts Rust writes. The proxy also
makes a hidden selected row unrepresentable, so the panel cannot be the source.

### D7. Panel commands and properties read the target

`LayersPanel::propertyPaths()`, the blend handler, the header `active` gate, and
the blend/opacity/fill eligibility read `selected_layer_paths()`. Kind and value
lookups still go through the source model (`model_->indexForPath`), which retains
filtered-out rows.

Selection-based panel actions in `layers_panel_actions.cpp`,
`layers_panel_menu.cpp`, `layers_panel_mask_menu.cpp`, `panel_group_menu.cpp`,
and `layers_panel_events.cpp` read the target, not `selectedPaths()`. Row-targeted
menu actions keep their explicit clicked path; when a menu action makes a row the
current row (CS6 right-click behavior), it writes the target first.

`syncControls()` stops calling `set_active_layer`; clearing the mask edit target
when the anchor changes moves into `set_layer_target`.

**Why:** filtering must not change which layers a command edits, and menus must
work headless where the tree is not authoritative.

### D8. Compatibility shims and C++ readers

`set_active_layer(path)` becomes `select_only(path)`; an empty path clears the
target. `active_layer_path()` returns `single()`. Both keep their signatures so
the control server and Rust tests migrate without a flag day. Callers that used
`set_active_layer("")` to disable tool edits while keeping a panel selection now
clear the target; those tests are updated to the new invariant (one selected
layer always has an edit target).

The write APIs have distinct jobs:

- `set_layer_target(paths, anchor)` is the panel's user-intent write; it is
  silent and no-ops when unchanged.
- `select_layer_path(path)` replaces the target with that one layer and emits
  the view's `changed` signal; tools and commands use it.
- `clear_layer_target()` empties the target and emits `changed`; Select >
  Deselect Layers uses it.
- Both Rust-originated writes clear a mask edit target whose layer is no longer
  the anchor.

The 33 non-test C++ calls to `active_layer_path()` are classified too. Tool gates,
crop snapping, smart-guide references, mask solo, the character panel, and layer
drag that need the current row use `layer_selection_anchor()`; only tool-edit
gates use `active_layer_path()`.

### D9. Open initializes the target before the opening history state

`open_jobs` currently captures the opening history state before
`apply_opened` assigns the active layer. The opening target is computed from the
decoded document in the worker (topmost pixel layer, else the topmost top-level
layer) and carried on `OpenedDocument`; `history_for` takes it, and
`apply_opened` assigns it after `reset_edit_state()`. The same ordering applies
to new documents, imports, image operations, and mode conversion.

**Why:** otherwise the opening snapshot carries an empty target and undoing the
first edit clears the selection.

### D10. File-size strategy

- Put the new invokables in a side bridge
  (`cxxqt_object/layer_target_bridge.rs`) like `layers_surface.rs`, so
  `cxxqt_object.rs` stays under its 1214-line allowlist ceiling. The existing
  `changed` signal carries target changes; no second signal is added.
- Split the delete/ungroup transitions into
  `impl_layers/target_transitions.rs` when `impl_layers.rs` would cross its
  1200-line cap.
- Put all new Qt cases in `tst_layer_target.cpp`. `tst_layers_panel.cpp` is at
  1399/1400; existing-case updates must stay net-zero or the file is split
  first.

### D11. Scope of the index invokables

The index-addressed invokables and the control-server JSON contract are
unchanged. Their bridge functions do get target transitions, so a control-server
layer operation that creates or deletes a layer leaves the target coherent, but
the request/response schema is untouched.

## Risks / Trade-offs

- **Rust `active_layer` readers and 33 C++ `active_layer_path()` readers**: each
  must be classified as edit target, anchor, or setter. Mitigation: mechanical
  migration, a review list, and unit tests for `single()` vs `anchor()`.
- **History target bookkeeping**: every `capture_live` call must pass the live
  target after the operation. Mitigation: `rg "capture_live\("` plus the history
  unit tests for undo, redo, jump, and named snapshots.
- **Refresh/filter feedback loop**: applying the target emits selection signals
  that would write it back. Mitigation: the apply guard on refresh, filter,
  isolation, and view changes; equality no-op; tests for filtered refresh and
  repeated refresh.
- **Panel tests using `selectPaths`**: it now also writes the target. Tests that
  used it as display-only setup behave as a user selection, which is the intent.
  Mitigation: run `tst_layers_panel`, `tst_merge_commands`,
  `tst_nested_layer_target`, `tst_last_layer`.
- **Ungroup remap**: surviving selected paths shift when a group is spliced.
  Mitigation: compute the mapping from the post-operation tree and test a mixed
  group + pixel selection.
- **Positional-path ceiling after undo**: a selection changed after a structural
  edit is sanitized, not remapped. Accepted and documented with a `ponytail:`
  comment; the upgrade is stable layer IDs.
- **No multi-layer Move**: the target exposes the set but the Move session still
  takes one layer. Accepted as a non-goal.
- **Group-only or empty documents**: the open fallback now selects the topmost
  top-level layer, which may be a group or adjustment; tools still refuse it.
  This is deliberate and tested.

## Migration Plan

1. Add `LayerTarget` and its unit tests.
2. Add the before/after target to history entries, including named snapshots.
3. Add the side-bridge invokables and switch initialization, including the
   open-state target before `history_for`.
4. Switch the panel to write/apply, guard filter/model/view changes, and remove
   the refresh capture/re-select and fallback.
5. Migrate selection-based panel commands and properties; set target transitions
   in every structural bridge command, including ungroup remapping.
6. Classify Rust and C++ readers; migrate frame menus; wire Move Auto-Select
   directly to the Rust target; delete the dead `layersPanel_` guards.
7. Add the Qt suite; run the full gates; archive the change.

Documents are not persisted with a target, so there is no data migration.
Opening a document initializes the target. Rollback is a revert of the commits;
no saved state changes.

## Open Questions

None that change the specs or the task breakdown.
