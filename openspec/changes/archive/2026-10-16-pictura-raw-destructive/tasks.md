# Tasks

## 1. Engine: destructive apply

- [x] 1.1 Add `apply_pictura_raw_destructive` in `document_ops/pictura_raw.rs`: resolve the layer, reject a missing path / group / adjustment / smart object / empty channel set / pixel-locked layer / non-positive rect / non-RGB-or-Grayscale mode, build an RGB(A) source from the layer's own channels over `rect`, run `pictura_adjust::render_pictura_raw`, and write the color planes back in place while preserving alpha, channel set, rect and Background identity (no store clearing, no `raw_channels` change). Verify with an inline-module unit test that a plain layer's pixels change, alpha and rect are unchanged, `smart_object` stays `None`, and its settings readback is empty.
- [x] 1.2 Handle Grayscale by replicating channel 0 into the buffer and writing the result back to channel 0 only; verify with a unit test on a Grayscale layer.
- [x] 1.3 Cover the refusals (group, adjustment layer, smart object, pixel-locked layer, CMYK document) and the Background case in unit tests: refused targets leave the document unchanged; a Background is filtered but stays flagged, named, and without an added alpha channel.
- [x] 1.4 Unit test that a 16-bit document edited destructively saves the edited pixels via a save/read round-trip, proving the writer re-widens the edited plane (`write.rs:786-793`) without any explicit store clearing.
- [x] 1.5 Re-export the new op from `document_ops/mod.rs` and `src/lib.rs`; confirm it is reachable as `pictura_render::apply_pictura_raw_destructive`.

## 2. Bridge: dispatch and predicate

- [x] 2.1 In `cxxqt_object/impl_pictura_raw.rs`, drop the convert-first block from `apply_pictura_raw_filter` and dispatch: embedded smart object with payload → `apply_pictura_raw`; raster leaf → `apply_pictura_raw_destructive`; otherwise false. Keep the one `"Pictura Raw"` record and the recomposite. Verify with the rewritten Qt Test in 4.1.
- [x] 2.2 Declare `layer_can_apply_pictura_raw(path)` in `cxxqt_object.rs` and implement it in `impl_pictura_raw.rs`, mirroring the op: embedded smart object with payload, or a non-group, non-adjustment, non-smart-object RGB/Grayscale raster layer with a color channel, a positive rect, and no pixel lock (no bit-depth check). Verify it compiles into the generated cxx-qt header and is callable from C++ in 4.3.

## 3. Menu enablement

- [x] 3.1 In `cpp/frame_menus.cpp`, replace the `FilterPicturaRaw` enabled provider's `can_convert_to_smart_object || can_replace_smart_object_contents` with `view->layer_can_apply_pictura_raw(path)`. Verify the provider no longer references the conversion predicate (grep) and the menu builds.

## 4. GUI tests

- [x] 4.1 Rewrite `tst_pictura_raw.cpp`: a plain raster layer applies destructively (pixels change, stays a plain layer, settings readback empty, exactly one `"Pictura Raw"` state); a Background applies destructively and stays a Background; an embedded smart object gains/refreshes the camera-raw smart filter and retains settings. Verify with `ctest -R '^tst_pictura_raw'`.
- [x] 4.2 Update the setups in `tst_acr_dialog.cpp` (`applyAcrossTabsRecordsOneState`) and `tst_layers_smart_filters.cpp` (both apply calls) to convert the layer to a smart object before applying, preserving each test's intent. Verify with `ctest -R '^tst_acr_dialog|^tst_layers_smart_filters'`.
- [x] 4.3 Add cases to the existing `tst_pictura_raw.cpp` (no new file, so no `tests/CMakeLists.txt` entry): the `Filter > Pictura Raw…` action is disabled with no document open and enabled for a selected raster layer, and `layer_can_apply_pictura_raw` is false for a group; verify with `ctest -R '^tst_pictura_raw'`.

## 5. Integration verification

- [x] 5.1 Run `cargo nextest run -p pictura-render -p pictura-app`, then the CTest Qt suites, then `bash scripts/verify-fast.sh`; confirm all pass and report any golden/fixture note.
- [x] 5.2 Run `openspec validate pictura-raw-destructive --strict` and confirm the change validates.
