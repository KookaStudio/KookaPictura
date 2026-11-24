# Tasks

Implementation, focused acceptance, archive, and close-out checks completed on 2026-11-24. The final aggregate run has no branch-caused failures. Two failures remain and both reproduce on clean origin/master: `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)` and `check-milestone-names.py --self-test`.

## 1. Test plan and tests first

- [x] 1.1 Map every scenario in `specs/tools/selection-coverage-transfer/spec.md` to one `test-plan.md` row and an automated test reference or justified manual check.
- [x] 1.2 Add the transfer-core tests: `same_document_duplicate_matches_direct_apply`, `cross_document_transfer_matches_direct_apply`, `colour_row_load_applies_in_place`, `new_document_holds_source_coverage`, `transfer_records_one_destination_state`, `transfer_leaves_source_unchanged`, and `same_size_is_identity_and_different_size_fits`.
- [x] 1.3 Add the tile-sharing tests: `new_channel_shares_source_tiles`, `same_size_load_keeps_tiles`, and `fitted_transfer_is_tiled`.
- [x] 1.4 Add `tests_profiles/memory/selection_transfers.rs` and register it in `memory.rs`: `tiled_transfer_heap_holds_no_canvas` below one 4250×4250 plane + 1 MiB, `colour_row_transfer_heap_is_bounded` below one shared derived/retained plane + 12 MiB, `same_document_save_holds_no_canvas_plane`, and `capped_transfer_refuses_before_resampling` below 1 MiB. Keep both ignored dense red reproducers reproducible.
- [x] 1.5 Add the edge-case tests: `selected_areas_channel_duplicates_canonical_coverage`, `disabled_mask_load_and_save_semantics`, `read_only_destination_refuses_unharmed`, `transfer_refusals_leave_no_state`, and `invert_precedes_fit_and_combine`.
- [x] 1.6 Add `tst_selection_save_load::saveDialogCrossDocument`, `pictura-app::cross_document_save_combines`, and the ignored tiled and colour-row dense reproducers.

## 2. Transfer core

- [x] 2.1 Implement `transfer_source` for `""` (live selection), `alpha:<n>` through `selection_from_target`, `mask:<path>` through `selection_from_mask`, and `color:<row>` with the visible-channel mask.
- [x] 2.2 Implement the new-channel, save-target, load-selection, and new-document appliers with the existing history labels. Apply Invert before fit and combine, enforce same-size dialog transfers, and preserve channel-cap, deleted-target, read-only, and absent-source refusals.
- [x] 2.3 Verify tile sharing, the heap guards, and the full transfer test set with `cargo nextest run -p pictura_app 'cxxqt_object::impl_selection::transfer::tests::'` plus the memory-profile tests.

## 3. Bridge surface

- [x] 3.1 Add the in-view and ref-pair declarations to `impl_selection/bridge.rs` and their thunks to `impl_selection/bridge/imp.rs`. `cpp/api/selection_bridge.h` already includes the generated header; no new bridge file is needed.
- [x] 3.2 Extend the in-view colour-row load with Invert and add the in-view duplicate entry.

## 4. Adapter migration

- [x] 4.1 Save Selection branches before acquiring, routes the same document through the in-view save, and routes other documents and New through the owner. Add `tst_selection_save_load::saveDialogThroughFrame` and make `saveDialogNewImage` frame-driven.
- [x] 4.2 Load Selection routes same-document colour rows in-view with Invert and cross-document loads through the owner. Add `tst_selection_save_load::loadDialogThroughFrame`.
- [x] 4.3 Duplicate Channel routes the same document in-view, and other documents and New through the owner.
- [x] 4.4 Drops use the ref-pair owner. The Channels panel branches on `source == view_`, keeps the Load drop's same-document rule, retains a guarded source in channel MIME, and rejects closed sources. Add `tst_channels_panel::crossDocumentNewButtonDrop` and `closedChannelSourceDropsIgnored`.
- [x] 4.5 Retire `duplicate_channel_coverage`, `save_selection_coverage`, `load_selection_coverage_fit`, and `new_selection_document`; keep `load_selection_coverage` and rewrite the affected tests and production callers. Add `transfer_adapters_do_not_export_dense_coverage` to guard all three adapter sources against whole-coverage reader calls.

## 5. Integration and close-out

- [x] 5.1 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 5.2 Build and run the affected Qt suites offscreen (`'^tst_(selection_save_load|channels_panel|channel_options|mask_edit|calculations)$'`) and the memory-profile tests under nextest.
- [x] 5.3 Run `bash scripts/verify-fast.sh`.
- [x] 5.4 Confirm `scripts/check-file-size.sh` passes without a new allowlist entry and `impl_selection.rs` stays under its cap after the applier deletion.
- [x] 5.5 Run `openspec validate --all --strict`.
- [x] 5.6 Review `ROADMAP.md` and `PARITY.md` and record in the pull request that this refactor changes no feature coverage; if a tracker or docs gap is found, edit with the required `TASK-ALLOWS-DOCS` marker.
- [x] 5.7 Archive `selection-coverage-transfer` in the same pull request; verify `openspec list` shows no non-archived change.
