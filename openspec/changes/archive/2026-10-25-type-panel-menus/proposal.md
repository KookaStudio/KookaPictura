# Proposal

## Why

Smoke testing the type-panel bundle (#517/#518) exposed both panel menus as
effectively empty in the live app. `rowsForPanel()` has no Character or
Paragraph rows, so `populatePanelMenu()` emits only Close / Close Tab Group;
the per-panel hooks then append Fractional Widths / Justification after Close.
The CS6 reference and `docs/02-ui-ux/panels/character-and-paragraph.md` (UI
surface table) require the full Character and Paragraph menus: the dynamic
shortcuts, OpenType submenu, Fractional Widths / System Layout, and the
Paragraph menu's Justification, hanging punctuation, composition, and
layout/feature sets.

## What Changes

- Add the full CS6 Character and Paragraph rows to the static panel-menu table.
- Route implemented commands through new `performPanelMenuAction` methods on the
  two panels: style toggles, OpenType features, Fractional Widths / System
  Layout, Reset Character; Roman Hanging Punctuation, Justification, the two
  composers, Reset Paragraph.
- Keep unimplemented rows present but disabled with the existing "not
  implemented yet" tooltip (vertical orientation, No Break, Middle Eastern
  Features, Hyphenation dialog, layout/feature-set engine choices).
- Sync each row's checked and enabled state from the panel controls on
  `aboutToShow`, so the menu and the buttons cannot drift.
- Keep Close / Close Tab Group as the final two entries.

## Capabilities

### New Capabilities

- `ui/type-panel-menus`: the CS6 Character and Paragraph panel menus, their
  live commands, disabled parity rows, and close ordering.

### Modified Capabilities

(none)

## Impact

- `crates/pictura-app/cpp/panels/panel_group_menu.cpp`: new rows, checkable
  submenu-child helper, and action routing.
- `crates/pictura-app/cpp/panels/character_panel.{h,cpp}` and
  `paragraph_panel.{h,cpp}`: menu state sync, `performPanelMenuAction`, and the
  two reset paths.
- `crates/pictura-app/cpp/tests/tst_character_paragraph_panels.cpp`: live-menu
  tests and updated existing menu tests.
- No model, codec, renderer, docs, dependency, or file-size-allowlist changes.
