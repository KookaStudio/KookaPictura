# Tasks

Finalization requested: no further local builds. Tests, feature audits, lint and
integrated gates are delegated to CI. Spec sync and archive are complete; unchecked
verification tasks remain pending. CI now runs both normal-profile feature audits
and renderer tests alone before the workspace test run. Checked source tasks record written
implementation, not executed verification.

## 1. Write the planned checks before production edits

- [x] 1.1 Review `test-plan.md` against all 68 delta scenarios and the three additional acceptance/checker rows; verify every row has runnable coverage and preserve the four issue-mandated comparison bodies and expected bytes.
- [x] 1.2 Add `thin_images_keep_only_level_zero` to renderer tests, checking 4096x2 and 2x4096 against the independent oracle; extend `level_count_matches_the_view_pyramid` with per-level sizes for its existing cases. Verify the assertions cover both stopping conditions without sharing level math; run them only after all code is finished.
- [x] 1.3 Add `a_tiled_level0_crops_without_materializing` and preserve the rebuild assertions in `a_tiled_level0_builds_the_same_levels`; verify contiguous/tiled bytes agree and the tiled source has no materialized mirror, before removing the mixed coarse/update checks.
- [x] 1.4 Add `view_oracle_is_test_only` in a small app build-configuration test module if needed for file size. Assert module and re-export gating, dev-only feature activation, the retained minimal methods, and the absence of imports/calls from oracle math into production region/level helpers; verify this guard fails when the module is ungated or a production helper is substituted.
- [x] 1.5 Add the standard-library-only `scripts/check-view-oracle.py` with `check_profile` and `self_check`. Consume an existing Cargo JSON build report, require an unambiguous renderer artifact with the oracle feature off, and run positive shared-export and negative oracle-import rustc probes against that exact artifact. Verify self-check fixtures reject malformed/missing/feature-enabled reports and unrelated compiler failures. Do not start a Cargo build from the checker.
- [x] 1.6 Add `panel_reads_stay_bounded` beside the app picture/display tests to assert current Info sample bytes and bounded region/tile work without a full composite. Adapt `histogram_source_level_is_bounded_and_size_independent` to check actual coarsest level dimensions, including 4096×2 level 0; do not assert a universal source-size or source-work bound.
- [x] 1.7 Add `previewStrokeUsesTilesAndRestoresZoom`, `frameBoundedPaintMatchesCpu`, and `panelsUseTileCrops` to `tst_present_cache.cpp`. Cover exact CPU replay and commit/cancel history, preview/zoom level restoration, queued-dab flush counts, optional GPU comparison with zoom-selected levels and exact current tiles after GPU damage, and actual Navigator/Histogram tile sources and bounded Histogram binning, including the thin level-0 source. Verify each slot asserts the corresponding THEN clauses; keep any new test hooks narrow and the file within its cap.
- [x] 1.8 Extend `cachedLevelCropMatchesTheUncachedDraw` to assert reduced-crop reuse, and `largeDocumentPresentsThroughALevel` to cover high zoom past the former 64-megapixel scaled-present bound and a sparse 16000-class live-stroke fixture. Extend `mid_stroke_tiles_match_a_full_recomposite_of_the_working_document` to include reduced levels and multiple queued dabs; verify exact bytes against the independent reference.
- [x] 1.9 Extend `no_stored_level_falls_back_to_exact` for an unavailable requested level and `preview_leaves_the_composite_unchanged` for an exact level-0-only preview; extend `solo_mask_and_live_stroke_fall_back`, `over_budget_backdrop_is_refused`, `unreducible_content_falls_back_to_full`, and `empty_rect_is_a_noop` to cover the explicitly listed gaps in `test-plan.md`; verify exact fallback output and unchanged damage where required. These retain existing test names rather than adding misleading new coverage claims.

## 2. Isolate and shrink the oracle

- [x] 2.1 Move the shared `PyramidLevel` container and `SMALLEST_SIDE`/`TILE` constants unchanged to `pyramid_level.rs`; update the production imports and preserve their root exports. Replace the oracle's direct private-dimension reads with the existing getters. Verify the container diff is a pure move, field visibility stays private and the module contains no shared reduction or level-count math.
- [x] 2.2 Declare the renderer's non-default `test-oracle` feature; gate the entire `view_pyramid` module and its `Planes`/`ViewPyramid` exports with `cfg(any(test, feature = "test-oracle"))`. Enable the feature only through the app renderer dev-dependency. Verify the manifest/source boundary with 1.4 and keep ordinary dependency declarations free of oracle activation.
- [x] 2.3 Remove `coarse`, `coarse_level`, `fits`, `update`, and `patch_level`, their dead sampling/damage helpers, coarse budget constant, and method-specific tests. Verify only `rebuild`, `level_count`, `level_size`, and `crop` remain on the oracle, while retained dimension, odd-edge, premultiplication, crop and tiled-source assertions survive.
- [x] 2.4 Update renderer module/API comments to describe production logical levels and the test-only reference without unconditional rustdoc links to gated exports. Verify repository-wide oracle references are production constants/container imports, test callers, gated definitions or explicit explanatory text; do not change the four required numerical comparison bodies.
- [x] 2.5 Reconcile the scoped ROADMAP entry with the tile-pyramid contract and issue #1349, preserving issue links on remaining unchecked items. Verify only the relevant roadmap wording changes; no `docs/`, archived proposal, dependency or golden baseline edits are needed.

## 3. Compile once the implementation is finished, then verify

Normal release and dev-app library builds passed with the 80% CPU budget
(12 jobs). Cargo JSON reports are saved at `/tmp/pictura-1349-build/release.jsonl`
and `/tmp/pictura-1349-build/dev-app.jsonl`; the feature audits remain pending.
The full CMake application/Qt-target build and renderer/application Rust test
compilation (`--no-run`, dev-app profile) also passed. No test binaries or
verification checks were executed.

- [x] 3.1 After every implementation and test source is written, run one normal library build pass per required profile: `cargo build -p pictura_app --lib --release --message-format=json` and `cargo build -p pictura_app --lib --profile dev-app --message-format=json`. Use the repository CPU budget (at most 80 percent), preserve each report in a temporary path, and rebuild only to address changed code. Verify both builds succeed and neither requests test targets or the oracle feature.
- [ ] 3.2 Run `check-view-oracle.py` on both reports and its `--self-check`; verify shared exports compile, oracle imports fail with the expected unresolved-import diagnostics, and neither renderer artifact enables `test-oracle`. Retain the two audit results as PR evidence.
- [ ] 3.3 Run renderer tests alone with no oracle feature, then the app comparisons for `rc_levels`, `tp_crop`, `tp_damage`, `tp_solo_restore` and retained oracle tests. Verify unchanged byte comparisons pass and both contiguous and tiled sources remain covered; run the ignored pan/zoom and solo-restore profiles only as explicit profiling checks if their evidence is needed.
- [ ] 3.4 Build the CMake Qt targets after source work is complete and run `tst_present_cache`, `tst_mask_edit`, `tst_large_document`, and `tst_edit_refresh` offscreen using the repository test conventions. Verify every GUI row in the plan is covered, including all added slots and explicit GPU-only skips when no adapter is available; add no legacy self-test check.
- [ ] 3.5 Run formatting, applicable clippy checks, `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, file-size checks and applicable repository guards that do not require an archived change. Verify the test-plan guard now resolves every planned test and no full-image or stored-level dependency has been introduced.

## 4. Finalize the implementation change

- [x] 4.1 Archive `retire-stored-view-pyramid` in the implementation PR to merge the deltas, using its `retire_capabilities: true` metadata to remove the emptied `compositing/view-pyramid` spec. Verify the five old stored-level requirements and the six obsolete observation requirements are gone, replacement requirements preserve or explicitly migrate their scenarios, all active references describe logical levels or tile sources, and `openspec validate --all --strict` passes on the resulting main specs.
- [ ] 4.2 After archive, run the required integrated verification gates including the zero-open-change guard, and prepare the PR for #1349 with the scenario/test evidence and both normal-profile feature audits. Verify the PR reports no manual-only plan rows, no new dependencies, no golden changes and the intentional ordinary-build API removal; keep all planning tasks unchecked until their implementation work is complete.
