# Proposal

## Why

`Ctrl+J` is bound to `layer.new.layerViaCopy`, which requires an active
selection and whose engine path refuses a group target. Photoshop CS6's
`Ctrl+J` is contextual: it duplicates a selected group, duplicates the layer
when there is no selection, and copies the selection pixels otherwise. Ctrl+J is
the only keyboard route: the Layer menu's Duplicate Group… and the panel menu
already duplicate groups, while Ctrl+J is a silent no-op on a group (or disabled
with no selection).
Source: parity register `LAY-12` (#590).

## What Changes

- `Ctrl+J` (`Layer via Copy`) becomes a contextual command:
  - a group target duplicates the whole selection: each selected group whole,
    including every descendant, plus any other selected layer;
  - no active selection (or a selection covering no pixels) duplicates the
    selected layer(s);
  - an active selection on a non-group target copies only the selected pixels,
    as today.
- `Shift+Ctrl+J` (`Layer via Cut`) keeps its active-selection requirement and is
  disabled for a group target, where it is currently an enabled no-op.
- The Layer menu keeps the `Layer via Copy` label and its `Ctrl+J` display; the
  entry is enabled whenever a layer target exists.
- Group duplication reuses the existing Layers-panel duplicate path
  (`duplicate_layers`), so no engine or codec behavior changes.
- The parity register row `LAY-12` flips to `SHIPPED` and the ROADMAP item is
  ticked.

## Capabilities

### New Capabilities
<!-- None: the changed behavior belongs to an existing capability. -->

### Modified Capabilities

- `compositing/layer-management`: the "Layer via Copy and Layer via Cut"
  requirement changes. `Layer via Copy` gains group duplication and
  no-selection layer duplication while keeping selection-pixel copy;
  `Layer via Cut` keeps its selection requirement and is disabled for a group
  target.

## Impact

- `crates/pictura-app/cpp/menu_registrar.cpp` — the `Layer via Copy` handler and
  the two commands' enablement providers.
- `crates/pictura-app/cpp/tests/tst_layer_commands.cpp` — Qt Test coverage for
  the three dispatch outcomes and the cut outcomes (selection, no selection,
  group target).
- `docs/dev/doc-code-parity-gaps.md`, `ROADMAP.md` — tracking rows
  (`TASK-ALLOWS-DOCS`).
- No Rust, command-catalogue, CMake, codec, or data changes.
