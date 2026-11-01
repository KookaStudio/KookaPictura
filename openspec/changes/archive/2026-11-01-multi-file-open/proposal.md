# Proposal

## Why

`File > Open` accepts one file at a time: `showOpenDialog()` calls
`getOpenFileName()`, which sets `QFileDialog::ExistingFile`. Photoshop opens one
document for every file in a Shift/Ctrl multi-selection, and this application
already does that for a workspace file drop (`FileDropRouter` starts one open per
path), so the dialog is the only single-file open entry point left (#1118).

## What Changes

- **`File > Open` selects multiple files.** A new `getOpenFileNames()` helper
  sets `QFileDialog::ExistingFiles` and returns every chosen path. The existing
  single-file helper keeps its behaviour for Open As, Place, and the other
  pickers.
- **One document per chosen file.** `showOpenDialog()` starts an open for every
  returned path. Each document follows the existing host rule (`Open Documents
  As Tabs` on: tabs; off: floating windows) and records its own path.
- **A failed file does not stop the rest.** The loop starts every path before any
  decode result arrives, so a corrupt selection cannot cancel its siblings.
- **No change to the other pickers.** Open As, Open As Smart Object, Place, and
  Open Recent stay single-file.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-lifecycle`: the "Open documents" requirement now covers a
  multi-file selection from the File > Open dialog, with one document per file
  and independent failures.

## Impact

- `crates/pictura-app/cpp/dialogs.{h,cpp}`, `cpp/frame.cpp`.
- `crates/pictura-app/cpp/tests/tst_multi_file_open.cpp` and
  `cpp/tests/CMakeLists.txt`.
- `docs/10-workflow-io/open-and-new.md`, `docs/dev/STATE.md`.
- No new dependencies; no engine or codec change.
