# Tasks

## 1. Tests first

- [x] 1.1 Add `pictura-select` unit tests for `Selection::fit_centered` (identity, letterbox, upscale, odd centring, zero size).
- [x] 1.2 Add `tst_channels_panel` cases for the Duplicate Channel dialog, same/cross/New document duplicates, Invert, the Create New Channel drop target, cross-document drag, different-size fit, cap refusal, and source preservation.
- [x] 1.3 Add `tst_selection_save_load` cases for selection drag across same-size and different-size documents, source preservation, own-document refusal, and the drag-preparation seam.
- [x] 1.4 Write `test-plan.md` mapping every scenario to its automated test or a justified manual check.

## 2. Engine: fit coverage

- [x] 2.1 Add `Selection::fit_centered` with bilinear sampling, aspect-preserving fit, integer centring, and empty borders.
- [x] 2.2 Run the `pictura-select` tests.

## 3. Rust bridge

- [x] 3.1 Add `color_row_coverage` for colour and composite rows.
- [x] 3.2 Add `duplicate_channel_coverage` with fit, invert, name, cap refusal, and one `Duplicate Channel` state.
- [x] 3.3 Add `load_selection_coverage_fit` with fit and one `Load Selection` state.
- [x] 3.4 Add the Invert argument to `new_selection_document` and update callers.
- [x] 3.5 Update the cxx-qt bridge declarations and wrappers.

## 4. C++ shell

- [x] 4.1 Add `DuplicateChannelDialog` and register it in `cmake/sources/dialogs.cmake`.
- [x] 4.2 Add the `Duplicate Channel…` command to the Rust catalogue, regenerate `command_ids.h`, map the panel menu row, and bind the handler and enabled provider.
- [x] 4.3 Implement `PicturaMainWindow::openDuplicateChannelDialog` and the coverage transfer.
- [x] 4.4 Extend the channel drag payload with source, alpha index, mask, and name; accept it on the Create New Channel button.
- [x] 4.5 Route channel drags and selection drags in `FileDropRouter` for tabs and canvases.
- [x] 4.6 Start the selection drag from selection tools when the pointer reaches another document's tab.
- [x] 4.7 Register the new menu translation unit and headers in the CMake fragments.

## 5. Verification and close-out

- [x] 5.1 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 5.2 Build the app and run the affected Qt suites.
- [x] 5.3 `bash scripts/verify-fast.sh`.
- [x] 5.4 Update `docs/dev/doc-code-parity-gaps.md` and `ROADMAP.md` in a `TASK-ALLOWS-DOCS` commit.
- [x] 5.5 `openspec validate --all --strict`.
- [x] 5.6 Archive `channel-selection-drag` in the same change.
