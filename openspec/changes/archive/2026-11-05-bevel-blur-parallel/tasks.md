# Tasks

## 1. Regression checks and profiling harness

- [x] 1.1 Add the former serial convolution as a test-only filters reference and write `gaussian_blur_f32_matches_serial`; verify it compares `to_bits()` for the fields, radii, edges, and worker counts in `bb_field_bits` after the final build.
- [x] 1.2 Write `gaussian_blur_f32_handles_degenerate_fields`; verify the six shapes and short/long kernel cases in `bb_degenerate` after the final build.
- [x] 1.3 Retain the former radius-taking blur as a render-test-only reference and add the scoped thread-local selector described in the design, before the new call-site sigma conversion. Write `bevel_blur_composites_match_former_serial` using uncached asymmetric masked documents of at most 128 rows, all supported styles/techniques, and softening off/on; verify complete-buffer equality against that independent reference and execution of both paths after the final build, keeping the test file within its size cap.
- [x] 1.4 Write `bevel_blur_composites_match_across_worker_counts` for the same generated cases, retaining one-worker versus multi-worker complete-buffer comparison in addition to the former-blur comparison; verify the scoped selector resets and caches are cleared between runs after the final build.
- [x] 1.5 Add ignored integration test `bevel_cold_composite_profile` with external input supplied through `PICTURA_BENCH_PSD`; verify it separates read/decode from the first CPU composite, preserves the output, reports settings without the input identifier, and checks the 4-second acceptance limit when explicitly invoked.

## 2. Shared blur and bevel integration

- [x] 2.1 Add public `pictura_filters::kernel::gaussian_blur_f32` with `f64` sigma/weights/accumulation, ordered taps, clamped borders, per-pass narrowing, row parallelism, and zero-area handling; add API documentation beside the function and verify the two filters tests after the final build.
- [x] 2.2 Route all three bevel rounding/softening calls through the shared helper with the original radius mapping and delete the private production `blur_f32`; verify the generated render regression and existing bevel/layer-effect tests after the final build. Keep existing `blur_plane` callers and golden baselines unchanged.

## 3. Further optimization authorized after initial measurements

- [x] 3.1 Restructure the floating-point blur to sum rows one tap at a time using per-worker scratch storage and padded horizontal input; verify existing bitwise field and complete-composite references, with no changed golden output.
- [x] 3.2 Measure the updated cold composite and the remaining bevel stages; retain all timings and output comparisons in verification.md and remove temporary diagnostic instrumentation before completion.
- [x] 3.3 Re-run affected-crate tests, Clippy, formatting, and repository checks after the final code changes; verify the final acceptance measurements with fixed profile and worker count.
- [x] 3.4 Write bitwise distance-field comparisons against the former serial transform, extend the complete-composite serial selector, and parallelize independent distance and shading passes; verify direct field comparisons and the full-composite oracle after the final build.

- [x] 3.5 Implement runtime AVX2 selection with portable Gaussian row-sum fallbacks for the floating field and 8-bit kernels; verify `weighted_rows_match_scalar`, bitwise field/composite checks, and unchanged filter digests after the final build.

## 4. Integration verification and completion


- [x] 4.1 After all code and tests are complete, compile the affected tests once with at most 80% of available CPUs; fix compile errors and rebuild only when code changed. Run focused filters and render/layer-effect suites, including region-composite regressions, then appropriate broader checks; verify no golden delta or dependency addition.
- [x] 4.2 Run the profiling harness against baseline and changed code in fresh processes with the same optimized profile, worker count, and external input, three runs each; verify each changed run is under 4 seconds and report durations/settings without naming the input. If the target is missed, report the result without expanding scope.
- [x] 4.3 Run `python3 scripts/check-test-plan.py` and `openspec validate bevel-blur-parallel --strict`; verify every named test now exists and all six scenario rows are covered. Run the repository gates appropriate to the final change.
- [x] 4.4 Archive the completed change in the shipping PR and validate the archived specs; verify the zero-open guard passes and the PR contains no local input identifier. The test plan has no manual-only reviewer checklist rows.

