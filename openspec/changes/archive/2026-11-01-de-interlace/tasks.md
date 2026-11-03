# Tasks

## 1. Spec and test plan

- [x] 1.1 Write the `imaging/video-filters` delta with the reconstruction and command requirements
- [x] 1.2 Write the `imaging/filter-app-ui` MODIFIED delta that moves the disabled example off De-Interlace
- [x] 1.3 Pair them with `test-plan.md`, one row per scenario
- [x] 1.4 Add the named tests (`pictura-filters`, `pictura-app`, `tst_de_interlace`) before the implementation they cover

## 2. Engine kernel in `pictura-filters`

- [x] 2.1 Add `InterlaceField` and `DeinterlaceMethod` to `src/video.rs`, with `de_interlace` over the planar buffer
- [x] 2.2 Expose the two enums from `lib.rs`
- [x] 2.3 Add the `Filter::DeInterlace` variant, its variant-table row and its dispatch arm
- [x] 2.4 Add the `DE_INTERLACE` catalogue row and its slot builder, plus the two choice arrays
- [x] 2.5 Add the variant to the exhaustive test tables (`tests/common/instances.rs`, `tests/kernel_digests.rs`, `tests/oracle/mapping.rs`) and to `catalogue/tests.rs`
- [x] 2.6 Unit-test both fields, both methods, the edges, alpha, refusal and the single-row no-op

## 3. App wiring

- [x] 3.1 Add the `de-interlace` mapping and descriptor tests
- [x] 3.2 Add `tst_de_interlace` (dialog defaults, commit and history label, disabled without a target) and register it in the tests `CMakeLists.txt`
- [x] 3.3 Move `tst_filter_menu::stubEntriesStayDisabled` to `Filter > Extract…` and add De-Interlace to `dialogCollectsParameterSlots`
- [x] 3.4 Note De-Interlace in the whole-layer preview doc comment

## 4. Command catalogue

- [x] 4.1 Flip `filter.video.de.interlace` to implemented
- [x] 4.2 Update the golden catalogue row, the unwired list and the implemented count

## 5. Cleanup and verification

- [x] 5.1 Tick De-Interlace in `ROADMAP.md` and update `public_docs/filter-video-other-digimarc.md`
- [x] 5.2 Update `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md`
- [x] 5.3 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`
- [x] 5.4 Build once, then run the affected Rust tests, `ctest -R '^(tst_de_interlace|tst_filter_menu)$'`, and `verify-fast.sh`
- [x] 5.5 Archive the change with `openspec archive de-interlace`
