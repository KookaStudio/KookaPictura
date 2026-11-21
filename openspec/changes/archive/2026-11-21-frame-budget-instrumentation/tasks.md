# Tasks

## 1. Stage the scenario tests

- [x] 1.1 Add `frameCoalescesViewportInput`, `frameCadenceIsPerCanvas`, and `frameRecordsLateTilesSeparately` to `tst_control_metrics`. Cover queued paints, per-canvas state, and a controlled tile completion.
- [x] 1.2 Add `frame_gpu_samples_follow_drawn_tiles` and `frame_gpu_samples_ignore_unpainted_results` to `pictura-app` tests. Cover attribution, deduplication, absent device durations, and independent CPU slack.
- [x] 1.3 Add `frameCapturePreservesOutput` to `tst_control_metrics_off` and `disabled_frame_metrics_skip_capture` to `pictura-app` tests. Compare equivalent draws and saves across startup modes and cover the disabled capture path.
- [x] 1.4 Add `frameScreenshotDoesNotConsumePending` and `frameGpuPartialDamage` to `tst_control_metrics`, covering both screenshot grab scopes and disjoint transformed paint regions. Add `frame_gpu_standin_attribution` to `pictura-app` tests for accepted, overwritten, and undrawn reduced stand-ins.
- [x] 1.5 Add `frameResetClearsPendingTiming` to `tst_control_metrics` and `frame_history_is_bounded_and_resettable` to `pictura-app` tests.

## 2. Capture and report canvas timing

- [x] 2.1 Add opt-in timing to the Qt viewport input and paint paths, and send each completed document paint record through the view bridge. Suppress capture around both explicit control screenshot grab scopes without consuming pending state or changing cadence. Coverage is in `tst_control_metrics`.
- [x] 2.2 Add the frame ring and capture reset to the existing app metrics snapshot. Coverage includes the response fields, capacity, capture order, and reset in the frame history and control metrics cases.
- [x] 2.3 Carry the first-to-last blend-pass device interval from the existing wgpu query result into successful display region repairs and reduced-level stand-ins. Attribute it through accepted tile content versions, including intentionally stale stand-ins, and actual supplied source spans intersected with the transformed effective paint region and content clip. Preserve the existing edit reporting. Coverage includes the GPU attribution tests and the hardware check in `test-plan.md`.
- [x] 2.4 Document the `metrics` capture and reset procedure and the five-second, 24 MP pan, zoom, and Rotate View measurements in `DEVELOPING.md`. Include how to compute interval p95, spot an evicted capture, interpret unavailable display presentation timing, and distinguish the blend-pass interval from conversion and readback.

## 3. Verify and prepare to ship

- [x] 3.1 Check coverage references with `python3 scripts/check-test-plan.py`, build under `scripts/with-build-lock.sh`, and run the targeted Rust and Qt suites. Verify metrics-off outputs and the existing metrics response scenarios. Run the reference gesture capture and hardware device timestamp checks in `test-plan.md`.
- [x] 3.2 Update the roadmap entry when the feature ships. Record the manual checks in the PR reviewer checklist, archive the completed change, and validate the OpenSpec tree with `openspec validate --all --strict`.
