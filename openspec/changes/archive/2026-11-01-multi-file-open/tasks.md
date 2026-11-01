# Tasks

## 1. Test seams

- [x] 1.1 Write `multiSelectOpensOneDocumentPerFile`, `multipleSelectionFollowsTheTabsPreference`, `singleSelectionOpensOneDocument`, and `failedFileDoesNotStopTheRest` in `crates/pictura-app/cpp/tests/tst_multi_file_open.cpp`; verify they fail before the dialog and frame changes.
- [x] 1.2 Register `tst_multi_file_open` in `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 2. Multi-file Open

- [x] 2.1 Add `getOpenFileNames()` to `cpp/dialogs.{h,cpp}` and share the dialog builder with `getOpenFileName()`.
- [x] 2.2 Start every path returned by `getOpenFileNames()` in `PicturaMainWindow::showOpenDialog()` (`cpp/frame.cpp`).

## 3. Documentation

- [x] 3.1 Update `docs/10-workflow-io/open-and-new.md` and `docs/dev/STATE.md`.
- [ ] 3.2 Verify `bash scripts/guard.sh` and commit the docs with `TASK-ALLOWS-DOCS`.

## 4. Verification and archive

- [x] 4.1 Run `ctest --test-dir build -R '^tst_multi_file_open$' --output-on-failure` and fix every failure.
- [x] 4.2 Run `python3 scripts/check-test-plan.py`.
- [x] 4.3 Run `openspec validate --all --strict`, then `openspec archive multi-file-open`, and confirm `scripts/check-openspec-archived.sh` passes.
