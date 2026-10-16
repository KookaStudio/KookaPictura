# Tasks

## 1. Dialog default from the clipboard

- [x] 1.1 In `crates/pictura-app/cpp/new_document_dialog.cpp`, open the dialog with Document Type `Clipboard` when `QGuiApplication::clipboard()->image()` is non-null (Default Photoshop Size otherwise), reusing the clipboard probe already made in `buildUi`; verify a manual `./build/pictura` run or the new Qt Test shows the clipboard dimensions.
- [x] 1.2 Add a `clipboardImagePreselectsClipboardType` case to `crates/pictura-app/cpp/tests/tst_new_document.cpp` covering both an image on the clipboard (type Clipboard, width/height from the image) and an empty clipboard (default type, Clipboard entry disabled), clearing the clipboard between; verify with `ctest --test-dir build -R '^tst_new_document' --output-on-failure`.

## 2. Integration

- [x] 2.1 Run `bash scripts/verify-fast.sh` (fmt, clippy, nextest, C++ self-test, Qt Test, file-size, guard, openspec) and `openspec validate new-document-clipboard-size --strict`; confirm all green.
