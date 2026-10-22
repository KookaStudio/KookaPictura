# Tasks

## 1. The descriptor's types and the bridge

- [x] 1.1 Add the shared cxx structs `FilterParamRow` and `FilterDescriptorRow` (including `slots`, `path`, `menu`, `dialog` and `map_after_ok`) and a `filter_descriptors()` free function returning an empty table, and verify the crate builds and the C++ side sees the types
- [x] 1.2 If a `Vec<SharedStruct>` field does not compile against cxx-qt, switch to the flat fallback — a `row: i32` index on the parameter row, grouped on the C++ side — and verify it compiles
- [x] 1.3 Add `crates/pictura-app/src/cxxqt_object/filter_map/descriptor/mod.rs` with `FilterControl`, `FilterParam`, `FilterDescriptor`, `DialogKind`, `FilterParam::slots()`, `FilterDescriptor::slots()`, `FilterDescriptor::defaults()`, `descriptor(kind)` and `arity(kind)`, and verify `cargo nextest run -p pictura-app` still passes

## 2. Test plan and test scaffold

- [x] 2.1 Update `test-plan.md` in this change folder so every scenario in the delta has a row and every coverage cell names an existing test or a test this change creates
- [x] 2.2 Add the Rust test module for the descriptor and write `arity_is_the_descriptor_slot_count` (canonical count, descriptor arithmetic, every default run's length, the one-more refusal, and the one-fewer refusal only when the count exceeds one), and verify it fails against the empty table
- [x] 2.3 Write `mapping_defaults_come_from_the_descriptor`, and verify it fails against the empty table
- [x] 2.4 Write `whole_layer_comes_from_the_descriptor`, and verify it fails against the empty table; assert `hdr-toning` is not a filter kind
- [x] 2.5 Write `engine_defaults_agree_with_the_descriptor` (build the filter from the descriptor defaults and compare it to the engine's `Default`), and verify it fails against the empty table
- [x] 2.6 Write `descriptor_covers_every_mapping_arm`, and verify it fails against the empty table
- [x] 2.7 Write the Qt cases `rowsComeFromTheRustDescriptor` (menu rows only, each row's slot counts, dialog kind, map-after-OK flag and preview flags included, and the no-menu-row descriptor absent from `filterCommands()` but found by `filterCommandForKind`), `everyRowOpensOnItsDefaults` (shared-dialog rows with parameters only) and `eachKindOpensTheDialogItsDescriptorNames` in `tst_filter_menu.cpp`, and verify they fail against the empty table

## 3. The descriptor's table

- [x] 3.1 Move the 110 menu rows into the five contiguous family modules field for field, in the C++ table's order (`paint`, `blur`, `distort`, `render`, `stylize`), and verify a scripted comparison against the old C++ table reports no field difference except `shear`'s `initial`, which now records the two end points the C++ curve widget opened on
- [x] 3.2 Add the no-menu-row `blur-gallery` descriptor (path `{"Filter", "Blur Gallery"}`, `menu: false`) and the shared Blur Gallery effects tail, and verify its slot count is 62 and it has no menu row
- [x] 3.3 Add the `whole_layer` flag to every descriptor, taking it from the 15-kind list in `filter_tools.rs` minus `hdr-toning` (14 filter kinds), and verify the two sets match
- [x] 3.4 Add the `dialog` kind and the `map_after_ok` flag to every descriptor, taking the dialog kinds from `frame_menus_filter.cpp`'s 11-kind chain and the flag from its post-OK Displace branch, and verify the two sets match

## 4. The mapping reads the descriptor

- [x] 4.1 Change the mapping's slot helpers to take `f64` defaults, move the mapping match to `filter_map/mapping.rs` (the descriptor-default reads push `filter_map.rs` past the cap), look up the descriptor once and pass its `slots()` to the existing `arity!` checks (no literal arity remains), update the mapping's docstring to distinguish canonical counts from variable lists, and verify the crate compiles
- [x] 4.2 Replace the arms' inline defaults with reads of the descriptor's default vector, and verify `cargo nextest run -p pictura-app` passes
- [x] 4.3 Make `lighting()` read the light defaults from the descriptor (which carries `Light::default()`'s values) and drop its `0.0` fallbacks, and verify the lighting and variable-list cases pass
- [x] 4.4 Give the Blur Gallery readers their defaults from the descriptor and replace the `FIELD_BLUR_SLOTS`, `IRIS_BLUR_SLOTS`, `TILT_SHIFT_SLOTS`, `PATH_BLUR_SLOTS`, `SPIN_BLUR_SLOTS` and `BLUR_GALLERY_SLOTS` canonical totals with descriptor lookups (keeping only the record strides, motion/strobe section sizes and `GALLERY_TAIL`), and verify the gallery and variable-list cases pass
- [x] 4.5 Move `filter_param_arity` and `commit_params` beside the descriptor, re-export them from `filter_map.rs` for `filter_tools.rs`, remove the Blur Gallery canonical-total re-exports, delete `filter_map/arities.rs`, make `helpers::filter_from_kind` build the descriptor's default vector and pass it through `commit_params`, rewrite `every_kind_defaults_from_empty_and_rejects_wrong_arity` to iterate the descriptor instead of `FILTER_ARITIES`, update the gallery tests that use the removed canonical totals, and verify the crate compiles and the Lens Blur commit still stores More Accurate (`tst_lens_blur_dialog::fasterStillCommitsMoreAccurate`)
- [x] 4.6 Replace `filter_preview_needs_whole_layer`'s list with the descriptor's flag (keeping the function as the callers' entry point), drop HDR Toning from its docstring, update the call sites and the `impl_filters`/`filter_tools` tests that import it, and verify the whole-layer case passes
- [x] 4.7 Delete the stale comment in `filter_map.rs` that says Custom "is left unmapped", and verify the comment is gone and `custom` still maps

## 5. The C++ rows come from the descriptor

- [x] 5.1 Replace `cpp/filter_commands.cpp` with the adapter: add `slotCount` to `FilterParamSpec`, add the dialog kind and map-after-OK flag to `FilterCommandSpec`, have `filterCommands()` return the `menu: true` rows in the old table order and `filterCommandForKind` search all rows including `blur-gallery`, correct the header's menu-order and path comments, and verify the menu builds the same paths and rows in the same table order
- [x] 5.2 Delete `slotCount()` from `cpp/filter_param_controls.cpp` and take each control's slot count from `FilterParamSpec::slotCount` in `values()`, `setValues()` and `scaledValues()`, and verify `everyRowsKindIsSupportedAndArityMatches` and the dialog tests still collect the right number of slots
- [x] 5.3 Switch `cpp/frame_menus_filter.cpp` on the descriptor's dialog kind and the Displace chooser on its map-after-OK flag, and verify each kind opens its own dialog and Displace still asks for its map after OK (`tst_displace`)
- [x] 5.4 Drop the `filter_kind_supported` skip in `wireFilterMenu` and verify every row is registered and implemented; do not delete `filter_kind_supported`, because `filter_gallery_dialog.cpp` still calls it

## 6. Docs and roadmap

- [x] 6.1 Update `ROADMAP.md:272` so the "110 filters" item's evidence path names the descriptor module instead of the deleted `arities.rs`
- [x] 6.2 Refresh `docs/dev/STATE.md`'s filter-mapping paragraph to describe the descriptor as the single source, in a commit carrying `TASK-ALLOWS-DOCS`
- [x] 6.3 In `docs/dev/doc-code-parity-gaps.md`, remove FILT-3 (fixed by `98fa943e`; the Radial Blur kernel now reads its Blur Center), refresh FILT-4 and FILT-6 to the files that now hold them (the Smart Sharpen descriptor and the Colored Pencil mapping arm), update the FILT-1 note that cites `arities.rs:80`, and refresh SYS-2's `filter_tools.rs` arity-check evidence, in a commit carrying `TASK-ALLOWS-DOCS`

## 7. Integration verification

- [x] 7.1 Run `openspec validate --all --strict` and verify the change validates
- [x] 7.2 Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` and verify both are clean
- [x] 7.3 Run `cargo nextest run --workspace` and verify the Rust layer passes
- [x] 7.4 Build once and run the Qt layer offscreen (`ctest --test-dir build -R '^tst_' --output-on-failure`), verifying `tst_filter_menu` and `tst_lens_blur_dialog` pass
- [ ] 7.5 Run `scripts/check-test-plan.py`, `scripts/check-file-size.sh` and `scripts/guard.sh` and verify they pass
