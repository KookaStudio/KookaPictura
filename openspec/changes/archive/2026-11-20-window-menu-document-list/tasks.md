# Tasks

## 1. Tests

- [x] 1.1 Add `windowMenuListOrder`, `windowMenuFullLabels`, `windowMenuChecksActive`, `windowMenuActivatesHosts`, `windowMenuFollowsChanges`, `windowMenuListIsLast`, and `windowMenuEmptyWithoutDocuments` to `crates/pictura-app/cpp/tests/tst_document_windows.cpp`, driving the list through `emit menu->aboutToShow()`; verify `python3 scripts/check-test-plan.py` accepts every row and the suite compiles.
- [x] 1.2 Confirm the existing `tst_ui_scale` cases cover the no-document placement scenario and the new `windowMenuListIsLast` covers the with-document order; verify `QT_QPA_PLATFORM=offscreen python3 scripts/cpu-budget.py exec -- ctest --test-dir build -R '^tst_ui_scale$' --output-on-failure` passes.

## 2. Window menu document list

- [x] 2.1 Add `windowDocumentActions_` and `refreshWindowDocumentList(QMenu*)` to `MenuRegistrar`, appending a separator and one checkable `windowDocumentEntry` action per `DocumentRegistry::windows()` row — the separator (`windowDocumentSeparator`) and rows are tracked together in `windowDocumentActions_` — with `fullLabel()` text (ampersands doubled), the active check, and an activation guard; verify the new `document-windows` cases pass.
- [x] 2.2 Wire the Window menu's `aboutToShow` in `MenuRegistrar::buildMenus()` so the list is rebuilt on every open; verify the list appears after the UI Size row and disappears with no documents.
- [x] 2.3 Build and run the targeted suites: `cmake --build build --target tst_document_windows tst_ui_scale` then `QT_QPA_PLATFORM=offscreen python3 scripts/cpu-budget.py exec -- ctest --test-dir build -R 'tst_(document_windows|ui_scale)' --output-on-failure`.

## 3. Close-out

- [x] 3.1 Tick the `#923` item in `ROADMAP.md`; verify the file still links every remaining unchecked feature item.
- [ ] 3.2 Run `bash scripts/verify-fast.sh` and fix any fmt, clippy, file-size, guard, or OpenSpec failure.
- [x] 3.3 Run `openspec validate --all --strict`, then `openspec archive window-menu-document-list`; verify `python3 scripts/check-openspec-archived.sh` passes.
