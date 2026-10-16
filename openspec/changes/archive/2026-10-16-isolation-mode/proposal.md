# Proposal: isolation-mode

## Why

Issue #112 asks for Photoshop's isolation mode: focus editing on one set of
layers, with a way to release a layer and to exit. Photoshop's actual feature
(introduced in Photoshop CC, not CS6) is the Layers panel's **Selected** filter
type — choosing it shows only the selected layers, the Move tool's Auto-Select
is scoped to them, and a layer can be removed with **Release from Isolation**.
The repo already has the Layers panel filter (`compositing/layers-filtering-search`)
but deliberately excluded the `Selected` type as CC-only, and the Move
Auto-Select (`impl_layers/auto_select.rs`, #384) is unscoped. This change adds
the Selected dimension and the isolation-mode behavior around it, and amends the
contract to bring the CC feature into scope.

## What Changes

- The Layers panel filter row SHALL offer a **Selected** dimension (no criteria
  control — the criterion is the current selection). With it chosen and the
  toggle on, only the selected layers and their group ancestors are shown.
- Isolation mode SHALL be that state: the **Move** tool's Auto-Select SHALL pick
  only a layer in the isolated set (or a descendant of one).
- The Layers panel row context menu SHALL offer **Release from Isolation** for an
  isolated row; it SHALL remove that row from the set (deselect it), and
  releasing the last one SHALL end isolation.
- With the Move tool active, the canvas right-click menu SHALL offer **Isolation
  Mode**, which SHALL isolate the current selection. Turning the filter off,
  choosing another dimension, pressing **Escape**, or switching documents SHALL
  end isolation.
- Isolation SHALL be view-only: no document mutation, no history state, no
  serialization.

## Capabilities

### New Capabilities

- `ui/isolation-mode`: the Selected filter as isolation mode, Move Auto-Select
  scoping, Release from Isolation, and entering/exiting the mode.

### Modified Capabilities

- `compositing/layers-filtering-search`: the filter row gains the `Selected`
  dimension and the predicate gains its clause.

## Impact

- `crates/pictura-app/cpp/panels/` — `layers_filter_bar.{h,cpp}` (the dimension),
  `layers_filter_proxy.{h,cpp}` (the predicate), `layers_panel.{h,cpp}` +
  `layers_panel_menu.cpp` (selection wiring, Release row, enter/release API).
- `crates/pictura-app/src/cxxqt_object/` — `state.rs` (the isolated set),
  `impl_layers/auto_select.rs` + `helpers_composite.rs` (scoped hit test),
  `impl_isolation.rs` (the bridge), `cxxqt_object.rs`, `impl_core.rs` (reset).
- `crates/pictura-app/cpp/` — `frame.cpp` (Escape + the Move canvas menu),
  `frame.h`.
- `docs/` — `05-layers/layer-filtering-and-search.md`, `02-ui-ux/panels/layers-panel.md`,
  `GLOSSARY.md` (the contract now includes the Selected type; `TASK-ALLOWS-DOCS`).
- Tests: a Rust `#[test]` for the scoped Auto-Select, plus Qt Test
  (`tst_layers_panel`, `tst_command_tree`). No new dependency.

## Provenance

Photoshop's isolation mode / Selected filter is documented in CC-era sources
(Adobe Help "Filter layers"; the feature is absent from the CS6 Help, which is
why the current spec excluded it). Behavioral parity only — no pixel oracle.
