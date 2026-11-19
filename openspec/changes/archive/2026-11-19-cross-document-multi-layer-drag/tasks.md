# Tasks

## 1. Regression coverage

- [x] 1.1 Add `tst_layer_target::crossDocumentMultiCopySelectsCopies` in
  `crates/pictura-app/cpp/tests/tst_layer_target.cpp` (that file has room;
  `tst_layers_panel.cpp` is at its 1408-line allowlist ceiling): give the source
  two named layers and a multi-layer target, send a single-path layer drag to the
  destination tab, and assert both copies with source stacking order, both
  selected, the dragged row's copy active, one new history state, and the source
  unchanged. Add the group-anchor case (a selected group plus a sibling) and the
  empty-destination-target case (the copies land on top in source order). Add the
  Grayscale refusal: an RGB multi-selection dropped on a Grayscale tab leaves
  zero copies and the destination history unchanged. Verify it fails before the
  fix:
  `cmake --build build --target tst_layer_target &&
  QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_layer_target$' --output-on-failure`.

## 2. Batch copy bridge

- [x] 2.1 Replace `copy_layer_from_document` with
  `copy_layers_from_document(view, source, paths, dest_anchor, source_anchor)` in
  `crates/pictura-app/src/cxxqt_object/clipboard.rs` and `clipboard/imp.rs`:
  check read-only, color mode, and bit depth before `edit`, resolve every source
  path and drop duplicates before mutating, skip the ones that do not resolve,
  sort them into the source document's order, insert every copy at the same
  destination insertion point (never chain an anchor onto a created copy),
  process in forward order for a sibling anchor and in reverse order for a group
  anchor or an empty (or stale) destination target, compute the final indices
  from that fixed point, set the multi-path target with the created path for
  `source_anchor` active (topmost copy when absent), record one "Duplicate
  Layer" state, and return the created paths. Never return `Edit::None` after an
  insertion. Verify with `cargo build -p pictura-app`.
- [x] 2.2 Update the bridge declaration comment in
  `crates/pictura-app/src/cxxqt_object/clipboard.rs` to the plural function and
  its new parameters (the C++ header is generated from it). Verify with
  `cargo build -p pictura-app`.

## 3. App seam

- [x] 3.1 Replace `PicturaMainWindow::copyLayerFromDocument` with
  `copyLayersFromDocument(const void* source, const QString& dragged)` in
  `frame.h`/`frame.cpp`: resolve the source view, and for a dragged layer that
  belongs to the source's multi-layer target pass the expanded list, the
  destination anchor (`layerAnchor()`) and `dragged` as the source anchor to the
  bridge, then refresh the panel once. Keep the frame method minimal. Verify with
  `cargo build -p pictura-app` and a successful CMake compile of the app.
- [x] 3.2 Add the expand/filter free function (declaration in
  `file_drop_router.h`) in `file_drop_router.cpp`: expand `dragged` to the source
  target when it belongs to it, and drop every path whose selected ancestor is
  also in the list. It must stay in sync with the engine's
  `selected_paths(doc, paths, true)` rule (`paths.rs:146-172`), which is
  `pub(super)` and cannot be reused. Verify with a CMake compile of the app.
- [x] 3.3 In `FileDropRouter::routeLayerDrag`, pass `paths.first()` to
  `copyLayersFromDocument` and ignore only an empty list. Verify the existing
  single-layer case still passes:
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_layers_panel$' --output-on-failure`.
- [x] 3.4 Update `tst_layer_target::crossDocumentCopySelectsCopy` to the
  `copyLayersFromDocument(source, sourcePath)` call, and ADD a one-history-state
  assertion and an undo check (the test has none today). Verify:
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_layer_target$' --output-on-failure`.
- [x] 3.5 Make the new multi-layer test green, including the Grayscale refusal
  (zero copies, unchanged history), the group-anchor and the empty-anchor cases.
  Verify:
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_layer_target$' --output-on-failure`.
- [x] 3.6 In `tool_move.cpp` hand `pictura::layer_selection_anchor(*v)` to
  `makeLayerDragMime` instead of `active_layer_path()`, so a multi-layer target
  can start the cross-document drag. Verify with a CMake compile of the app, and
  manually drag with the Move tool from a multi-layer target onto another
  document's tab (the source keeps all layers, the destination gains the whole
  selection).

## 4. Verification

- [x] 4.1 Build the app and run the two affected suites:
  `cmake --build build --parallel &&
  QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(layers_panel|layer_target)$' --output-on-failure`.
- [ ] 4.2 Archive only once the code and tests are green
  (`openspec archive cross-document-multi-layer-drag`), then run
  `bash scripts/verify-fast.sh` and `openspec validate --all --strict`. The
  zero-open gate (`scripts/check-openspec-archived.sh`, run by `verify-fast.sh`)
  fails while the change is still open, so this step is not runnable mid-apply.
