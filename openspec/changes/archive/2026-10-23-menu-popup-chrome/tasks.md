# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_design_system::menuPopupChrome`: render a menu with a red
  icon, assert the top-left popup pixel is transparent and the icon's left edge
  is at least 8 px from the popup edge.
- [x] 1.2 Add `QMenu::icon` and `QMenu::indicator` to the stylesheet-selector
  coverage test.

## 2. Implementation

- [x] 2.1 Add `border-radius: 4px` and `padding: 2px` to the `QMenu` rule.
- [x] 2.2 Add `QMenu::icon { left: 10px; }` and `QMenu::indicator { left: 10px; }`.
- [x] 2.3 Set `WA_TranslucentBackground` on `QMenu` in the application style's
  `polish` override.

## 3. Verification

- [x] 3.1 Build and run `tst_design_system`, `tst_command_tree`, and
  `tst_info_panel`.
- [x] 3.2 `openspec validate --all --strict`
- [x] 3.3 `python3 scripts/check-test-plan.py`
