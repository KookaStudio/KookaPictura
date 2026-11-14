# Tasks

## 1. Regression coverage

- [x] 1.1 Extend `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists` so the disabled branch creates a document (e.g. `QVERIFY(window_->newDocument(...))`) before opening the Performance page, then asserts the checkbox is unchecked, the active view's `gpu_compute()` is false (exercising the `addDocument` sync), and `saveSessionNow()` records `gpuCompute` false; verify the suite builds and runs with `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_gpu_backend$' --output-on-failure` (expected to fail on a no-candidate machine until 2.1 lands)
- [x] 1.2 Update self-test check 70 so its default expectation is candidate-gated: include `gpu_probe.h`, compare `gpuView->gpu_compute()` with `avail || pictura::gpuChainHasResetPath()`, and keep the existing on/off/restore assertions; verify `./build/pictura --headless --self-test` prints `PASS` for check 70 on a machine with no candidate (expected to fail until 2.1 lands)

## 2. Normalize the preference

- [x] 2.1 In `crates/pictura-app/cpp/frame.cpp`, immediately after `gpuCandidatesAvailable_` is assigned, set `gpuCompute_ = false` when no candidate exists and add a comment naming the safe-mode exception; verify the extended `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists` passes on a machine with no candidate
- [x] 2.2 Add a `QSignalBlocker` to `PreferencesDialog::setGpuCompute` so the programmatic seed cannot emit `gpuComputeChanged` (consistent with `setTypeShowAsianOptions` and `setMemoryUsagePercent`); verify `tst_command_tree::preferencesPages` passes on a no-candidate machine (expected to fail before 2.2)
- [ ] 2.3 Confirm the gate reads `gpuCandidatesAvailable_` rather than `gpuAvailable_`; on Windows with a cached `crashed` backend (safe-mode reset path), verify Use GPU Compute stays enabled and checked and the status label stays `CPU (safe mode)`, and record the result in the PR body

## 3. Integration

- [ ] 3.1 Run `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, and `bash scripts/verify-fast.sh`; verify all pass with the change present
- [x] 3.2 Run the C++ self-test and the Qt GPU suite together (`QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^(tst_gpu_backend|tst_command_tree)$' --output-on-failure` and `./build/pictura --headless --self-test`); verify both pass on a no-candidate machine
