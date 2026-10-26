# Proposal: Linked layers move together and show a link badge

## Why

Linking layers does not work end to end (#959). The engine already stores a
transient link set (`Layer > Link Layers` / `Unlink Layers`, `Select Linked
Layers`), and the bridge projects a `linked` flag per row, but:

- the row delegate never paints a link glyph, so a linked layer is
  indistinguishable from an unlinked one except for its underlined name;
- the Move tool moves only the single active layer: `commit_move` (and the
  arrow-key nudge) translate one path, so dragging a linked layer leaves the
  rest of its set behind;
- the action-strip Link button is hard-disabled (`layers_panel.cpp:285`) with a
  "not implemented yet" tooltip, so the CS6 way to link a selection is absent.

The CS6 contract is that linked layers "retain their relationship until you
unlink them" and move as one (`docs/05-layers/layer-management-ui.md`).

## What Changes

- The Layers-panel row delegate draws a `layers.link2` chain glyph in a row's
  right-edge badge run when the row is in a link set, reserving its advance in
  the same right-edge walk as the fx/lock/Blend If badges and the name rect.
- The Move tool translates every member of the active layer's link set by the
  same delta, in one undoable "Move Layer" state, for both the drag
  (`commit_move`) and the arrow-key nudge (`translate_layer`); the live drag
  preview shows the set moving together. A member that cannot be translated (a
  group, adjustment, position-locked, or unresolved path) is left in place.
- The action-strip Link button is enabled with a layer selection and toggles
  the selection's link state, showing the link glyph when the selection is not
  all linked and the unlink glyph when it is.

Link sets stay transient session state (they are not serialized and a
structural edit clears them); this change does not add persistence.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-management`: moving linked layers together.
- `ui/layers-panel`: the linked-row badge and the active Link strip button.

## Impact

- C++ panel: `panels/layers_panel_internal.h` (badge + layout),
  `panels/layers_panel.cpp` / `.h` (strip button, `syncStripButtons`),
  `panels/layers_panel_mask_menu.cpp`, `selftest.cpp` (strip expectations).
- Rust: `cxxqt_object/impl_transform/session.rs` and `mod.rs` (move group),
  `cxxqt_object/impl_transform/dispatch.rs` (nudge), `cxxqt_object.rs` /
  `impl_layers_select.rs` (link-state query), `cxxqt_object/state.rs`.
- Tests: new `tst_linked_layers`, Rust unit tests, updated self-test.
- Docs: `docs/05-layers/layer-management-ui.md`,
  `docs/02-ui-ux/panels/layers-panel.md`, `docs/dev/STATE.md`.
