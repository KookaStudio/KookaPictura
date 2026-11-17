# Tasks

Tests in sections 1–5 must be written and checked off before production changes
in sections 6–10. Inspect the named assertions to complete test-writing tasks;
do not compile or build tests during these sections. Runtime verification is
deferred until all code is finished in section 11. Source delivery is recorded below. New tests use the existing Rust/Qt harnesses.
Build, execution, formatting, validation, evidence, and archive tasks in section
11 remain deferred at the user's request; no results are claimed.

## 1. Core model checks

Planned test location: `crates/pictura-core/src/layer_content/tests.rs`.

- [x] 1.1 Write `pictura-core::content_constructors_have_one_active_kind`; verify its assertions construct all seven kinds and assert compatible payload access, group children, and an ordinary pixel without an active adjustment.
- [x] 1.2 Write `pictura-core::content_types_have_no_execution_dependency`; verify its assertions check manifest dependency direction and construct/use public core content types without importing or depending on an execution crate, including in test-only dependencies. The separate downstream test in section 3 proves re-export identity.
- [x] 1.3 Write `pictura-core::fill_content_retains_each_parameter_set`; verify its assertions retain solid RGBA, all gradient fields/kinds/defaults, and pattern id/scale/link/origin exactly.
- [x] 1.4 Write `pictura-core::incompatible_content_transitions_leave_the_layer_unchanged`; verify its assertions refuse incompatible payload/transition requests without changing active content, attributes, or preservation.
- [x] 1.5 Write `pictura-core::proxies_and_masks_preserve_content_identity`; verify its assertions exercise proxy-backed Text/SmartObject, vector-masked Pixel, and raster-masked Fill without changing identity or plane sharing.

## 2. Codec and preservation checks

Planned test location: `crates/pictura-codec/tests/layer_content*.rs and parser unit tests`.

- [x] 2.1 Write `pictura-codec::adjustment_candidates_preserve_raw_bytes`; verify its assertions capture every recognized adjustment key and exact payload, including trailing unknown bytes, through read-only preservation and PSD read-save-read.
- [x] 2.2 Write `pictura-codec::adjustment_decoders_preserve_parameter_contract`; verify its assertions cover every destructive valid, malformed, range, version, trailing-byte, and ignored-field case retained in the committed-payload requirement using fixed expected parameters.
- [x] 2.3 Write `pictura-codec::conflicting_content_preserves_psd_and_psb_data`; verify its assertions round-trip PSD and PSB conflict/opaque candidates, unknown extra blocks, proxies, raw channel streams, and source-depth data without normalization churn.
- [x] 2.4 Write `pictura-codec::conflicting_import_candidates_follow_one_precedence`; verify its assertions normalize the conflicting candidate matrix to one active value, including groups and both decodable and opaque primary/vector content, while retaining losing bytes.
- [x] 2.5 Write `pictura-codec::content_writer_preserves_serialized_baselines`; verify its assertions compare established writer golden bytes and original preserved block/channel bytes without generating expectations with the migrated writer.
- [x] 2.6 Write `pictura-codec::fill_decoders_preserve_parameter_contract`; verify its assertions cover solid tuple/descriptor, five gradient kinds, pattern defaults, and all retained malformed fill cases with fixed expected parameters and None results.
- [x] 2.7 Write `pictura-codec::opaque_text_preserves_identity_and_bytes`; verify its assertions keep malformed TySh as opaque Text with its exact bytes and pixel fallback through PSD/PSB save and reopen.
- [x] 2.8 Write `pictura-codec::vector_fill_decoders_preserve_contract`; verify its assertions assert solid/gradient vscg decoding, content-over-key dispatch, version/malformed refusals, and primary-block precedence.
- [x] 2.9 Migrate existing `pictura-codec::codec_preserves_vector_fill_block` and `pictura-codec::existing_files_stay_byte_identical` to typed/read-only access as needed; verify original whole-document and exact golden-byte assertions remain.
- [x] 2.10 Write `pictura-codec::oversized_cube_points_are_bounded_during_import`; verify an in-memory cube with more than 64^3 numeric points, under sufficient budget for the supported maximum/input representation, stops before excess point allocation and retains the existing malformed-lookup no-op plus original bytes.
- [x] 2.11 Write `pictura-codec::hidden_lookup_tables_share_the_document_budget`; verify individually valid hidden LUTs fit each allocation, earlier raw/DOM/work claims fit, and their retained stores exhaust the shared explicit document budget with a typed resource error before the excess retained allocation; include a sufficient-budget success control.
- [x] 2.12 Write `pictura-codec::content_decode_temporaries_are_admitted_before_allocation`; verify newly introduced raw copies, descriptor DOM/text/list/raw buffers, and parser work buffers are checked against explicit per-allocation and remaining-total limits before allocation, including hidden content and small final parameters; assert resource refusal is not erased into None/opaque fallback.
- [x] 2.13 Write `pictura-codec::admitted_content_preserves_parameters_and_bytes`; verify admitted destructive/fill/LUT imports keep fixed expected parameters, complete rendered samples, and original saved bytes, including currently accepted later cube-size declarations and shared storage without duplicate allocation claims.

## 3. Destructive application checks

Planned test location: `crates/pictura-adjust/tests/content_contract.rs`.

- [x] 3.1 Write `pictura-adjust::every_destructive_variant_applies_to_valid_rgb_and_rgba`; verify its assertions apply every surviving Adjustment variant with valid parameters to RGB and RGBA, require success, assert complete expected color output and unchanged alpha.
- [x] 3.2 Write `pictura-adjust::invalid_adjustment_parameters_leave_buffers_unchanged`; verify its assertions assert typed errors and no partial mutation for all retained range and finite-value validation cases.
- [x] 3.3 Write `pictura-adjust::unsupported_buffers_are_refused_without_mutation`; verify its assertions reject unsupported channel counts and short coverage without changing samples.
- [x] 3.4 Retain `pictura-adjust::bad_buffers_error_not_panic` and `pictura-adjust::match_color_is_refused_at_native_depth`; verify malformed buffers still error and the native Match Color refusal leaves samples unchanged. Replace obsolete fill-refusal cases with the distinct-fill and all-destructive-success checks, without reducing the remaining validation coverage.
- [x] 3.5 Write `pictura-adjust::content_reexports_are_identical_to_core_types` in the downstream adjust integration suite; construct the re-exported content/parameter values and pass them through core-typed functions, and vice versa, so compilation proves type identity without any execution dependency in core or its tests.

## 4. Resolved-route and transition checks

Planned test location: `crates/pictura-render/src/tests/content_routes.rs, content_transitions.rs, and fill_content.rs`.

- [x] 4.1 Write `pictura-render::adjustment_content_routes_preserve_output`; verify its assertions cover supported destructive and opaque no-op primary adjustments, preserving backdrop, alpha, mask, and opacity behavior.
- [x] 4.2 Write `pictura-render::decoded_vector_fill_precedes_smart_and_type_sources`; verify its assertions assert decodable vscg wins over smart/text candidates only when the primary block is absent.
- [x] 4.3 Write `pictura-render::edited_fill_reopens_with_new_parameters`; verify its assertions edit preserved fill parameters, save/reopen, and assert the new active value/output plus unrelated original bytes.
- [x] 4.4 Write `pictura-render::fill_content_routes_preserve_output`; verify its assertions cover Solid/Gradient/Pattern routing and complete generated output, including primary-fill versus vscg conflicts.
- [x] 4.5 Write `pictura-render::fill_encoders_preserve_descriptor_contract`; verify its assertions assert every amended encoder round-trip, exact known descriptor layout/default omission, standard authored solid fill, and legacy tuple decode.
- [x] 4.6 Write `pictura-render::gpu_eligibility_consumes_the_resolved_content_route`; verify its assertions assert the existing eligibility/decline categories for every resolved route without requiring a GPU device.
- [x] 4.7 Write `pictura-render::gradient_fill_output_remains_unchanged`; verify its assertions compare full CPU samples for five geometries, reverse/scale, bounds, and mask no-op against fixed existing expectations.
- [x] 4.8 Write `pictura-render::group_content_routes_preserve_output`; verify its assertions cover pass-through and isolated groups with opacity and raster/vector masks against existing complete output.
- [x] 4.9 Write `pictura-render::pattern_fill_output_remains_unchanged`; verify its assertions assert tile adjacency/repetition, scale, nonzero linked/unlinked anchoring/origin, own alpha, missing-pattern placeholder, and mask no-op.
- [x] 4.10 Write `pictura-render::pixel_content_routes_preserve_output`; verify its assertions cover ordinary/masked/empty pixel paths and undecodable vector-fill pixel fallback against pre-migration route and full-buffer expectations.
- [x] 4.11 Write `pictura-render::proxy_routes_preserve_semantic_identity`; verify its assertions change proxy availability for Text and SmartObject and assert the render route changes while semantic identity does not.
- [x] 4.12 Write `pictura-render::rasterized_content_remains_pixel_after_reopen`; verify its assertions bake supported Fill/Shape/Text/SmartObject and conflicting imports, preserve command-specific masks/attributes, and verify no retired content reappears after save/reopen.
- [x] 4.13 Write `pictura-render::shape_content_routes_preserve_output`; verify its assertions cover authored and imported vscg/vsms shape appearances and the existing shape-recognition and saved-flag distinction.
- [x] 4.14 Write `pictura-render::smart_object_content_routes_preserve_output`; verify its assertions cover proxy/source choice, supported/unknown/disabled filters, master switch, stack modes, unresolved source, and existing text/proxy fallbacks.
- [x] 4.15 Write `pictura-render::text_content_routes_preserve_output`; verify its assertions cover parsed and opaque Text with and without proxies and failed raster generation using established output/fallback expectations.
- [x] 4.16 Write `pictura-render::undecodable_primary_content_blocks_fallthrough`; verify its assertions retain a primary no-op over a valid vscg and colored pixels, and verify raw preservation as well as complete CPU output.
- [x] 4.17 Write `pictura-render::vector_masked_pixels_do_not_become_shapes`; verify its assertions add/remove a vector mask on pixels and assert unchanged Pixel identity and refused shape-content commands.
- [x] 4.18 Write `pictura-render::vector_mask_changes_transition_between_fill_and_shape`; verify successful vector-mask add/delete/rasterize makes the existing Fill↔Shape transitions atomic, uses effective vmsk/vsms presence even when disabled/malformed, preserves refusal and saved-flag behavior, and survives save/reopen with the correct active geometry and command-specific mask result. Assert no extra vogk/vstk/style cleanup and keep Pixel-mask controls Pixel.

## 5. App and plan-parser checks

Planned test location: `crates/pictura-app/cpp/tests/tst_layer_content.cpp`.

- [x] 5.1 Write `tst_layer_content::rasterizeKeepsAppearanceAndHistory`; verify its assertions exercise every amended Rasterize scenario through commands, asserting appearance, retained attributes, one undo step on success, and no changes/history on refusal; preserve later Type/Smart Object availability.
- [x] 5.2 Write `tst_layer_content::rowsAndCommandsFollowContent`; verify its assertions assert all seven kinds project compatible row strings/badges/facts and content-edit eligibility, including proxies and masks.
- [x] 5.3 Write `tst_layer_content::typeRowsUseNormalizedIdentity`; verify its assertions assert both row-kind projections for valid/malformed/proxy-backed Text, ordinary/background pixels, and higher-priority conflicting content while retaining raw bytes.
- [x] 5.4 Write `tst_layer_content::undoRedoRestoresContentAndPreservation`; verify its assertions undo/redo fill edits and supported rasterization, then compare active identity, saved/reopened payloads, full image, and history counts.
- [x] 5.5 Extend `check-test-plan::self_test` with plain/outer-quoted scenario names containing inline code and ending in a backtick; verify assertions demand the exact original title and preserve existing parser cases.
- [x] 5.6 Write `tst_layer_content::vectorMaskTransitionsKeepRowsAndHistory`; verify fill/shape vector-mask add/delete/rasterize updates rows, forced-lock facts, eligibility, and existing history counts, then save/reopen and undo/redo each state while retaining the original saved flags and command-specific appearance; include ordinary vector-masked pixels as controls.

## 6. Shared data and destructive execution

- [x] 6.1 Move plain Adjustment/parameter data, needed Match Color statistics/parameters, and Replace Color sample/parameter records into core; add FillContent and compatibility re-exports. Verify there are no fill variants in Adjustment, no duplicated type definitions, and no core dependency on execution or PSD parsing, as covered by sections 1 and 3.
- [x] 6.2 Keep validation/errors and math in adjust, replacing error-coupled inherent validation with local validation functions where necessary. Verify the exhaustive apply match supports every surviving valid 8-bit variant and the section 3 assertions still cover input errors, alpha, and native refusals.

## 7. Codec normalization and model invariants

- [x] 7.1 Move pure destructive/fill/vector PSD decoders and necessary pure parsing helpers from render to codec without changing arithmetic, descriptor validation, defaulting, or unknown-data handling; expose destructive-only decode_adjustment and separate decode_fill. Verify section 2's fixed-parameter matrices retain all 33 committed-payload scenarios.
- [x] 7.2 Introduce private authoritative content/preservation storage, typed accessors, validated construction/replacement, and group-only child ownership. Verify sections 1–2 prevent independently active incompatible fields and retain shared proxy/native/channel storage without whole-plane copies.
- [x] 7.3 Add the Result-bearing budget-aware import decode path and thread the existing DecodeBudget through new raw-copy, DOM/string/list/raw-data, parser-work, and retained-parameter allocations before wiring eager normalization. Admit actual copies/capacity growth before allocation under the same per-allocation and cumulative document limits, including hidden content; do not refund temporary claims or double-charge an ownership move. Bound cube point accumulation at the supported global 64^3 maximum and retain final/later-header size validation. Verify section 2's excess-cube, temporary-admission, hidden-LUT-total, and sufficient-budget cases distinguish typed resource failure from existing malformed/no-op results; keep compatibility helper return types outside import.
- [x] 7.4 Replace codec's independently populated Layer options with private candidate assembly and one normalization policy after tree, channels, linked records, and text resolution. Use the budget-aware Result path rather than Option helpers that erase resource errors; verify conflict ordering, opaque Text/adjustment cases, effective vector-mask-key presence, and strict primary-versus-vscg behavior against sections 2 and 4.
- [x] 7.5 Adapt PSD/PSB writing and read-only raw access to preserved bytes and the active value; preserve tag/channel order, original descriptor payloads, flags, and existing encoding defaults. Verify section 2's PSD/PSB preservation and serialized-baseline checks.

## 8. Renderer routes and engine producers

- [x] 8.1 Replace private ambiguous Fill(Adjustment) routing with one public resolved-route function over active LayerContent, distinct destructive/fill routes, and existing smart-filter/stack/proxy/failure decisions. Verify all seven route tests and CPU/GPU-eligibility category assertions from section 4; do not retain a second production precedence chain.
- [x] 8.2 Migrate CompositePlan, CPU/native/GPU preparation and fallback, fill compositing, effects/mattes, source-cache identities, and damage/streaming readers to typed content or the shared resolved route. Verify unchanged complete CPU samples, existing GPU tolerances, once-per-plan decoding, cache invalidation on content edits, and existing lazy/tiled sharing regressions.
- [x] 8.3 Migrate workspace layer constructors, clone/history data, group nesting, mode/size transforms, merge/clipboard/filter/paint/selection paths, and all remaining content-kind checks to typed access. Verify no public writable active-option mirrors or duplicated precedence remain; raw serialization and attribute checks stay scoped to their data.

## 9. Authoring, rasterization, and history

- [x] 9.1 Stage supported content edits with their affected encoded representation and commit through typed content transitions; migrate fill/shape/text/smart-object authoring and conversion. Verify edited-fill save/reopen and unrelated-block preservation in section 4, with atomic refusal before mutation.
- [x] 9.2 Make successful vector-mask addition on decodable Fill install Shape and final effective-mask deletion/rasterization on Shape install Fill in the same staged transition. Preserve vmsk-over-vsms presence priority, malformed/disabled recognition, no-op refusals, saved flags, and existing mask/style/live-shape cleanup behavior; retire only active shape geometry and the mask definitions the command removes. Verify section 4's mask transition/save-reopen matrix and section 5's rows/eligibility/history case, including rasterized vector-times-raster coverage and unchanged Pixel controls.
- [x] 9.3 Migrate supported content rasterization paths to install Pixel content and remove obsolete primary/vector/type/smart definitions, including losing imported candidates that could reactivate on reopen. Verify section 4's reopen/appearance matrix and preserve command-specific masks, attributes, shape-effect reach, and existing Type/Smart Object support.
- [x] 9.4 Adapt history capture, undo/redo, and source sharing to restore active content and preservation together. Verify the section 5 undo/redo and vector-mask transition assertions and existing history copy-on-write/retention checks, without new full-plane copies.

## 10. App projection and mandatory plan tooling

- [x] 10.1 Derive bridge row kinds, adjustment/fill/shape/smart flags, badges, and content-command eligibility from authoritative content; retain stable C++ labels and command IDs for non-conflicting documents. Verify section 5's row, type, rasterize, and history cases, including malformed TySh and proxies.
- [x] 10.2 Register tst_layer_content in the Qt Test CMake list and use the existing offscreen fixture/report pattern; verify its five slots are discoverable and every plan reference names an actual test before runtime verification.
- [x] 10.3 Fix only the plan_rows cell-unquoting edge case in scripts/check-test-plan.py so balanced outer quoting is removed without stripping a scenario's inline-code backtick. Verify the extended self_test covers exact title matching and all prior cases; leave unrelated guard behavior unchanged.
- [x] 10.4 Complete the scoped workspace sweep for residual active-content field writes/precedence chains, migrate retained tests/examples to the public model, and update ROADMAP.md for #1344. Verify all test-plan names exist, file-size limits hold, and changes stay within the agreed model migration; docs/ remains unchanged.

## 11. Compile last and integration verification

The user requested archival and a shipping PR with all builds and verification
delegated to CI. Main specs are synced and this change is archived; the tasks
below remain unchecked because no local execution evidence has been collected.

- [ ] 11.1 Once every code/test edit is finished, configure and build the CMake app once using at most 80% of allowed CPUs and the repository's budget controls. Fix compilation errors and rebuild only after code changes; verify the application and registered Qt suite binaries are current.
- [ ] 11.2 Run python3 scripts/check-test-plan.py --self-test and the exact change's complete plan check, then the Rust/Qt checks via scripts/test-report.sh. Verify all new named tests run, existing render/composite/adjustment/codec/lazy-open/history suites remain green, the new hidden-content allocation limits refuse before excess allocation, and mask transitions retain their UI/file/history states, and report optional oracle/GPU skips honestly. Do not regenerate fixtures/goldens or weaken comparisons.
- [ ] 11.3 Run formatting, workspace clippy, file-size and other required guards plus openspec validate --all --strict; verify remaining integration checks pass and the touched-file review contains no independent kind classifier. Keep dependency/license checks unchanged because this change adds no dependencies.
- [ ] 11.4 Complete the test plan with actual CI evidence and no unexplained missing/manual rows, and verify the archived-only OpenSpec gate and final static guards. Main-spec sync and archival are complete at the user's request. The PR must state that all scenarios have automated coverage sources and report actual environment-limited oracle runs after CI executes them.
