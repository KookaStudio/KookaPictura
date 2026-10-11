# Tasks

## 1. Code

- [x] 1.1 `setScreenMode`: Full Screen Mode With Menu Bar applies
  `!panelsHidden_`; Full Screen Mode starts with the menu bar and panels hidden
  and resets `fullScreenChromeShown_` on entry (except when re-selecting the
  mode).
- [x] 1.2 `togglePanelsHidden`: Standard and Full Screen With Menu Bar toggle
  `panelsHidden_`; Full Screen Mode toggles the transient reveal of the panel
  columns and the menu bar and re-lays the workspace frame.

## 2. Tests

- [x] 2.1 Update `tst_screen_modes::tabRevealsFullScreenChrome` to assert the
  menu bar reveal/hide and the transient restore.
- [x] 2.2 Add `tst_screen_modes::fullWithMenuBarKeepsPanels` asserting the
  retained menu bar/tools/panels, the hidden status bar, and the Tab toggle.

## 3. Docs and verification

- [x] 3.1 Update `docs/02-ui-ux/application-frame.md`'s Tab/full-screen bullet.
- [x] 3.2 Run `ctest --test-dir build -R '^tst_'`,
  `openspec validate --all --strict`, and `bash scripts/verify-fast.sh`.
