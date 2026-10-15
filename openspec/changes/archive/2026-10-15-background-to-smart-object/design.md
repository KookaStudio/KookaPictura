# Design

## Context

See `proposal.md — Why`. The gate is a single engine predicate
`pictura-render::can_convert_to_smart_object`; the bridge predicate
`PictureView::layer_can_convert_to_smart_object`, the `Layer ▸ Smart Objects ▸
Convert to Smart Object` command, `Filter ▸ Convert for Smart Filters`, and the
`Filter ▸ Pictura Raw` enablement all delegate to it. `convert_for_smart_filters`
is an alias of `convert_to_smart_object`.

The Background→ordinary-layer step already exists as
`layer_ops::create::layer_from_background` (clear flag, release locks, append an
opaque alpha channel when absent, rename to the next free `Layer N`) and as the
`PictureView::convert_background` / `layer_from_background` bridge commands. The
bridge records history once per command, after the engine op returns, so a silent
engine unlock is naturally part of the same history state.

Two facts shape the spec deltas: the OpenSpec validator (1.13.2) refuses a
`MODIFIED` requirement that renames or drops a scenario the main spec still has,
and a completed-but-unarchived change (`smart-object-row-menu`) already
`MODIFIED`s the `ui/layers-panel` `Panel and row menus` requirement.

## Goals / Non-Goals

**Goals:**

- `Convert to Smart Object` succeeds on a Background as one history state, leaving
  an ordinary (non-Background) smart-object layer with an unchanged composite.
- Keep the predicate single-sourced so every menu surface agrees.
- Stay inside the self-test file-size/rule-11 constraints and a clean archive.

**Non-Goals:**

- Changing `Filter ▸ Pictura Raw` behavior (separate change, #478).
- Wiring the still-disabled `Convert to Smart Object` entry in the Layers-panel
  *flyout* menu (`panel_group_menu.cpp`); it is disabled for every kind today and
  is pre-existing scaffolding, not part of this issue.
- Handling 16/32-bit documents (`write_psd` remains 8-bit only).
- Changing the Background→layer operation itself or its bridge history label.

## Decisions

### D1 — Relax the shared predicate, don't split it

Drop `&& !layer.background` from `can_convert_to_smart_object` and keep every
other guard. Alternative (rejected): keep the engine predicate strict and do the
unlock in the app command only, computing a separate enablement. That would
duplicate the eligibility logic across the command, the row menu, Convert for
Smart Filters, and Pictura Raw, and would make the engine op and its predicate
disagree. The unavoidable consequence is that `Convert for Smart Filters` also
works on a Background (CS6-coherent), captured in `imaging/filter-app-ui`.

### D2 — Unlock inside the engine operation

In `convert_to_smart_object`, when the resolved layer is a Background, unlock it
(the existing `release_background` + `next_layer_name` from `create`, which is
what `layer_from_background` does) before cloning it into the embedded source, so
the source's layer is a normal layer and `filename` is `<next free Layer N>.psd`.
Doing it in the engine (not the bridge) means all callers — the command, the
alias, `apply_pictura_raw_filter` — share it, and the bridge still records exactly
one state because the engine step is silent.

`next_layer_name` yields the document's highest `Layer N` + 1 (`Layer 1` when none
exists — see `layer_ops/tests.rs`), so the design does not promise the literal
name `Layer 0`; it promises the next free `Layer N`, which is what CS6 does.

**Atomicity:** the unlock must not violate the operation's "returns `false`
without mutating the document" contract if `write_psd` fails. So author the
payload from a *copy* of the layer with the unlock applied, call `write_psd`, and
only after it succeeds apply the unlock and the smart object to the real layer.
The document is mutated only on success.

### D3 — Row menu via the kind mask

`kRowSpecs` already has `convertToSmartObject` wired to the convert bridge and the
`ConvertToSmartCan` predicate. Only its kind mask changes from `Pixel` to
`Pixel | Background`, so a Background row shows the same row, enabled by the
relaxed predicate, and reselects after conversion. Alternative (rejected): a
second row id for backgrounds — pointless duplication of the same handler.

### D4 — Shrink the self-test, grow the Qt test

`lpr_smart_object_convert` (277) currently asserts the Background refuses. Under
the new contract that assertion is invalid; per AGENTS.md rule 11 the self-test
only shrinks, so remove the background assertion and the `background_from_layer`
call that set it up, plus its slot in the `ST_PASS` format/args and the stale
comment. Cover the Background→smart-object case in `tst_smart_object_actions.cpp`
instead. The Qt test creates the Background explicitly (the bridge `newDocument`
makes a plain `Layer 0`, not a Background) via `background_from_layer`, then
asserts: row and command enabled, one `Convert to Smart Object` state, the row
kind is no longer `background`, a smart object is present, and a second dispatch
is disabled.

### D5 — Spec structure

- `compositing/smart-object-layer-actions`: the eligibility requirement is
  `REMOVED` + `ADDED` (renamed to `Convert to Smart Object eligibility`) because
  the validator forbids renaming the `The Background is refused` scenario under
  `MODIFIED`. `The raster proxy and rendering are unchanged` is `MODIFIED` to
  carve out the Background unlock (the base text forbids renaming and adding a
  channel). The command requirement is `MODIFIED`.
- `ui/layers-panel`: a standalone `ADDED` requirement rather than `MODIFIED`
  `Panel and row menus`, so it cannot collide with the unarchived
  `smart-object-row-menu` change. Once that change archives, fold this row into
  `Panel and row menus` and delete the standalone requirement.
- `imaging/filter-app-ui`: `Convert for Smart Filters command` is `MODIFIED` to
  add Background eligibility.

## Risks / Trade-offs

- [Interim: `Filter ▸ Pictura Raw` on a Background now converts it to a smart
  object] → Accepted; the destructive-on-normal-layer fix is #478. Recorded in the
  proposal so it is not mistaken for intended final behavior.
- [`tst_command_tree.cpp` asserts the exact Background row list and will fail] →
  Add `Convert to Smart Object` to `expectedBackground` (task 3.4).
- [The Qt test's first instinct is a fresh document, which is not a Background] →
  Create it with `background_from_layer` and take the history baseline after it.
- [Spec drift: the `ui/layers-panel` ADDED requirement duplicates the per-kind
  row contract] → Fold it into `Panel and row menus` once `smart-object-row-menu`
  archives.
- [The self-test file is ~1148/1200 lines] → Only remove lines; do not add.

## Migration Plan

No data or schema migration. Rollback is reverting the change; documents that were
converted while the change was live stay valid (an ordinary smart-object layer).

## Open Questions

None blocking.
