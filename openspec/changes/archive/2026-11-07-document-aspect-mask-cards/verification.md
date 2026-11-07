# Verification

Mask cards now use document geometry independently of their owner content card.
Plain row-body clicks return to content after dedicated controls have handled
their clicks. Adjustment creation keeps the new layer selected with its mask
active. Explicit raster/vector thumbnail clicks retain their target.

- Build completed after all source edits. The final build-tree check passed.
- All 11 focused Layers/mask CTest entries passed.
- All 248 complete Qt CTest entries passed: 2,455 checks passed, two existing skips.
- Combined report: 8,033 passed, 124 skipped, zero failed. It includes 5,071
  workspace Rust checks and both app self-test invocations.
- Formatting and Clippy passed before the interrupted full gate. Repository
  guards, test-plan checks, strict OpenSpec validation, control protocol checks
  and both memory-harness checks passed.
- The UI spec was synced, compared against the delta, and archived. Unrelated
  requirements were preserved. All eight tasks and all 19 automated plan rows
  are complete; no manual reviewer checks remain.

The default full verification run was interrupted by the machine crash. Its
completed Rust JUnit report survived and was reused with the current build.
Both app self-tests and the complete Qt layer were then run separately, followed
by each remaining check sequentially. Remaining verification used one job/process
at a time with CTEST_PARALLEL_LEVEL=1 and RAYON_NUM_THREADS=1; no build overlapped
any test. Persistent local results are under build/verification-1199/.

No dependencies or golden baselines changed. Fill fixtures retain their fill
kind; adding a vector mask would convert them into shapes with a combined
content/vector thumbnail. Separate vector cards are tested on pixel, group,
adjustment and type rows; raster cards cover all seven owner kinds in portrait
and landscape documents.

These local results predate the rebase onto master at 36bb3323. The rebase
preserves #1200's corrected mask-link polarity: new masks use flags 0, retained
layer movement tests the UNLINKED bit, and imported fixtures explicitly keep
their unlinked state. Both roadmap entries and the upstream glyph-opacity
requirement were retained. At the user's request, no further local builds or
application tests run after this rebase; CI validates the final branch.
