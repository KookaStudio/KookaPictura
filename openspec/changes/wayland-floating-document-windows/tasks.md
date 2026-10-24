# Tasks

Per the repo workflow, nothing is built or run until the code is finished.
Every "Verify" names the check that the final build and test run must pass.

## 1. Tests

- [x] 1.1 In `tst_document_float`, write `tearOutUnderWayland`, `ctrlDragDocksAtDrop`, `ctrlDragRefusedWithoutDocking`, `ctrlDragElsewhereKeepsFloat`, `fileDropOnStripOpensFile`, `headerDoubleClickDocks`, and `floatTopLevelUnderWayland`. Reset the system-move seam and the dock-drag hook in `cleanup()`. Verify `python3 scripts/check-test-plan.py` finds every case the plan names.
- [x] 1.2 Rewrite `floatHostingFollowsPlatform` so the unforced float is a `Qt::Tool` top-level parented to the frame on every platform. Verify the file stays under the 1400-line test cap (`bash scripts/check-file-size.sh`).

## 2. Implementation

- [x] 2.1 Add `PanelFloat::childOverlayForcedForTest()`. Host `DocumentFloatWindow` as a frameless `Qt::Tool` top-level unless that hook forces a child. Verify `floatHostingFollowsPlatform` and the in-frame geometry cases in `tst_document_arrange` (`cascadeStepsWindows`, `cascadeKeepsHeadersReachable`) still pass.
- [x] 2.2 Add `DocumentFloatWindow::movesBySystem()`, `setSystemMoveForTest()`, and `startSystemMove()`. Make `dragTo` try the system move at the drag threshold. Verify `floatTopLevelUnderWayland` and `dragHeaderDocks`.
- [x] 2.3 In `tearOutFilter`, try the system move on the new float right after `beginDrag`. Verify `tearOutUnderWayland`, `tearOutFloats`, and `tearOutLastTab`.
- [x] 2.4 Add the header's Ctrl+drag (`onDockDragRequested`) and double-click (`onDockRequested`). In the frame, add the private MIME type, `startWindowDockDrag` (with `QDrag::exec` on the next event-loop turn), `setWindowDockDragForTest`, `tabStripGlobalRect()`, and `windowDropFilter` on `documentTabBar` and `documentTabs`. Verify `ctrlDragDocksAtDrop`, `ctrlDragRefusedWithoutDocking`, `ctrlDragElsewhereKeepsFloat`, `fileDropOnStripOpensFile`, `headerDoubleClickDocks`, `tst_document_arrange::prefDisablesDocking`, and the layer drop tests in `tst_layers_panel`.
- [ ] 2.5 Verify `openspec validate --all --strict`, `bash scripts/check-file-size.sh`, `bash scripts/guard.sh`, and the full Qt Test layer.

## 3. Docs

- [ ] 3.1 Rewrite parity row UI-14 in `docs/dev/doc-code-parity-gaps.md` as the remaining Wayland divergence (a plain header drag doesn't dock there). Verify the row count in the header still matches the table.
- [ ] 3.2 Update the document windows entry in `docs/dev/STATE.md`. Verify `bash scripts/guard.sh` passes with `TASK-ALLOWS-DOCS` in the commit.
