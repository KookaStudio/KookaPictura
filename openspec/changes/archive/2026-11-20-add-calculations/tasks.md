# Tasks

## 1. Engine kernel

- [x] 1.1 Add a public grayscale accessor to `ApplyImagePlanes` using the Grayscale conversion weights (Rec.601, matching Image ▸ Mode ▸ Grayscale) and verify the existing Apply Image suite still passes: `cargo nextest run -p pictura-render --test apply_image`.
- [x] 1.2 Add `pictura-render/src/calculations.rs` with the two-source blend, per-source Invert, Opacity toward Source 1, mask scaling toward black, Add/Subtract arithmetic, and 8-bit output; export it from `lib.rs` and verify with `cargo check -p pictura-render`.
- [x] 1.3 Write the engine tests `add_scale_two_averages`, `subtract_honours_scale_and_offset`, `layer_modes_match_the_compositor`, `invert_applies_per_source`, `opacity_mixes_toward_source_one`, `gray_matches_grayscale_conversion`, `alpha_channel_as_source`, `sixteen_bit_sources_keep_precision`, `mask_hides`, `inverted_mask_reveals_dark`, `selection_as_mask`, and `no_mask_applies_everywhere`; verify with `cargo nextest run -p pictura-render --test calculations`.

## 2. App bridge and result writers

- [x] 2.1 Extend the existing `image_apply_image` bridge (it owns `ApplyImageSnapshot`) with the calculations preview and apply entry points, and add the `save_calculations_channel` helper in `impl_selection.rs`: the preview returns full-resolution grayscale bytes and size, and the apply entry point handles Selection and New Channel; verify with `cargo check -p pictura-app`.
- [x] 2.2 Add the grayscale new-document writer used by the New Document result, mirroring the existing File ▸ New grayscale construction, and a Rust unit test `calculations_document_fills_gray_layer`; verify with `cargo nextest run -p pictura-app calculations_document`.
- [x] 2.3 Confirm the existing `crates/pictura-app/cpp/api/filters_bridge.h` already includes the generated `image_apply_image.cxxqt.h`, so the new bridge entry points reach the dialog without a manual declaration; verify with `cargo check -p pictura-app` and `cmake --build build --target pictura_shell`.

## 3. Dialog

- [x] 3.1 Add `calculations_dialog.{h,cpp}` with the two pickers, Gray/colour/Transparency/Layer Mask/alpha channel lists, mask group with Selection, Blending without Dissolve, Opacity, Scale/Offset rows, Result, and the preview image; add it to `cmake/sources/adjustments.cmake` and verify it builds with `cmake --build build --target pictura_shell`.
- [x] 3.2 Wire the Image menu entry in `menu_image.cpp` with its enabled provider (document present, not Indexed/Bitmap); the handler owns the commit after Accepted: Selection and New Channel through the bridge, New Document by creating a `PictureView`, calling the writer, and adding the document; the dialog refuses acceptance at the cap and names the 56-channel limit; verify with `cmake --build build --target pictura_shell` (the Qt suite follows in 3.3).
- [x] 3.3 Write `crates/pictura-app/cpp/tests/tst_calculations.cpp` covering `menuRowIsLive`, `sourcesMatchDimensions`, `channelListHasGrayAndNoComposite`, `scaleOffsetFollowMode`, `defaults`, `modeList`, `disabledModes`, `resultNewDocument`, `resultNewChannel`, `resultNewChannelUndo`, `resultNewChannelAtCap`, `resultSelection`, `resultSelectionReplaces`, `resultsAgree`, `previewFollowsControls`, `previewOff`, `cancelChangesNothing`, and `previewMatchesCommit`; register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt` and verify with `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_calculations$' --output-on-failure`.

## 4. Command catalogue

- [x] 4.1 Mark `image.calculations` implemented in the catalogue, add `ImageCalculations` to the frozen const names, add the id to `modal_ids()`, regenerate `command_ids.h`, update the catalogue and const-name goldens, and update `default_catalogue_shape_is_stable` to 679 rows / 469 implemented / 389 const names; verify with `python3 scripts/gen-command-ids.py --check` and `cargo nextest run -p pictura-script`.
- [x] 4.2 Confirm the script loader treats `image.calculations` as a live, modal, recordable AppNative command with no parameters (like Apply Image); verify with `cargo nextest run -p pictura-script` and the `tst_calculations::menuRowIsLive` case.

## 5. Documentation and registers

- [x] 5.1 Update `docs/08-selection/channel-based-masking.md`: resolve the Result-default and Selection-combine open questions, record the mask/opacity, preview, and 8-bit narrowing decisions as inferred, mark SEL-013's Calculations behaviour shipped where it applies, and replace the stale `pictura_channels::calc` module-map line with the shipped `pictura-render::calculations` module.
- [x] 5.2 Flip parity register row SEL-2 to SHIPPED with its evidence and issue link, tick `ROADMAP.md`'s Calculations item, and update `docs/dev/STATE.md`; verify with `bash scripts/guard.sh` and a docs commit carrying `TASK-ALLOWS-DOCS`.

## 6. Integration

- [x] 6.1 Run `bash scripts/verify-fast.sh` and fix any failure without weakening a test.
- [x] 6.2 Validate the change with `openspec validate add-calculations --strict` and archive it in the same pull request with `openspec archive add-calculations`.
