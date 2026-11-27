# Tasks

## 1. Tests before implementation

- [x] 1.1 Write Qt Test cases `slotsKeepSources`, `flipAndOffset`, `controlsAndSlots`, `previewMapping`, `nonAlignedMapping`, `healingTransformedSampling`, `previewEligibility`, `autoHide`, `brushClipping`, `rotatedBrushClipping`, `healingSourceCancelled`, `blendAndOpacity`, `sourceAlpha`, `grayAlignment`, `sourceStaysSnapshot`, and `asynchronousSourceInvalidation` in `tst_clone_source_overlay`, plus `pictura-app::source_cache_reuses_only_the_same_source_and_covered_region` and the associated cache color and cancellation tests, plus the transformed-healing Rust tests; register the suite and cover every test-plan row.

## 2. Overlay state and rendering

- [x] 2.1 Bind the controls and per-slot session settings; verify defaults, slot changes, and non-destructive edits using `controlsAndSlots`.
- [x] 2.2 Integrate Healing Brush with the shared slot and extend its offset-only sampling to apply the slot transform; connect both tools’ sampling maps to a transient canvas preview with bounded asynchronous source caching and stale-result cancellation. Verify `slotsKeepSources`, `flipAndOffset`, `previewMapping`, `nonAlignedMapping`, `healingTransformedSampling`, and `previewEligibility`.
- [x] 2.3 Implement stroke visibility and brush-footprint clipping; verify `autoHide` and `brushClipping`.
- [x] 2.4 Implement opacity, inversion, and the four blend modes; verify `blendAndOpacity`, `sourceAlpha`, and `grayAlignment`.

## 3. Integration

- [x] 3.1 Review and update ROADMAP.md and PARITY.md for changed feature coverage; inspect the resulting tracker entries.
- [ ] 3.2 After implementation, run the authorized build and planned Qt suite, then applicable project gates; archive the change after tracker review and validate OpenSpec when authorized.
