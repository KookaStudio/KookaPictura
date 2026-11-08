# Design

## Context

See `proposal.md` for the motivation. The current state that shapes the
approach:

- The Rust catalogue (`pictura_script::command_catalogue`, 597 rows) already
  feeds the menu bar through `addDefaultCommands`. `entries.rs` is
  hand-maintained; its "generated from `command_tree.cpp`" header is stale and
  the generator is gone. `const_names.rs` carries the same stale "generated
  from" header; this change corrects both.
- 209 non-test `setHandler` sites across 25 files. 33 non-test
  `commandIdForPath` call sites remain; most translate catalogue-driven leaves
  (adjustment and filter families).
- `control_server.cpp` keeps a 40-id modal denylist. The request-scoped dialog
  guard from the archived `harden-control-server` change is already in place and
  `tst_control_modal_guard::everyCommandAnswers` already dispatches every
  command, so the hang is fixed; only the denylist cleanup remains.
- Panel menus carry their own tables: `LayersPanel`'s 50-row `RowSpec` with a
  26-value `Enable` enum and two action switches; `rowsForPanel` for 16 panels
  with six `performPanelMenuAction` implementations plus `runPanelMenuAction`'s
  dispatch chain; Properties footer buttons call the bridge directly.
- The registry's handlers already read the Rust layer target
  (`layerAnchor()`, `selectedLayerPaths()`), and `showContextMenu` already
  selects an unselected row. #829 (post-op target ownership) is open and is out
  of scope beyond the pre-op anchor.

## Goals / Non-Goals

**Goals:**

- One command namespace for the menu bar, panel row menus, panel flyouts,
  Properties footer, and the control socket: id, label, enabled/checked,
  shortcut, modal status.
- Delete the panel-local command tables and dispatch switches, not wrap them.
- Keep the Rust catalogue the single source; C++ may not hand-author commands
  or ids.
- Preserve behavior where the spec already fixes it, except the deliberate
  CS6/parity changes named in the spec deltas.

**Non-Goals:**

- Post-op layer-target ownership (#829): the row anchor is a pre-op user
  choice, not a post-op rule. The row's Move Up/Down keeps its current
  `move_layer_path` behavior until #829, because `arrange_layer_run` collapses
  the selection.
- Panel content lifecycle (#280) and main-window collaborator wiring (#1166).
- Making dialogs scriptable over the control socket.
- Migrating panel preferences (Add Mask by Default, Clip to Layer, Character/
  Paragraph checkboxes) into commands; they do not act on the document and stay
  panel-local controls.

## Decisions

### Frozen ids with one validated derivation

Handlers register against generated `command_ids::` constants. The static
leaves that still derive their id at runtime are frozen: Color Range, Refine
Edge, Warp Text, Convert To Paragraph/Point Text, Replace All Missing Fonts,
the 19 layer-style leaves, Apply Image, Image Size, Canvas Size, Print, Print
One Copy, Filter Gallery, Read Watermark, the Align/Distribute leaves,
Auto-Blend Layers, Lens Correction, the Type leaves, and the
`frame_document_menus.cpp` Image Size, Canvas Size, and Print call sites.
`commandIdForPath` survives only for Rust data-catalogue leaves (the adjustment
catalogue and filter descriptors). A derived id the catalogue does not contain
is rejected: the bridge returns an empty identifier and emits a warning naming
the path.

Alternative: make the registry register handlers by path and delete the free
function. Rejected: it moves the same lookup into the registry without removing
the path/id translation, and the issue's instruction is to register against
constants.

### Modal metadata is a Rust list, not a field on every row

The catalogue module exposes `modal_ids() -> &'static [&'static str]`, bridged
as `command_is_modal(id)` and cached in `CommandRegistry`. `dispatch_command`
refuses from it and the C++ denylist is deleted. The request-scoped guard stays
for dialogs reached by any other path.

Alternative: add `modal: bool` to `CommandSpec`. Rejected: `entries.rs` is
hand-maintained, so it churns 597 lines for a property only the control server
reads.

### Targeting is model (a): the right-click makes the row the anchor

`showContextMenu` and `showMaskThumbMenu` write the Rust target with the current
selection and the clicked path as the anchor before building the menu. Registry
handlers keep their `void()` shape and read the target. A layer-specific command
(mask, style, smart-object content) acts on the anchor; a selection-wide command
(delete, duplicate, merge, paste style, clear style) acts on the whole
selection.

Alternative: `dispatch(id, target)`. Rejected: it duplicates the Rust target,
forces every handler to accept a target, and contradicts the single-target
invariant. The anchor write is a pre-op choice and does not pre-empt #829.

### The row table stays as layout; enablement and labels leave it

`RowSpec` keeps the kind mask, block order, and separators, because the row
menu intentionally shows a different set from the menu bar. It loses the
`Enable` enum and the action switches. Each row resolves to a command id at
menu-open time:

- fixed rows map to one id;
- dynamic rows resolve by state (`clippingToggle` → Create/Release,
  `link` → Link/Unlink, `rasterizeLayer` → Layer/Fill Content/Smart Object);
- Add Layer Mask and Add Vector Mask become submenus of the existing commands.

Rows whose effect differs from the menu-bar command get a panel-only id rather
than a false 1:1 mapping: `newLayer`/`newGroup` create immediately (the menu
commands open dialogs), and `moveUp`/`moveDown` keep `move_layer_path`.

Actions are built from a detached action, not the menu-bar `QAction`:
`buildMenuBar` skips an empty path in the menu bar, so an empty-path command has
no menu item. `CommandRegistry::detachedAction(id, parent)` returns a fresh
`QAction` carrying the provider label, enabled and checked state, the checkable
flag, `data(id)`, and no shortcut (a reused menu action would draw the menu-bar
shortcut in the panel menu). Triggering it dispatches the id. Disabled stubs
with no command stay local rows with the `— not implemented yet` tooltip. A
panel menu is built once, so its `aboutToShow` calls
`CommandRegistry::applyState` on every action to re-read the label, enabled,
and checked state after a document or selection change.

### Panel-only commands use an empty menu path

A panel action that is not a menu-bar command is a catalogue row with
`path: &[]` and an id `panel.<panel>.<action>` (or a panel family prefix such
as `actions.`, `character.`, `paragraph.`). `buildMenuBar` does not put it in
the menu bar, but it attaches a hidden action to the window carrying the
command's shortcuts, so a binding assigned in the editor fires. The Keyboard
Shortcuts editor groups empty-path commands by their panel: `panel.<panel>` uses
`<panel>`, and the panel families use their first segment; the search index
already skips empty-path commands. Preferences stay local and are not commands.

### One label source

Dynamic labels move into registry label providers (Duplicate, Delete, Merge).
The menu bar, the row menu, and the panel flyout read the provider, so those
three surfaces show the same plural/singular label. `describe()` — and
therefore `list_commands` and the Keyboard Shortcuts editor — reports the
catalogue label. This deliberately changes the main-menu labels for a
multi-selection; the spec delta records it.

### Flyout rows become commands, layout stays

`rowsForPanel` keeps the label/separator/submenu layout, but an implemented row
carries a command id and is built from `detachedAction(id, parent)`. The six
`performPanelMenuAction` implementations and `runPanelMenuAction` are deleted
(a small local preference path stays for Add Mask by Default and the
Character/Paragraph preferences); the Adjustments panel's catalogue-driven rows
map to the existing `Layer > New Adjustment Layer > <leaf>` ids and contribute
no panel-only commands. The Properties footer buttons dispatch
`panel.properties.reset`, `panel.properties.toggleVisibility`, and
`panel.properties.deleteLayer`; Clip maps to Create/Release Clipping Mask by
`row.clipping`. The Delete command is path-scoped: it deletes `{path_}` with
`layers_can_delete({path_})` and `clearSolo`, not `LayerDeleteLayer` (which
deletes the selection).

## Risks / Trade-offs

- **Qt's right-press current-index behavior is style-dependent** → write the
  anchor explicitly in `showContextMenu`/`showMaskThumbMenu` instead of relying
  on it, and cover it with a multi-selection test.
- **Registry dispatch can change a row's effect** where the row used a
  path-specific bridge (New Layer immediate vs dialog; Move Up vs Arrange) →
  classify every row against the command's effect and add a panel-only id where
  they differ; the parity suite names the representative set.
- **Main-menu labels change** (Duplicate/Delete become plural for a
  multi-selection) → accepted for cross-surface parity; recorded in the spec
  delta.
- **`LayerArrange*` collapses the selection** → the row's Move Up/Down stays on
  its current command until #829; do not map it to Arrange in this change.
- **Panel-only commands enlarge `list_commands` and `describe()`** → empty-path
  commands are skipped by the menu bar and the search index; the shortcut editor
  groups them by panel.
- **The modal audit can miss a dialog** → keep the request-scoped guard as the
  second line and keep `tst_control_modal_guard::everyCommandAnswers`; add the
  known missing ids (Image Size, Canvas Size, Fill, Stroke, Adjustments, and the
  other `runDialog` users) to the list.
- **File size** → `layers_panel_menu.cpp` and `panel_group_menu.cpp` shrink; no
  allowlist change is expected.

## Migration Plan

1. Freeze the static ids, regenerate `command_ids.h`, and add the catalogue
   validation.
2. Add `modal_ids()` and switch `dispatch_command`; delete the denylist.
3. Add the anchor write and its test.
4. Convert the Layers row/mask/vector menus; delete the `Enable` enum and the
   switches.
5. Convert the flyout, History, Actions, Character, Paragraph, and Properties
   footer (keeping the local preference paths); delete `runPanelMenuAction` and
   the six `performPanelMenuAction` implementations.
6. Add the parity suite and update the shortcuts editor.
7. Validate with `openspec validate --all --strict`, `cargo nextest`,
   `ctest -R '^tst_'`, and the guard scripts.

Rollback is a revert of the single PR; no on-disk format changes. The shortcut
store ignores unknown ids, so a revert leaves no harmful override.
