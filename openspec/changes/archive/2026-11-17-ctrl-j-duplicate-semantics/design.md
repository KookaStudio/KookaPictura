# Design

## Context

- The command registry owns one `QAction` per catalogue id; that same action
  carries the menu label and the keyboard accelerator, so an action disabled by
  its enabled-provider cannot fire its shortcut.
- `Ctrl+J` is catalogued on `layer.new.layerViaCopy`
  (`crates/pictura-script/src/command_catalogue/entries.rs:177`), and the engine
  path refuses a group target (`layer_ops/via.rs:37-39`).
- The Layers-panel duplicate path already duplicates groups, descendants, and
  multi-selections, and records one undo state
  (`panels/layers_panel_actions.cpp:137`, `document_ops/layer_ops/properties.rs:280`).
- The registry already supports a contextual command: `layer.merge.layers`
  (`Ctrl+E`) changes its label, handler, and enablement with the selection, and
  the command-registry spec requires shortcuts to follow the current selection.
- `Layer via Cut` shares the via-copy bridge but must keep its selection
  requirement.

## Goals / Non-Goals

**Goals:**

- One `Ctrl+J` command whose behavior follows the active target and selection.
- Reuse the existing panel duplicate path so the engine and codec stay untouched.
- Keep the `Layer via Copy` label and its `Ctrl+J` menu display.

**Non-Goals:**

- Changing `Layer via Cut`'s selection-copy or clear behavior; only its
  group-target enablement is tightened so it is disabled instead of an enabled
  no-op.
- Routing non-raster, non-group kinds (adjustment, fill, type, smart object)
  with a selection; their channel-less copy is a pre-existing engine behavior
  this change leaves untouched.
- Moving the shortcut to `layer.duplicate.layer` or splitting the menu item from
  the accelerator with a second action or key router.
- Any Rust, catalogue, CMake, or data change.

## Decisions

### D1: Contextual single command, not a second command or key router

`Ctrl+J` stays on `layer.new.layerViaCopy` and its handler branches. A second
command sharing the key makes Qt report an ambiguous shortcut and neither fires;
moving the key to `layer.duplicate.layer` breaks selection copy and drops the
menu display that `docs/02-ui-ux/menus.md` records. This mirrors the Ctrl+E
merge command.

### D2: Group and empty-selection targets delegate to the panel duplicate path

The handler keys on the active row (the anchor). A group anchor, or a selection
that covers no pixels, takes the duplicate path and calls the existing
`duplicateSelection()`, which duplicates every selected path (each selected
group whole with every descendant, alongside the other selected layers); without
a Layers panel it does nothing. This inherits group/descendant copying,
multi-selection handling, solo clearing, copy selection, and the single undo
state. A non-group anchor with a non-empty selection keeps the existing
anchor-only `layer_via_copy(anchor)` call.

### D3: Enablement splits; the shortcut must be live

`Layer via Copy` becomes enabled whenever a document and a layer target exist;
`Layer via Cut` keeps `document && a selection covering at least one pixel &&
target` and additionally requires the target row kind to be non-group. The
group test refreshes the registry explicitly and asserts the action is enabled
with no selection, so a stale QAction cannot silently swallow `Ctrl+J`.

### D4: The label stays `Layer via Copy`

Photoshop keeps that menu label while the accelerator is contextual, so no label
provider is added. A menu click therefore follows the same contextual behavior as
the shortcut.

## Risks / Trade-offs

- **Menu item enabled for a group with no selection** → Accepted: the single
  action cannot be disabled for the menu and enabled for the accelerator. The
  spec now defines `Layer via Copy` as contextual, so menu and shortcut agree.
- **Enablement depends on live registry refresh** → The command-registry spec
  already requires selection-driven refresh; the Qt test asserts the transition.
- **Adjustment/fill/type/smart-object layers with a selection** → Their
  channel-less copy produces a selection-shaped pixel layer today. This change
  does not alter that pre-existing behavior; a separate parity item covers
  non-raster kinds.
- **Delegate path needs the Layers panel** → The handler delegates only when the
  panel exists and otherwise leaves the document unchanged; the app always
  builds the panel and the test exercises that route.
- **Layer via Cut on an alpha-less or locked source** → The engine keeps its
  pre-existing clear behavior; this change only corrects the group enablement.

## Migration Plan

None: no persisted state, serialization, or user preference changes.
