# Tasks: swatches-session-persistence

## 1. Tests first

- [x] 1.1 `tst_session_store`: an absent key, a round-trip, and an
  unrelated-save preservation.
- [x] 1.2 `tst_swatches_panel`: an added swatch and an emptied panel survive a
  new window.

## 2. Session store

- [x] 2.1 `SessionState` `swatches` + `swatchesLoaded`; load and save.
- [x] 2.2 `PicturaMainWindow::saveSession` writes the live grid;
  `buildPanels(state)` restores it and wires `countChanged`.

## 3. Docs and verification

- [x] 3.1 Update the STATE entry; `openspec validate --all --strict`;
  `bash scripts/verify-fast.sh`; archive the change.
