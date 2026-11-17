# Tasks

## 1. Engine availability predicate

- [x] 1.1 Write `blend_availability_32_bit`, `blend_availability_lab`,
  `blend_availability_lab_32_bit_intersection`, `blend_availability_bitmap_indexed`
  and `blend_availability_unrestricted_modes` in
  `crates/pictura-core/src/tests.rs` before 1.2. The first build and run is the
  final verification (8.1); do not build per task.
- [x] 1.2 Implement `BlendMode::allowed` in `crates/pictura-core/src/blend.rs`
  with the 15-mode HDR set, the 8 Lab exclusions and the Normal-only
  Bitmap/Indexed rule, evaluating the Lab and 32-bit restrictions conjunctively
  (the malformed Lab+32 case is their intersection); verify at 8.1.

## 2. Engine bridge

- [x] 2.1 Write `allowed_blend_mode_keys_follow_mode_and_depth` in
  `crates/pictura-app/src/cxxqt_object/engine_tables/blend_modes/imp.rs` over a
  pure colour-mode/depth key helper, before 2.2. This is a helper test, not the
  bridge; the bridge read is covered by the Qt combo tests. The first build and
  run is 8.1.
- [x] 2.2 Add the key helper and the `allowed_blend_mode_keys(view)` free
  function to the `engine_tables` bridge, reading `document_color_mode` and
  `document_bit_depth`; declare the view type in the bridge
  (`include!("pictura_app/src/cxxqt_object.cxxqt.h")` and
  `type PictureView = super::PictureView;`, following `channels.rs`); verify at
  8.1.

## 3. Blend gate helper and Layers panel

- [x] 3.1 Write `layersComboOffersHdrSubset`, `layersComboFollowsLab`,
  `layersComboFollowsIndexed`, `layersComboRestoresAllModes`,
  `layersComboKeepsUnsupportedCurrent` and `layersComboLeavesGroupPassThrough`
  in `crates/pictura-app/cpp/tests/tst_blend_mode_list.cpp` before 3.2; include
  `api/document_bridge.h` for `convert_image_mode`, `convert_to_indexed` and the
  32-bit setup, and create 32-bit documents at 32 bits where the test does not
  need a conversion. The first build and run is 8.2.
- [x] 3.2 Add `allowedBlendModes` and `syncBlendCombo` to
  `crates/pictura-app/cpp/blend_modes.{h,cpp}`, replace the two-line
  `findData`/`setCurrentIndex` block in `LayersPanel::syncControls` with the
  `syncBlendCombo` call and its index apply, and run it on every `syncControls`
  with an empty key when no row is selected; ignore the group `"pass"` sentinel;
  reword the `blend_modes.h` "combo index is the engine index" comment. Verify
  at 8.2 and `bash scripts/check-file-size.sh` keeps `panels/layers_panel.cpp`
  at 1261 lines, within its 1262-line allowlist entry.

## 4. New Layer dialog

- [x] 4.1 Write `newLayerDialogFollowsGate` in
  `crates/pictura-app/cpp/tests/tst_blend_mode_list.cpp` for a Lab and a 32-bit
  document, the no-view all-27 case, and the `group=true` New Group case,
  before 4.2. It cannot compile until 4.2 adds the view parameter; the first
  build and run is 8.2.
- [x] 4.2 Pass the appended defaulted `PictureView* view = nullptr` through
  `LayerNewDialog`'s constructors and `get`, populate the Mode combo from
  `allowedBlendModes`, and look the neutral flag up by key; update the four
  production `get` sites (3 in `layers_panel_actions.cpp`, 1 in
  `fill_layer_editor.cpp`); the `tst_fill_layer_editor.cpp` call keeps the null
  default. Verify `newLayerDialogFollowsGate` and the existing
  `layerCombosFollowEngineList` at 8.2.

## 5. Layer Style dialog

- [x] 5.1 Write `layerStyleCombosFollowGate` and
  `layerStyleKeepsUnsupportedCurrent` in `tst_blend_mode_list.cpp`; update
  `layerStyleComboIsEngineIndexed` to assert the engine index in `itemData`
  (the combo no longer has empty item data); extend
  `tst_layer_style::groupBlendingOptions` to assert the group `blending.mode`
  combo stays at its default with no appended `pass` entry. Write these before
  5.2; the first build and run is 8.2.
- [x] 5.2 Rewrite `LayerStyleDialog::bindBlendMode` to add only allowed entries,
  store the engine index in `itemData`, write `itemData` back, append the stored
  index as a disabled entry when it is unavailable, and keep the
  `std::isfinite` guard so a group/background NaN leaves the combo default and
  appends nothing; reword the `blend_modes.h` index comment if 3.2 has not done
  it. Verify the two new tests, `layerStyleComboIsEngineIndexed`,
  `tst_layer_style`, and `tst_layer_style::groupBlendingOptions` at 8.2.

## 6. Fade

- [x] 6.1 Update `FadeTest::labDisablesRestrictedModes` to the documented eight
  modes, and add `hdrDisablesRestrictedModes` (create the document at 32 bits;
  include `api/document_bridge.h` if the test converts one) and
  `eightBitOffersAllModes`; write them before 6.2. The first build and run is
  8.1/8.2.
- [x] 6.2 Replace `fade/imp.rs::mode_allowed`'s private list with
  `BlendMode::allowed(document_color_mode(doc), document_bit_depth(doc))`, add
  `Subtract` and `Divide` to the Rust `lab_refuses_the_eight_restricted_modes`
  loop, and rename the test; verify `cargo test -p pictura-app lab_refuses` and
  `tst_fade` at 8.1/8.2.

## 7. Contract documentation

- [x] 7.1 Correct the five 32-bit count occurrences in
  `docs/05-layers/blend-modes.md` (lines 88, 275, 356, 392, 393; 14 -> 15,
  13 -> 12) and amend the 32-bit/Lab "refuse or convert, never silently blend"
  edge case to state the interim disabled-current-mode policy pending #490;
  also correct the stale six-mode Lab lists in
  `crates/pictura-app/src/cxxqt_object/fade.rs`,
  `docs/06-filters/filters-overview.md`, and `docs/dev/STATE.md` to the
  documented eight modes;
  verify `rg -n '14 modes|14 available|13 unavailable|13 unsupported'
  docs/05-layers/blend-modes.md` returns nothing, the printed list still has 15
  names, and the edge case names #490.
- [x] 7.2 Tick `32-bit and Lab blend-mode menu gating (#589)` in
  `ROADMAP.md` and mark LAY-11 `SHIPPED` with its evidence in
  `docs/dev/doc-code-parity-gaps.md`; verify the road map line starts `- [x]`
  and the register row reads `SHIPPED`.
- [x] 7.3 Commit the documentation updates separately as
  `docs: correct 32-bit blend-mode availability counts (#589) TASK-ALLOWS-DOCS`;
  verify `bash scripts/guard.sh` passes on that commit.

## 8. Integration verification

- [ ] 8.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo nextest run -p pictura-core -p pictura-app`; this is the first Rust
  build/run after tasks 1-6, and all must pass.
- [ ] 8.2 Build once and run
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(blend_mode_list|fade|layer_style|layers)' --output-on-failure`;
  this is the first Qt build/run after tasks 3-6, and the suites must pass.
- [ ] 8.3 Run `openspec validate --all --strict` and
  `python3 scripts/check-test-plan.py`; verify both pass.
- [x] 8.4 Archive the change with
  `openspec archive blend-mode-menu-gating`; verify
  `bash scripts/check-openspec-archived.sh` passes.
- [ ] 8.5 Run `bash scripts/verify-fast.sh`; verify the full fast gate passes.

> Integration gates 8.1–8.3 and 8.5 are deferred to CI in this PR; no local
> test run was performed for the archive.
