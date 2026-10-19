# Design

## Context

See `proposal.md` for motivation. The relevant engine surface is small and
already centralized in `crates/pictura-render/src/document_ops/layer_ops/`:

- `properties.rs` owns within-container moves (`move_path`,
  `move_path_to_dest`/`can_move_path_to`/`move_path_to`), Arrange
  (`arrange_target`/`arrange_path`/`can_arrange_path`), grouping
  (`group_paths`), and rename (`rename_path`).
- `clipping.rs` owns clipping creation (`can_create_clipping_mask`), base
  resolution (`base_below`), and post-edit reconciliation
  (`normalize_clipping`).
- `create.rs` owns tree-aware insertion (`insert_node`), `group_layer`, and
  `background_from_layer`.
- The app bridge and the Layers panel call the engine predicates directly:
  `can_move_layer_to` drives the drop indicator, `clipping_can_create` drives
  menu enablement, and `clipping_toggle` drives Alt-click and the Properties
  panel. Move Layer Up/Down reaches `move_path` through
  `LayersPanel::moveCurrent`.

Containers are bottom-first `Vec<Layer>`; "above" is a higher index. The
in-session Background flag is authoritative for the guards; the codec and the
rename path set it per the First-class Background requirement, and no guard
re-derives it from a name or index.

## Goals / Non-Goals

**Goals:**

- Enforce one Background floor shared by delta moves, drop targets, Arrange, and
  Move Layer Up/Down.
- Allow at most one Background per document; `Background From Layer` refuses a
  second one, and rename no longer creates one.
- Refuse clipping creation whose resolved base is a Background and prevent
  stack edits (move, grouping, ungrouping, Reverse, Background conversion) from
  leaving a clip on a Background.
- Keep dry runs (`can_*`) and mutations in lockstep.
- Leave PSD load/save round-trip semantics unchanged.

**Non-Goals:**

- Cleaning an invalid clip at PSD load time.
- Disabling the New Layer dialog's clipping checkbox in the UI.
- Reworking clipping membership or compositing rules beyond the Background base.
  Merge paths keep their documented behavior: only `Merge Clipping Mask` folds
  clipping coverage; Merge Down / Merge Layers do not re-base runs.
- Preventing paste/duplicate from landing below a Background; `insert_node`
  always inserts above the selection, so those paths already preserve the
  invariant in a valid document.

## Decisions

### One `background_floor` helper in `properties.rs`

`background_floor(container) = highest index of a flagged Background + 1`, or
`0` with none (use `rposition`). It is private to `properties.rs` because only
the move/arrange functions need it.

- `move_path`: refuse when `index < floor` (a malformed source already below the
  Background) or `floor > last`; otherwise clamp the delta target to
  `[floor, last]`. The `index < floor` guard is required because the target is
  computed pre-removal and inserted after removal, so the pre-removal floor
  would otherwise jump a sibling.
- `move_path_to_dest`: refuse when a concrete `dest_index` is below the floor.
  An append (`dest_index == None`) is always above the floor.
- `arrange_target`: refuse when `index < floor` or `floor > front`; otherwise
  clamp `Back` to `floor` and `Backward` to `max(index - 1, floor)`.

Alternative considered: guard each command separately. Rejected because it
duplicates the index math and lets `can_*` and the mutation drift.

### Refuse drops, clamp relative commands

A drop below the Background has no unambiguous legal landing position, so
`move_path_to_dest` returns `None`; the panel then shows no indicator and
ignores the drop. Delta moves, Arrange, and Move Layer Up/Down are relative, so
they stop at the floor instead of failing outright. This matches the issue's
expected drag behavior and keeps Send to Back useful.

Same-container pre-removal coordinates: comparing the pre-removal destination
index to the pre-removal floor is correct while the source is above the
Background (the valid state). A source already below the flagged Background is
outside the valid state and is refused before any mutation.

### Clipping base check uses `base_below`

`can_create_clipping_mask` keeps every existing guard and adds the base check.
The full predicate is: the path parses and resolves; the selected layer is not
the Background; the selected layer is not already clipped; and
`base_below(container, index)` is `Some(base)` with `container[base]` not the
Background. Using `base_below` (not the immediate sibling) refuses a layer above
an existing run whose base is a Background.

`create_clipping_mask` keeps routing through this predicate, which covers the
menu, Alt-click, and the Properties toggle.

For insertion, `clipping_base_allowed(container, index)` returns true only when
a non-Background base exists below the insertion point. `insert_node` clears a
requested `clipping` flag when it is false, replacing the three `index == 0`
checks. The dialog stays unchanged; the engine is the authority.

### Release Background-based runs in `normalize_clipping`

After the existing moved-flag reconciliation, find the highest flagged
Background (`rposition`) and release the contiguous clipped run directly above
it. Running this last prevents the moved-flag logic (including
`same_base_below`) from re-clipping a node directly above the Background. This
covers Reverse, ungroup, grouping, Background conversion, and future edit paths
that call the normalizer.

Alternative considered: clear only a single clipped child. Rejected because a
run above the Background would re-base to the newly released child instead of
being released as a group.

### Grouping normalizes both containers

`group_paths` and `group_layer` call `normalize_clipping` on the new group's
children before insertion and on the parent container after insertion. Without
this, grouping a clipped layer can leave it clipped at the new group's bottom,
which the modified clipping requirement forbids. Grouping a base keeps its run
attached to the group (the group becomes the new base), which is the documented
clipping behavior; only a clipped bottom is cleared.

### One Background and no rename-created Background

`background_from_layer` refuses when any node in the document already carries
the Background flag (walk the tree; the selected layer's own flag is already
checked). `rename_path` no longer sets or clears the Background flag; it only
changes the name. The codec still derives the flag on read from the bottom
top-level layer named `Background`, and `Layer from Background` /
`Background From Layer` remain the only flag transitions. This matches
`docs/05-layers/layers-overview.md:72`.

### PSD codec untouched

The codec keeps reading and writing the clipping byte and the Background name
verbatim. No load-time normalization is added: a PSD created elsewhere with an
invalid clip loads as authored and is normalized by the first stack or clipping
edit. Sanitizing on load would silently rewrite vendor files and is outside the
bug's scope; the runtime invariant is covered by the normalizer tests.

## Risks / Trade-offs

- **Pre-existing invalid clips disappear after an edit** → This is the intended
  contract; the row simply loses the clip indicator on the next normalization.
- **Multiple or misplaced Background flags** → New documents cannot gain a
  second flag; a pre-existing malformed document still uses `rposition` for the
  floor and `normalize_clipping` releases above the highest flag. `floor > last`
  refuses instead of panicking on `clamp`.
- **Test file budgets** → `tests.rs` is 1323/1400 lines; clipping tests go in
  `clipping.rs` (422 lines) and the #717 tests must stay compact. If the four
  floor tests would exceed the remaining 77 lines, put them in a new
  `tests_background.rs` module (declared from `layer_ops/mod.rs`) instead of
  growing `tests.rs` past the cap. Qt coverage goes in
  `tst_layers_panel_fixes.cpp` (747 lines) and `tst_command_tree.cpp`, not the
  1376-line `tst_layers_panel.cpp`, to avoid an allowlist entry.
- **Existing assertion encodes the old Arrange behavior** → The test at
  `tests.rs:1219` expects `normal` at index 0 after Send to Back; it must expect
  `[Background, normal, locked]` instead.
- **No UI hint that the dialog clip was ignored** → Acceptable for now; a
  disabled checkbox is a separate UX change with no behavior impact.
- **Rename behavior changes** → Renaming a normal layer to `Background` no
  longer makes it the Background. This matches the docs; the dedicated
  `Background From Layer` command remains the only conversion path.
