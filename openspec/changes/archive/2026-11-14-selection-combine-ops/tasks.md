# Tasks

## 1. Tests first

- [x] 1.1 Write codec tests for alpha names: ASCII and non-ASCII round-trip, 1006 fallback, unchanged resource bytes, psd-tools oracle.
- [x] 1.2 Write bridge tests for save/load operations, invert, names, mask writes, refusals, and state counts.
- [x] 1.3 Create `tst_selection_save_load` with dialog defaults, targets, operations, same-size filtering, New image, cross-document, refusal, and cancel cases.
- [x] 1.4 Extend `tst_channels_panel` with Ctrl+Shift/Alt clicks, drag-to-load, Alt-click Save, stored names, and one-state cases.
- [x] 1.5 Extend `tst_mask_edit` with Ctrl+Shift/Alt mask clicks and save-into-mask cases.
- [x] 1.6 Write `test-plan.md` mapping every scenario in this change to a test or a justified manual check.

## 2. Codec: channel names

- [x] 2.1 Add 1006 Pascal and 1045 Unicode alpha-name readers and writers in `crates/pictura-codec/src/image_resources.rs`.
- [x] 2.2 Export `alpha_names` and `set_alpha_names` from the codec.
- [x] 2.3 Add the psd-tools oracle test for named alpha channels.
- [x] 2.4 Run the codec tests.

## 3. Rust bridge: save and load

- [x] 3.1 Add target encoding and decoding for `new`, `alpha:<index>`, and `mask:<path>`.
- [x] 3.2 Implement `save_selection_to` for new and existing channels.
- [x] 3.3 Implement `load_selection_at` with invert and operation.
- [x] 3.4 Extend `load_channel_selection` with the operation.
- [x] 3.5 Implement mask target listing and save-into-mask with metadata preservation and `Edit::Regions`.
- [x] 3.6 Read and update channel names on save, new channel, and delete.
- [x] 3.7 Add coverage transfer methods for cross-document save and load.
- [x] 3.8 Add the New-image constructor.
- [x] 3.9 Update the `cxxqt_object.rs` declarations and every Rust and C++ call site.
- [x] 3.10 Run the app crate tests.

## 4. C++ dialog and Select menu

- [x] 4.1 Add `save_load_selection_dialog.{h,cpp}` and register it in the CMake sources fragment.
- [x] 4.2 Implement Save mode: Document, Channel, Name, Operation, same-size filtering, New.
- [x] 4.3 Implement Load mode: Document, Channel, Invert, Operation.
- [x] 4.4 Wire `SelectSave` and `SelectLoad` to the dialog.
- [x] 4.5 Run the new dialog suite.

## 5. Panels

- [x] 5.1 Channels panel: modifier chain, stored names, drag-to-load, Alt-click Save, 56-channel refusal feedback.
- [x] 5.2 Layers panel: Ctrl-prefixed combine on raster-mask thumbnails.
- [x] 5.3 Run the panel suites.

## 6. Verification and close-out

- [x] 6.1 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 6.2 Build the app and run the affected Qt suites.
- [x] 6.3 `bash scripts/verify-fast.sh`.
- [x] 6.4 Update `docs/dev/doc-code-parity-gaps.md` and `ROADMAP.md` in a `TASK-ALLOWS-DOCS` commit.
- [x] 6.5 `openspec validate --all --strict`.
- [x] 6.6 Archive `selection-combine-ops` in the same change.
