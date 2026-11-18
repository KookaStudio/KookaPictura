# Tasks

## 1. Write the planned checks before implementation

- [x] 1.1 Add core `source_state_legal_cases` and `source_state_rejects_invalid_combinations` in a dedicated state-test module; verify by source inspection that the design matrix and palette/sample contradictions are covered, without compiling yet.
- [x] 1.2 Add core `source_queries_include_descendant_stores` and `native_plane_authority_preserves_fallbacks`; verify the cases cover nested layer-only retention, later layer edits, converted modes, mismatched rect/depth, missing channels, and a matching control.
- [x] 1.3 Add core `composite_roles_and_output_policy_are_independent` and `state_replacement_refusal_is_atomic`; verify empty, absent, placeholder, non-RGB, RGBA, and mismatched buffers, independent output choices, and both rejected replacement paths are asserted.
- [x] 1.4 Add render `stored_composite_queries_agree_with_document`; verify source covers mode/depth forwarders and the shared stored predicate with present, absent, empty, and mismatched cases.
- [x] 1.5 Add codec `assigning_working_profile_preserves_converted_source`; verify it reads the existing Convert fixture, assigns a working profile, and checks both ICC records, unchanged pixels, and allocation sharing.

- [x] 1.6 Add app `save_absent_composite_preserves_merged_output_policy`: start with an absent composite, exercise the application picture-save preparation and writer, read back merged pixels, check output-disabled omission with a maintained buffer, and preserve the direct codec absent-buffer error.
- [x] 1.7 Add app `history_hollow_reattaches_shared_composite`: verify detached geometry, channels, output policy and tracked-plane slot order, then restore and assert unchanged shared allocations.

## 2. Encapsulate the core state

- [x] 2.1 Introduce `SourceState` and `CompositeState` with private internals, checked construction/replacement, borrowed views, and metadata-preserving sample edits; remove all eight public loose fields and document invariants beside the interface. Verify the public surface contains no mutable escape hatch for correlated metadata and uses existing Plane/Samples stores only.
- [x] 2.2 Centralize effective mode/depth, stored presence, shape, retention, retained-depth, and authoritative-plane queries in core; preserve dynamic layer scans and existing fallbacks. Verify their source against every planned predicate case and adapt the existing retained-depth test.
- [x] 2.3 Migrate core constructors and COW checks to the state interface without changing their expected bytes/identity; verify all six document-model plan rows retain their existing assertions and the dedicated test module stays within the file-size limit.

## 3. Migrate codec and render with their existing checks

- [x] 3.1 Migrate codec read/merged/normalize/lazy and profile paths to coordinated construction and replacements, including zero-filled placeholders and skipped merged decoding; preserve direct codec writer buffer preconditions and errors. Adapt existing ICC policy tests; verify by inspection that original/working profiles, palettes, native samples, and resource 1039 retain their current semantics.
- [x] 3.2 Migrate all PSD/PSB writers, streamed save, encode-cache keys, raster save consumers, examples, and codec test setup. Verify output mode/depth, compression and image-data omission still follow the same branches; preserve every oracle expectation and fixture byte.
- [x] 3.3 Migrate render mode/depth, geometry, native-sample edits, and composite patch/store/drop operations; forward existing public queries to core. Adapt the current render tests and verify no unchecked mutable store access or new dense materialization is introduced.

## 4. Migrate application and history consumers

- [x] 4.1 Route transform-backdrop and named-snapshot stored-composite checks through the shared predicate and inspect current filter_tools/picture readers for surviving equivalents. Verify the removed filter check is not reintroduced and no app consumer independently reconstructs the shared predicate.
- [x] 4.2 Migrate presentation, open/save/export, session/history plane extraction/restoration, display damage, recovery, memory accounting, and remaining workspace callers/tests. Implement the checked composite detach/reattach transition with preserved metadata, output policy, slot order and sharing. Verify with source searches that no legacy field access remains, and preserve COW/history assertions and row/tile access patterns.
- [x] 4.3 Address review regressions for packed Bitmap staging, streamed native-depth conversion, fixed-size composite writes, and metadata-preserving sample replacement; add the five checks named in the test plan. Execution remains deferred.

## 5. Compile and verify the integrated change

- [x] 5.1 After every code change above is written, run the CMake app/test build once under the documented CPU budget of at most 80%; verify success, fixing errors and rebuilding only after code changes.
- [ ] 5.2 Run all fifteen new tests, including the five review regressions, the existing core COW/retention checks, render composite and mode/depth checks, and app history checks through the documented runners; verify every automated plan row passes and `python3 scripts/check-test-plan.py` finds no missing scenarios or tests.
- [ ] 5.3 Run codec read/write and the depth, color-mode, ICC/profile, and tiled-plane oracle suites with their independent tools installed; verify PSD/PSB samples, resources, compression, and merged-image behavior with unchanged fixtures/goldens, and record actual results in the change.
- [ ] 5.4 Run the required formatting, clippy, unified tests, guards, and strict OpenSpec checks following testing-conventions.md; verify the final implementation diff is scoped and has no new dependencies or baseline changes.
- [x] 5.5 Archive this completed change and sync deltas before the shipping PR, then rerun strict OpenSpec validation and the archive guard; verify only archived changes remain and the PR states there are no manual test-plan rows.

Implementation tasks 1–4 are written. On 2026-11-17, the user authorized the
build: CMake configured successfully and the app/Qt test build passed under
`scripts/cpu-budget.py` with an 80% budget (12 of 16 allowed CPUs). The first
attempt found a stale PDF `source_planes` initializer; it was migrated to
`SourceState::replace_planes`, then the rebuild passed. The user subsequently
authorized archival, commits, and the PR with remaining checks delegated to CI.
All three spec deltas are synced and this change is archived. Strict OpenSpec
validation passed (452 items), the archive guard passed, and the test plan has no
manual rows. Tasks 5.2–5.4 remain pending: no tests or build commands were run
during archival, and the remaining verification gates are delegated to CI.
