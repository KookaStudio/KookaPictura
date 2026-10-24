# Tasks

Per the repo workflow, nothing is built or run until the code is finished.
Every "Verify" names the check that the final build and test run must pass.

## 1. Tests

- [x] 1.1 In `tst_document_float`, write `headerDragCarriesWindow`, `tearOutDragDocksOnStrip`, `dockIndicatorBetweenTabs`, `dockIndicatorWithoutTabs`, `dockDropWithoutTabs`, `dockIndicatorHidesOnLeave`, `handDragShowsIndicator`, and `dockZoneInactiveTiledOrOff`. Verify `python3 scripts/check-test-plan.py` finds every case the plan names.
- [x] 1.2 Rewrite `tearOutUnderWayland` and `floatTopLevelUnderWayland` for the carried window drag, switch the seam to `setWindowDragForTest`, and check in `ctrlDragDocksAtDrop` that Ctrl+drag carries no window. Verify the file stays under the 1400-line test cap (`bash scripts/check-file-size.sh`).

## 2. Implementation

- [x] 2.1 In `DocumentFloatWindow`, replace `movesBySystem()` / `setSystemMoveForTest()` / `startSystemMove()` with `movesByWindowDrag()` / `setWindowDragForTest()`, make `dragTo` hand a top-level float's plain drag to `onWindowDragRequested` at the drag threshold, and report each hand move through `onDragMoved`. Verify `headerDragCarriesWindow`, `floatTopLevelUnderWayland`, and `dragHeaderDocks`.
- [x] 2.2 In the frame, give `startWindowDockDrag` the carried window (the two `application/x-qt-mainwindowdrag-*` keys), skip `exec` once the button is up, and dock a recorded drop after `exec` returns (`applyPendingDock`). Verify `ctrlDragDocksAtDrop` and `tearOutDragDocksOnStrip`.
- [x] 2.3 In `tearOutFilter`, start the carried drag for a top-level float that moves by window drag, with the header centre as the grab offset. Verify `tearOutUnderWayland`, `tearOutFloats`, and `tearOutLastTab`.
- [x] 2.4 Replace `tabStripGlobalRect()` with `dockZoneGlobalRect()` and `dockIndexAt()`, add the `documentDockIndicator` line (`updateDockIndicator`, `hideDockIndicator`) and its theme rule, and drive it from `windowDropFilter` (including `DragLeave`), the hand drag, and the release. Verify `dockIndicatorBetweenTabs`, `dockIndicatorWithoutTabs`, `dockDropWithoutTabs`, `dockIndicatorHidesOnLeave`, `handDragShowsIndicator`, `dockZoneInactiveTiledOrOff`, `fileDropOnStripOpensFile`, and `tst_document_arrange::prefDisablesDocking`.
- [ ] 2.5 Verify `openspec validate --all --strict`, `bash scripts/check-file-size.sh`, `bash scripts/guard.sh`, and the full Qt Test layer.

## 3. Docs

- [x] 3.1 Rewrite parity row UI-14 in `docs/dev/doc-code-parity-gaps.md` for the window drag (the ponytail: a compositor without `xdg_toplevel_drag_v1` leaves the window in place during the drag). Verify the row count in the header still matches the table.
- [x] 3.2 Update the document windows entry in `docs/dev/STATE.md`. Verify `bash scripts/guard.sh` passes with `TASK-ALLOWS-DOCS` in the commit.
