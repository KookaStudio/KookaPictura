# Tasks

## 1. Tests first

- [x] 1.1 Add `freshSessionKeepsPanelMenuHooks`,
  `workspaceSwitchKeepsPanelMenuHooks`, and
  `panelMoveKeepsPanelMenuHook` to
  `crates/pictura-app/cpp/tests/tst_workspaces.cpp`; confirm they fail before
  the hook registry exists. (Verified: all three failed with the old
  group-only resolution restored, then passed with the fix.)

## 2. Frame-owned hook registry

- [x] 2.1 Add `PicturaMainWindow::setPanelMenuHook(panelName, hook)` and
  `panelMenuHook(panelName)`, backed by a `QHash<QString,
  std::function<void(QMenu*)>>` keyed by the panel object name.
- [x] 2.2 In `buildPanels`, register the Actions, Character, and Paragraph hooks
  on the frame instead of on the groups it creates.

## 3. Group resolution

- [x] 3.1 Add `PanelGroup::owningFrame()` with the same docked/floating lookup
  `PanelColumn::owningFrame()` uses.
- [x] 3.2 In `updateHeaderMenu`'s `aboutToShow` handler, resolve the hook from
  the group first, then the owning frame, then the Actions fallback.

## 4. Verification and archive

- [x] 4.1 Run `cmake --build build --parallel` and
  `ctest --test-dir build -R '^tst_workspaces' --output-on-failure`.
- [x] 4.2 Run `python3 scripts/check-test-plan.py`, `openspec validate --all
  --strict`, and `bash scripts/verify-fast.sh`. (The fast gate's only failure is
  the pre-existing ImageMagick `ramp_Divide.rgba` oracle mismatch on the local
  7.1.2-27 Beta; every Rust, self-test, and Qt suite passed.)
- [x] 4.3 Update `docs/dev/STATE.md` and commit the docs with
  `TASK-ALLOWS-DOCS`.
- [x] 4.4 Archive the change with `openspec archive panel-menu-hooks-regroup`
  and confirm `scripts/check-openspec-archived.sh` passes.
