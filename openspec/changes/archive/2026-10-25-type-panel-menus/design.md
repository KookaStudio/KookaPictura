# Design

## Context

- `rowsForPanel()` in `panels/panel_group_menu.cpp` is the one transcribed
  per-panel menu table. It has no `characterPanel` or `paragraphPanel` entry, so
  `populatePanelMenu()` emits only Close / Close Tab Group for those panels.
- `updateHeaderMenu()` builds that menu and calls the panel's hook on
  `aboutToShow`. `runPanelMenuAction()` currently routes only Layers, History,
  Actions, and adjustment ids.
- `CharacterPanel::updatePanelMenu()` and `ParagraphPanel::updatePanelMenu()`
  currently create and append their one item, so it lands after Close.
- The CS6 menus are recorded in the smoke screenshots and in
  `docs/02-ui-ux/panels/character-and-paragraph.md:90-91`.

## Goals / Non-Goals

**Goals**

- The live menus match the CS6 rows and grouping.
- Implemented commands work from the menu and match the panel controls.
- Menu check states follow the active layer and the panel controls.
- Close / Close Tab Group stay last.
- One history state per committed change, none from opening the menu.

**Non-Goals**

- Vertical-only Underline Left/Right, the Hyphenation dialog, No Break, the
  Middle Eastern Features submenu, and the layout/feature-set engine choices.
  They stay present and disabled; the parity register/spec can grow them later.
- Per-character selection styling (tracked separately by #927).
- Any model, codec, or renderer change.

## Decisions

### 1. Rows live in the static table

Add `characterPanel` and `paragraphPanel` entries to `rowsForPanel()` instead of
building them in the panels. `populatePanelMenu()` already appends Close / Close
Tab Group after the rows, so the close entries stay last without a special path.
The existing `dis`/`chk`/`chkAction`/`sub`/`kid` helpers cover the rows; add a
`chkKid` helper for checkable submenu children (the eight OpenType features).

Alternative: append the whole menu in the panel hooks. Rejected: it duplicates
the table and still needs a separate close-ordering path.

### 2. Route ids to panel-owned handlers

Extend `PanelGroup::runPanelMenuAction()` to route `character.*` and
`paragraph.*` ids to new `performPanelMenuAction(const QString&)` methods on the
current Character or Paragraph panel, mirroring the existing Layers/History
pattern. The static table already connects every row with an action id to the
dispatcher, so the hooks no longer create or connect actions.

### 3. Reuse the panel's one-shot apply paths

Menu handlers call the same paths as the controls:

- style and OpenType rows click the matching panel button, which already
  enforces the caps/position exclusivity and records one state;
- Roman Hanging Punctuation flips the existing checkbox;
- composer rows set the existing combo index;
- Justification opens the existing dialog;
- Fractional Widths writes the panel value and calls `apply()`; System Layout is
  its inverse and is a no-op when already off;
- Reset Character copies the model defaults over the panel-edited character
  fields (keeping the layer's family, style, size, and colour, a documented
  simplification) and records one state;
- Reset Paragraph writes the model paragraph defaults and records one state.

### 4. Sync state from the controls

`updatePanelMenu()` finds rows by their action id (recursing into submenus) and
sets checked/enabled from the matching control or `shown_`. This keeps the menu
and the buttons in step, and `System Layout` is always `!fractionalWidths`.
Unimplemented rows carry no action id, so `populatePanelMenu()` disables them
and attaches the not-implemented tooltip. The OpenType submenu is enabled only
when at least one of its children is enabled.

### 5. Test through the live menu

Tests obtain the actual `PanelGroup` header menu
(`panelWidgetMenu_<panel>`) after making the panel the active tab, call
`updatePanelMenu()` to sync, then assert rows, order, enabled state, and
`trigger()` behavior. The existing `fractionalWidthsMenu` and Justification tests
move from a bare `QMenu` to this helper and use `trigger()` (not `setChecked()`)
so the dispatcher runs.

## Risks / Trade-offs

- **Menu/control drift** → sync on every `aboutToShow`; handlers refresh the
  panel after applying. Tests assert both the action and the button state.
- **Double history state** → handlers reuse the existing one-shot paths; tests
  assert exactly one state and undo.
- **Disabled rows look incomplete** → the existing per-panel menu convention is
  explicit about unimplemented commands; the spec names them.
- **Menu dispatch mismatch** → ids are shared by the table and the handlers; a
  typo leaves the row enabled but inert. The content tests and the toggle tests
  catch a missing id.
