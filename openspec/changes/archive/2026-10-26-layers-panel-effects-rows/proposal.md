# Proposal: Layers panel layer-effects rows

## Why

A layer that carries a layer style shows only the `fx` badge on its row; the
panel never lists the effects themselves. CS6 nests a layer's effects under its
row — an **Effects** group and one row per effect, each with its own eye — and
lets the user reach the Layer Style dialog and the layer-style commands from
those rows. Without the nested rows the badge is a dead end: there is no way to
see which effects a layer carries, to switch one effect off, or to open the
style dialog from the panel (#960).

## What Changes

- Project each styled layer's effects as synthetic child rows: an **Effects**
  group row (disclosure + eye for the master `fx.visible` switch) and one row
  per present effect (`<effect>.on` eye), in the layer-style list order.
- Make the layer row's `fx` badge the disclosure that expands/collapses the
  group; a newly styled layer starts expanded when the panel option
  **Expand New Effects** is on.
- Double-clicking the `fx` badge or the **Effects** row opens the Layer Style
  dialog on Blending Options; double-clicking an effect row opens it on that
  effect's page.
- Right-clicking the `fx` badge, the **Effects** row, or an effect row opens
  the CS6 layer-style menu (Disable/Enable Layer Effects, Blending Options…,
  the ten effect entries, Copy/Paste/Clear Layer Style, and the disabled
  Global Light… and Create Layer, Hide/Show All Effects, Scale Effects…).
- A synthetic effect row never becomes the document's layer target.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the nested layer-effects rows and their interactions.

## Impact

- C++ panel: `layers_panel.cpp`, `layers_panel_model.h`,
  `layers_panel_internal.h`, `layers_panel_events.cpp`, `layers_panel_menu.cpp`,
  `layers_panel_actions.cpp`, `layers_panel_target.cpp`.
- App bridge: the layers snapshot gains the effects' enabled state and the
  master switch (`impl_layers.rs`, `cxxqt_object.rs`, `api/layers_api.*`).
- Dialog: `layer_style_dialog.{h,cpp}` gains the shared effect title/keys table.
- Tests: `tst_layers_effects` (new suite) and `tst_layers_snapshot`.
- Docs: `ROADMAP.md`, `docs/dev/STATE.md`.
