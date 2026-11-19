# Design

## Context

See `proposal.md` for motivation. The constraints that shape this design:

- The layer drag payload is newline-joined paths (`layers_panel_model.h:72-78`),
  but the panel's provider returns only `currentPath()` (`layers_panel.cpp:239-250`),
  so the router always sees one path.
- The source document is resolvable at drop time and keeps its own layer target
  (`selected_layer_paths`, `layer_selection_anchor`) after another tab becomes
  active.
- The destination projects a multi-path target into the panel on refresh
  (`layers_panel_target.cpp:185-216`).
- `Edit::None` suppresses history and presentation but does not restore the
  document (`edit.rs:238-242`), so a batch must never return it after mutating.
- The engine copies one node per call and checks color mode / bit depth before
  cloning or inserting (`transfer.rs:20-43`).
- Tree indices are bottom-first (`paths.rs:7`) while `flatten_rows` and the
  panel are topmost-first (`paths.rs:59-75`), and `insert_node` inserts a
  sibling at `anchor+1` (`create.rs:6-12, 287-317`) or as a group's last child.
  The placement rule below accounts for both.

## Goals / Non-Goals

**Goals:**
- A drop from a multi-layer selection copies every selected layer whose
  ancestor is not also selected in one "Duplicate Layer" state, preserving
  stacking order.
- Destination selection matches the copies, with the dragged layer's copy
  active.
- The single-layer drop keeps its current behavior and undo shape.

**Non-Goals:**
- Carrying the selection in the drag payload, multi-row reorder inside the
  panel, or multi-row strip-button actions.
- Converting copies into a different color mode or bit depth; mismatches are
  still refused.
- Changing Move-tool movement inside one document.

## Decisions

### Expand at the drop, not in the payload

The frame resolves the source view, and when the dragged layer belongs to that
view's multi-layer target it copies the whole target instead of the single
dragged path. A dragged path outside the target — a `/@sf/...` smart-filter
synthetic row — has no layer node (`parse_path` rejects it, `paths.rs:11-26`),
so `copy_path_to_document` resolves nothing and the drop copies nothing, as
today.

Alternative rejected: make the provider emit the selection. That payload feeds
the in-panel row drop, whose handler moves only `dragged.first()`
(`layers_panel_model.h:874-880`), so it would silently move one of N — the same
bug one file over — and it would also change strip-button delete/duplicate/group
into multi-row operations. Both are outside this change.

Alternative rejected: loop the existing single-copy bridge from the router. It
does not fix the reproduction because the payload has one path, and it records
one history state per layer.

### One batch bridge call, one edit state

`copy_layers_from_document(view, source, paths, dest_anchor, source_anchor)`
resolves the paths, drops duplicates, checks read-only and color mode / bit
depth, then enters one `edit` that loops `copy_path_to_document` inside the
destination borrow, sets the target, and records one `Duplicate Layer` state,
mirroring `duplicate_layers` (`impl_layers.rs:870-915`). `dest_anchor` is the
destination's current layer (the New Layer position); `source_anchor` is the
dragged layer. The bridge returns the final created paths in source order, so
the caller can verify and the target's active row can be the created path for
`source_anchor`, falling back to the topmost copy when that source was not
copied.

Alternative rejected: a C++ loop over the existing bridge. It produces N
history states and N panel refreshes, and any mid-loop refusal risks a partial
copy with no recorded state.

### One insertion point, order by anchor kind

Never chain an anchor onto a created copy: `insert_node` inserts above a sibling
anchor but *into* a group anchor (`create.rs:287-317`), so chaining reverses the
order and can nest a copy inside a copied group.

Every copy is inserted at the same destination insertion point (the New Layer
position of `dest_anchor`). With a sibling anchor the sources are processed in
forward panel order (topmost-first): each insert at `anchor+1` pushes the copy
block already placed at that point up, so the first source ends on top. With a
group anchor — and with an empty destination target, which appends at the top —
the sources are processed in reverse panel order, because each append lands
above the previous one.

### Current layer / target and created paths

The sources are sorted into the source document's top-to-bottom order
(`flatten_rows`), so the copies keep their stacking order regardless of how the
target was written. With a fixed insertion point, the i-th copy's final index is
`base + i` for an append and `base + (n - 1 - i)` above a sibling (later inserts
push the earlier block up, like `duplicate_paths`); the bridge computes those
indices directly rather than replaying each shift.

### Pre-validate, never lose a mutation

All source paths are resolved before the first insertion, and read-only,
color mode, and bit depth are checked before `edit` is entered. A path that does
not resolve is skipped. A refused drop therefore leaves both documents
unchanged without even settling a pending composite or cancelling a vector
place. If an insertion still fails after the first copy landed, the bridge
records the edit for the copies that landed rather than returning `Edit::None`,
because `None` would leave untracked layers. A non-resolving destination anchor
takes `insert_node`'s root-append fallback, and the bridge computes its index
for that case too.

### Skip descendants of selected ancestors

A selected group already carries its children. Paths with a selected ancestor
(prefix match on `/`) are dropped from the copy list so a child is copied once.
The app-side helper owns this rule and the bridge's contract says `paths` must
not contain a node together with its selected ancestor, because the engine's
`selected_paths(doc, paths, true)` (`paths.rs:146-172`) is `pub(super)` and is
not reachable from the app.

### Where the expansion lives

The expand/filter helper is a free function in `file_drop_router.cpp` (declared
in `file_drop_router.h`), because `frame.cpp` is 1180 lines against a 1200-line
code budget and is not allowlisted. `PicturaMainWindow::copyLayersFromDocument`
resolves the source view, calls the helper and then the bridge, and refreshes
the panel once.

### Selection and anchor

The destination target is set to every final created path. Its active row is the
created path mapped from `source_anchor` (the dragged layer); when that layer
was not copied — a descendant of a selected ancestor, or a synthetic row — the
topmost copy. `LayersPanel::refresh()` then paints the multi-selection.

### Move tool inherits the expansion

`tool_move.cpp` hands `layer_selection_anchor(*v)` to `makeLayerDragMime`
instead of `active_layer_path()`, which returns `layer_target.single()` and is
empty for a multi-layer selection (`impl_core.rs:771-777`); without the change a
multi-layer target never starts the drag. The frame's expansion then copies the
whole target, matching CS6's multi-layer move. Single-selection behavior is
unchanged.

## Risks / Trade-offs

- **A copied group and a separately selected descendant** → the prefix filter
  skips the descendant; if the panel later forbids that combination, the filter
  is harmless.
- **A source target path that no longer resolves** → skipped before mutating; if
  none resolve, the drop is ignored and the destination is unchanged.
- **A large selection makes one big history state** → matches the existing
  multi-layer `Duplicate Layer` convention; per-layer states would be worse for
  undo.
- **The bridge's signature changes from one path to a list** → internal seam;
  the app seam stays single-path and both bridge call sites are updated in the
  same change.
- **The Move-tool multi-selection path has no direct GUI test** → the panel drop
  is covered by the new test; the Move-tool gesture (a blocking QDrag from a
  canvas drag) is a manual check.

## Migration Plan

None. UI behavior only; revert the commit to roll back.
