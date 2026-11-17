# Tasks

Implementation source and test functions are written. On 2026-11-17, the
user-authorized CMake development build passed all 1,425 steps under the 80%
CPU budget (12 of 16 CPUs), compiling the app and Qt Test executables. Its four
Rust warnings were subsequently addressed in source. The user then requested
archival, commits, and a PR, with all further builds handled by CI. Rust test
targets have not been compiled, and test execution and the full gates remain
pending in CI. Completed implementation checkboxes do not claim those checks
have passed. Formatting, the active test-plan check, and strict OpenSpec
validation passed locally without compiling or running tests.

Write each group's missing tests before its production changes and retain all
existing assertions. Test-source tasks are verified by source inspection when
written; their execution is deferred until every code and test edit is finished.
Do not run a build, compiler, test-build, or Rust test command before that point.

## 1. Core coverage interface and projections

- [x] 1.1 Add std tests `coverage_samples_document_coordinates`, `coverage_rows_match_samples`, `coverage_rebase_shares_storage`, `coverage_invalid_geometry_is_refused`, `coverage_uniform_regions_preserve_tiles`, and `raw_mask_projection_preserves_samples` under core's coverage module. Verify their assertions cover `cov_coordinates`, `cov_rows`, `cov_rebase`, `cov_invalid`, `cov_uniform`, and the raw-mask part of `cov_sources`, including storage-sharing and zero-materialization checks.
- [x] 1.2 Add and export the concrete shared `Coverage` type with checked geometry, constant coverage, explicit exterior samples, point/row access, uniform/covered-bounds queries, and checked rebase. Document the coordinate and absence rules at the public interface. Verify the implementation uses tile/row access, shares source storage, and satisfies the six tests in 1.1 after the final build.
- [x] 1.3 Add `coverage_sources_preserve_selection_and_channel_bytes` in pictura-select before adding selection/channel projections. Verify its fixtures contain the specified fractional values, both storage forms, and size-refusal cases, and that the test names the public coverage interface rather than a synthetic mask.
- [x] 1.4 Expose selection and alpha-channel coverage without full-plane export or a reverse dependency from core to select. Preserve source dimensions, exact bytes, size checks, and tile storage; verify with 1.3 and the existing selection/tiled-mask tests after the final build.

## 2. Renderer, real masks, and clipboard

- [x] 2.1 Add `coverage_constraints_distinguish_absent_and_empty`, `renderer_edits_share_coverage_input`, `clipboard_reads_shared_coverage`, `coverage_preserves_real_mask_semantics`, `feathered_sparse_coverage_matches_flat`, and `feathered_selection_also_clips_paint_without_materialization` in appropriately sized renderer test modules. Verify source assertions cover `cov_none_empty`, `cov_render`, the engine part of `cov_clipboard`, `cov_mask`, and the renderer half of `cov_feather`; retain fixed pixel/refusal expectations in addition to flat-versus-tiled comparisons.
- [x] 2.2 Migrate filter/adjustment and Apply Image selection parameters and internal reads to coverage; migrate filter proxy selection reduction independently of layer-mask metadata. Verify all callers use the new interface and retain Liquify, transparency-lock, grayscale/native-target, and partial-coverage behavior via 2.1 and existing filter/Apply Image/proxy checks after the final build.
- [x] 2.3 Migrate matting, Layer via Copy/Cut, selected-content moves, selection-driven mask creation, and selection-gated smart-filter paths to coverage. Keep real `LayerMask` consumers metadata-aware, including `mask_refine::resolve_raster`; share only their raw sampling as appropriate. Verify 2.1 and existing matting, mask-refinement, canvas-mask-domain, and mask-creation assertions pass after the final build.
- [x] 2.4 Migrate clipboard selection interfaces, bounds, and row reads to coverage, retaining bounded `Clip` outputs and explicit persistent masks for Into/Outside. Verify `clipboard_reads_shared_coverage`, existing clipboard tests, and 2.3's mask checks after the final build; confirm empty and locked controls retain their existing refusals.

## 3. Paint, strokes, and healing

- [x] 3.1 Add `paint_edits_share_coverage_input`, `healing_reads_shared_coverage`, and `feathered_sparse_paint_matches_flat` in appropriately sized paint test modules before changing the corresponding implementations. Verify source assertions cover `cov_paint`, all Patch and Move/Extend modes in `cov_healing`, and the paint half of `cov_feather`, including offset/growing layers, rebased mask targets, and constant tile behavior.
- [x] 3.2 Migrate fill/gradient/bucket and eraser selection inputs and the fill `Through` seam to coverage; preserve tile-uniform fast paths through shared region queries. Verify 3.1 and existing fill, bucket, gradient, and eraser tests after the final build, with no dense adaptation of sparse input.
- [x] 3.3 Replace retained stroke `CoverageClip` selection geometry with the shared coverage value and migrate stencil/per-dab restoration reads. Verify 3.1 plus the existing brush, retouch, growing-layer, and mask-local selection-clipping assertions after the final build.
- [x] 3.4 Migrate Patch and Content-Aware Move/Extend selection interfaces and bounded solver input preparation to coverage. Preserve solver traversal/order and existing no-op/refusal rules; verify `healing_reads_shared_coverage`, existing `healing_reads_a_tiled_selection`, and existing healing/patch/move assertions after the final build.

## 4. App integration and synthetic-adapter removal

- [x] 4.1 Add std session tests `quick_mask_coverage_keeps_temporary_clip` and `clipboard_coverage_preserves_history` before app migration. Verify assertions cover `cov_quick_mask` and the history part of `cov_clipboard`, including implicit-full Quick Mask, both display interpretations, independent temporary clips, and exact undo/redo. Keep the existing selection-reader materialization guard and update only its interface calls.
- [x] 4.2 Migrate app filter/adjustment jobs, proxies/tools/depth inputs, matting, layer creation, selection move/refine, paint/healing, clipboard, Quick Mask, and alpha-channel producers. Retain owned coverage in jobs/strokes/previews as needed and use checked rebasing for mask-local edits. Verify every migrated consumer takes the common interface and 4.1 plus the existing session/selection-reader checks pass after the final build.
- [x] 4.3 Replace persistent-mask adapter uses at fill/adjustment defaults, Add Layer Mask, refinement outputs, and masked paste with explicit real-mask construction. Delete `selection_to_mask` and `selection_mask_for`, including obsolete test uses, without changing existing mask metadata policies. Verify a production-source search finds neither helper, and 2.1/4.1 plus existing mask/adjustment/history assertions pass after the final build.

## 5. CI validation and PR handoff

The user's finalization instruction supersedes the original requirement to
finish local validation before archival. Keep the remaining CI work unchecked
until its results are available; the earlier CMake success predates the final
warning cleanup and any integration with current master.

- [ ] 5.1 CI: compile the final app/workspace and Rust test targets for all migrated consumers. The earlier local CMake build passed within the 80% CPU budget; do not run further builds locally.
- [ ] 5.2 CI: run the 18 named new checks from `test-plan.md` plus existing filter, Apply Image, proxy, matting, mask-refinement/native-depth, clipboard, healing, stroke clipping, Quick Mask, and selection-reader regressions. Verify all planned assertions and zero-materialization guards pass without weakening expectations or changing golden baselines.
- [x] 5.3 Run `python3 scripts/check-test-plan.py` while `coverage-seam` is still active. Every scenario has its planned test and the active test-plan check passed before archival.
- [x] 5.4 Sync the capability spec and archive `coverage-seam` in the implementation PR as requested, with the CI validation tasks explicitly pending.
- [ ] 5.5 CI: run the full required repository gates, including formatting, clippy, test/report and guard checks, strict OpenSpec validation, and the zero-open check after archival. Verify there are no stale-build omissions, file-size violations, surviving synthetic adapters, or failing relevant checks; record any environment limitation accurately.
- [ ] 5.6 Collect the final CI validation evidence in the PR after its gates finish. The PR body must distinguish the earlier local CMake result and static checks from pending CI. The test plan has no manual rows to copy into a reviewer checklist.
