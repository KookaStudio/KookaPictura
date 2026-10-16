# Tasks: export-as-dialog

## 1. Contract

- [x] 1.1 `docs/10-workflow-io/export-formats.md`: add an **Export As (CC)**
  section — the dialog surface, the PNG/JPG/PNG-8/GIF format set, Image Size and
  Resample, Canvas Size, and per-layer output — with its provenance
  (`TASK-ALLOWS-DOCS`).

## 2. Engine / bridge

- [x] 2.1 `crates/pictura-app/src/cxxqt_object/web_export.rs`: add
  `layer_flattened(view, i)` — composite the top-level layer alone on a scratch
  document and crop to the layer's bounds (union for a group), sRGB RGBA.
- [x] 2.2 Rust `#[test]` for `layer_flattened`: a two-layer document exports one
  layer's pixels on transparency at its bounds; a group exports its subtree.

## 3. The dialog

- [x] 3.1 `export_as_dialog.{h,cpp}`: rebuild `ExportAsDialog` on
  `QVector<ExportEntry>` — the layer list with checkboxes and thumbnails, the
  preview + zoom, File Settings (Format + Transparency/Quality), Image Size
  (linked W/H/Scale + Resample), Canvas Size (+ Reset), and the debounced,
  capped size estimate. Expose `encoded(i)`, `sized(i)`, `saveTo(i, path)`,
  `suffix()`, `checkedEntries()` for tests.
- [x] 3.2 PNG-8 / GIF encoding through `web_quantize` / `web_encode_gif`;
  PNG / JPG through `QImageWriter`.
- [x] 3.3 `exportAsFromView`: build the entries from `web_flattened` +
  `layer_flattened`, then the file-vs-folder export flow and naming.

## 4. Self-test and Qt Test

- [x] 4.1 `selftest_session.cpp`: drop the `ExportAsDialog` quality-gating
  assertions from `lpr_raster_export` (code 532); keep the Save / Quick Export
  assertions. Exit code 532 stays (the check shrinks).
- [x] 4.2 New `crates/pictura-app/cpp/tests/tst_export_as.cpp`: formats and
  suffixes, PNG-8/GIF round-trip, Transparency/Quality gating, Image Size
  linking, Resample list, Canvas Size + Reset, the layer list, and `saveTo`
  byte equality. Add it to `CMakeLists.txt`.

## 5. Verification

- [x] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo nextest run -p pictura-app` (or workspace), and `cargo test --workspace --doc`.
- [x] 5.2 CMake build + `ctest --test-dir build -R '^tst_'` (new suite included).
- [x] 5.3 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh`; `openspec validate
  --all --strict`.
