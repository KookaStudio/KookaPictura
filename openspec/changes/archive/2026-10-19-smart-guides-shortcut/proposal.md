# Proposal: smart-guides-shortcut

## Why

The `View > Show > Smart Guides` toggle added for #737 has no keyboard
shortcut. Photoshop binds it to `Ctrl+Shift+;` (`Command+Shift+;` on macOS), and
users expect to flip alignment feedback without opening the menu.

## What Changes

- Bind `View > Show > Smart Guides` to `Ctrl+Shift+;`; Qt renders that as
  `Command+Shift+;` on macOS.
- Record the binding in the View menu contract (`docs/02-ui-ux/menus.md`).
- Extend `tst_smart_guides::smartGuidesToggle` to assert the default binding.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `tools/smart-guides`: the visibility-toggle requirement gains the shortcut.

## Impact

- `crates/pictura-app/cpp/command_tree.cpp` — the default key sequence.
- `crates/pictura-app/cpp/tests/tst_smart_guides.cpp` — the assertion.
- `docs/02-ui-ux/menus.md` — the documented shortcut (commit needs
  `TASK-ALLOWS-DOCS`).
- No new dependencies.
