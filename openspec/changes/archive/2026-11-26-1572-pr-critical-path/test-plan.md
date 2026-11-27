# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| C++-only change | Classify a JSON list containing only `crates/pictura-app/cpp/panel.cpp`. | `cpp=true`, `rust=false`, `render=false`, `docs=false`. | `scripts/classify-ci-paths.py::self_check` |
| `Rust change runs everything` | Classify a list containing `crates/pictura-mcp/src/lib.rs`. | `rust=true` and `cpp=true`. | `scripts/classify-ci-paths.py::self_check` |
| `Mixed change runs everything` | Classify C++ and renderer paths together. | `rust`, `cpp`, and `render` are true. | `scripts/classify-ci-paths.py::self_check` |
| `Unrecognized path runs everything` | Classify an unfamiliar root path. | `rust` and `cpp` are true. | `scripts/classify-ci-paths.py::self_check` |
| `Workflow edit re-runs the suite` | Classify `.github/workflows/ci.yml`. | `rust`, `cpp`, and `render` are true. | `scripts/classify-ci-paths.py::self_check` |
| A skipped job still reports | Open a C++-only pull request and inspect the skipped job checks. | Skipped jobs report success instead of no check run. | manual: GitHub Actions job-level skip reporting requires a live workflow run. |
| `ci_fetch_retry` | Force the first paths-filter attempt to fail in a live workflow run. | The second attempt supplies the file list, the classifier job succeeds, and no third fetch occurs. | manual: failure injection for a hosted action requires a live workflow run. |
| `ci_fetch_fail_closed` | Pass a failed filter status and malformed JSON to the classifier self-check, then inspect the workflow conditions. | Run-all outputs are written before the non-zero classifier exit. | `scripts/classify-ci-paths.py::self_check` |
| `ci_missing_outputs` | Inspect each gated job condition and evaluate it with an empty `needs.changes.outputs` value. | Every gated job runs because empty is not exact `false`. | manual: GitHub Actions expression evaluation requires a live workflow run. |
| `ci_separate_jobs` | Inspect the `lint` and `test` job dependencies. | Neither job depends on the other. | manual: workflow topology is not executable in a unit test. |
| `ci_render_nextest` | Inspect the ordered `test` steps. | The dedicated renderer nextest run precedes workspace nextest. | manual: workflow command order requires source inspection. |
| `ci_release_packaging` | Inspect the packaging workflow pull-request conditions. | Feature pull requests skip packaging dry runs. | manual: Release PR identity is supplied by GitHub at runtime. |
| `gpu_precise_render_gate` | Classify MCP-only and script-only lists. | `render=false` for both. | `scripts/classify-ci-paths.py::self_check` |
| `gpu_renderer_change` | Classify `crates/pictura-render/src/lib.rs`. | `render=true`. | `scripts/classify-ci-paths.py::self_check` |
| `gpu_workflow_change` | Classify `.github/workflows/ci.yml`. | `render=true`. | `scripts/classify-ci-paths.py::self_check` |
| `gpu_classifier_failure` | Inspect the render job conditions after a failed classifier publishes run-all outputs. | Both render jobs run despite the failed classifier. | manual: cross-job execution after failure requires a live workflow run. |
| `gpu_windows_warp_device` | Run the existing Windows DX12 smoke on a WARP runner. | The device is created and the test passes. | `pictura_render::gpu::select::dx12_device_comes_up_with_fxc` |
| `gpu_windows_fxc_probe` | Run the existing smoke with `WGPU_DX12_COMPILER` unset. | The compiler label is FXC. | `pictura_render::gpu::select::dx12_device_comes_up_with_fxc` |
| `gpu_windows_missing_device_fails` | Run the smoke on a Windows runner without a usable adapter. | The device assertion fails. | `pictura_render::gpu::select::dx12_device_comes_up_with_fxc` |
| `gpu_windows_no_cpu_tests` | Inspect all Windows `run` steps. | Only Clippy and the DX12 smoke execute Cargo commands. | manual: workflow command inventory requires source inspection. |
| `vo_corrosion_reuse` | Have both build stages emit a Cargo completion record, then inspect the app build capture. | Only the app build record reaches the audit. | `scripts/test_report_execution.py::app_build_log_excludes_qt_build_tail` |
| `vo_dev_app_audit` | Run the checker self-check against a synthetic `dev-app` artifact and diagnostics. | Feature, profile, shared-export, and oracle-import checks remain enforced. | `scripts/check-view-oracle.py::self_check` |
| `vo_report_fails_closed` | Feed mixed valid logs, malformed JSON-looking lines, missing artifacts, and failed builds to the checker self-check. | Valid Cargo records are accepted and every invalid report is rejected. | `scripts/check-view-oracle.py::self_check` |
