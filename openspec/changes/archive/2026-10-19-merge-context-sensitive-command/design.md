# Design

## Context

See `proposal.md` for motivation. The engine already implements both merge
scopes: `PictureView::merge_layers(paths)` chooses `MergeScope::Down` for one path
and `MergeScope::Selected` for two or more, and `can_merge_layers` mirrors that
choice. The defect is in the command table and UI wiring:

- `command_tree.cpp` declares both `layer.merge.down` and `layer.merge.layers`.
- `frame_menus_layer_ops.cpp` wires the former to `currentPath()`, so a
  multi-selection still leaves an enabled `Merge Down` that ignores the rest of
  the selection.
- `frame_menus.cpp` already wires `layer.merge.layers` to `selectedPaths()`.
- The Layers panel flyout dispatches `merge_layers(selectedPaths())` correctly
  but always labels the row `Merge Down` and never disables it.

The main `layer-management` spec has two conflicting requirements: the
selection-dependent command and `Merge Down as a distinct command`. The original
design (`openspec/changes/archive/2026-09-18-layers-panel-management/design.md`,
D4) and the command-registry spec describe one command with one id.

## Goals / Non-Goals

**Goals:**

- One Layer-menu merge item whose label, scope, and enablement follow the
  selection.
- `Ctrl+E` stays on that item and routes to Down for one layer, Selected for
  two or more.
- The Layers panel flyout label and enabled state follow the same rule.
- Retire both stale `layer-management` requirements without changing engine
  behavior.

**Non-Goals:**

- Changing merge semantics, history labels, or the engine API.
- Making non-menu command listings selection-aware.
- Migrating saved keyboard overrides for the retired id.

## Decisions

### D1 — Delete `layer.merge.down` instead of hiding it

The stable id for the single command is `layer.merge.layers`, as declared in
`openspec/specs/document/command-registry/spec.md`. Keeping a second id hidden or
guarded would preserve an internal distinction CS6 does not have and would leave
two handlers to keep in sync. Deleting the id, its handler, its enable provider,
and `merge_down_run` restores D4. Stale shortcut overrides for the removed id are
ignored by `CommandRegistry::setShortcuts`, which silently skips unknown ids.

Alternatives rejected: a hidden alias (extra code for no user-visible benefit)
and two visible items with selection-dependent enablement (still shows two merge
entries, contrary to CS6 and the issue acceptance).

### D2 — Dynamic label through the existing label-provider hook

`CommandRegistry::setLabelProvider` already drives dynamic action text, and
`buildMenuBar` connects each menu's `aboutToShow` to `refresh()`. Add a provider
for `layer.merge.layers` that returns `Merge Down` only when
`layersPanel_ && layersPanel_->selectedPaths().size() == 1`, and `Merge Layers`
otherwise. The null guard is required: `frame.cpp` calls `buildMenus()` before
`buildPanels()`, so `layersPanel_` is null while `buildMenuBar()` runs its first
`refresh()`. The zero-selection case therefore deliberately reads `Merge Layers`
and the existing enable provider leaves it disabled.

Alternative rejected: rebuilding the Layer menu on selection changes, which
duplicates the existing refresh path.

### D3 — Flyout label and enablement update on `aboutToShow`

`PanelGroup::updateHeaderMenu` rebuilds the panel menu on tab changes, not on
layer-selection changes. Give wired panel-menu actions their action id via
`QAction::setData`, then find `mergeDown` and update it from the Layers panel:

- text: `Merge Down` for one selected path, `Merge Layers` otherwise;
- enabled: `LayersPanel::canMergeSelection()`.

Add `LayersPanel::canMergeSelection()` as a thin read-only wrapper around
`view_ && view_->can_merge_layers(selectedPaths())`, matching the Layer-menu
enable provider. Set both values once at build time and reconnect on
`headerMenu_->aboutToShow`; the menu is recreated by `updateHeaderMenu`, so the
connection lifetime follows the menu. The dispatch path stays unchanged because
it already uses `selectedPaths()`.

Tests must fetch the live menu through `headerMenuButtonForTest()->menu()` and
emit `aboutToShow` before reading; `panelMenuTextsForTest()` alone does not fire
the hook and can read a stale label.

Alternatives rejected: connecting `LayersPanel::selectionChanged` to a full menu
rebuild (selection changes are frequent and the menu only needs the state when
it opens) and leaving the flyout enabled (it would keep violating
`openspec/specs/ui/layers-panel/spec.md`'s disable-when-inapplicable rule).

### D4 — New Qt test file plus one engine test

`tst_command_tree.cpp` is close to the 1400-line test cap. Put the new
selection-state and flyout cases in `tst_merge_commands.cpp` and register it in
`crates/pictura-app/cpp/tests/CMakeLists.txt`. Remove the retired
`LayerMergeDown` coverage from `tst_command_tree.cpp`; that suite keeps covering
arrange, stamp, delete, and lock.

The existing Merge Down engine test uses fully opaque layers and does not cover
masks, so add one small `merge_tests.rs` case that merges a masked layer and
checks the resulting alpha against the compositor oracle. This is test-only; it
does not change engine behavior.

### D5 — Spec and docs

Replace both conflicting `compositing/layer-management` requirements with one
context-sensitive requirement via `REMOVED` + `ADDED`. The command-registry and
Layers-panel specs already describe the single command and need no delta. Correct
the divergent docs in a separate docs commit with `TASK-ALLOWS-DOCS`:

- `docs/05-layers/merge-and-flatten.md`: command table, adjacent-merge wording,
  UI surface, `LayerMergeActions`, and the one-layer edge case.
- `docs/02-ui-ux/panels/layers-panel.md`: flyout label and disable behavior.
- `docs/02-ui-ux/menus.md`: one `Merge Down / Merge Layers` item on `Ctrl+E`.

`docs/02-ui-ux/keyboard-shortcuts.md`'s `Alt + Merge Down` panel-menu line and
`docs/dev/m42-panel-menus.md`'s historical status table stay out of scope: the
former still describes the one-layer label, and the latter is a milestone note.

## Risks / Trade-offs

- **External dispatch of `layer.merge.down`** → the id was an implementation
  detail added by the later arrange/stamp change; dispatch now returns false.
  No documented contract depends on it.
- **Static label in non-menu listings** → `CommandRegistry::describe()` uses the
  declarative label, so the Keyboard Shortcuts dialog keeps `Merge Layers`. The
  Layer menu and panel flyout, the surfaces in this issue, use the dynamic label.
- **Stale flyout label if the menu is read without opening** → set the label at
  build time as well as on `aboutToShow`; tests emit `aboutToShow` to exercise
  the live path.
- **Selection changes while a menu is open** → the label updates on the next
  open; menus close on selection changes, so this is not reachable in practice.
- **File-size caps** → `frame_menus.cpp` is at 1171/1200 lines; keep the label
  provider to a few lines. The new test file avoids the 1400-line test cap.
