# Test Plan

One row per scenario in the spec deltas. Coverage names an automated test, or
`manual:` with the reason it cannot be automated.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `gb_dx12_fxc_default` | Call `resolve_dx12_compiler(None)` | The compiler is FXC | `pictura-render::dx12_compiler_defaults_to_fxc` |
| `gb_dx12_compiler_env` | Set `WGPU_DX12_COMPILER=staticdxc` and call `dx12_compiler()` | The compiler is `StaticDxc` | `pictura-render::dx12_compiler_reads_the_environment` |
| `gb_dx12_compiler_all_paths` | Inspect `gpu/backend.rs` and `pictura-app/src/gpu.rs` | Both set `backend_options.dx12.shader_compiler = dx12_compiler()` | manual: code inspection; the helper is the only policy source |
| `gb_probe_detail_ok` | Format a success detail from a fixture adapter and `Some("FXC")` | The detail contains the adapter name, driver, and FXC | `pictura-render::probe_detail_names_the_adapter_and_compiler` |
| `gb_probe_detail_failure` | Format a failure detail from a `RequestAdapterError` string | The detail contains the error text | `pictura-render::probe_detail_uses_the_request_error` |
| `gb_probe_detail_persisted` | Inject a probe returning `{0, "compiler=FXC; ..."}` into `selectGpuBackend`, save and reload the cache | The entry's `detail` matches the probe detail | `tst_gpu_backend::selectGpuBackendWritesTheProbeDetail` |
| `gb_probe_detail_bounded` | Inject a 2000-character detail into `selectGpuBackend` | The stored detail is 512 characters and the launch continues | `tst_gpu_backend::probeDetailIsBounded` |
| `gpu_windows_warp_parity` | `windows-render` runs the nextest `gpu` profile with `--features software-gpu --run-ignored all` | The parity suites report passed results on WARP | `pictura-render::gpu_matches_cpu_within_one_lsb`, `pictura-render::gpu_is_available_under_a_software_device` |
| `gpu_windows_fxc_probe` | Run the DX12 probe test on Windows with `--features software-gpu` | The outcome is `Ok` and the detail contains FXC. The WARP parity run proves the instance path; the probe detail proves the label. | `pictura-render::dx12_probe_is_clean_with_or_without_an_adapter` |
| `gpu_windows_missing_device_fails` | Run the Windows step on a runner without WARP | The job fails instead of reporting skipped tests | manual: CI negative case; needs a runner without WARP |
| Production rejects a software device | Run the default-feature `pictura-render` lib tests | `accelerates(DeviceType::Cpu)` is false | `pictura-render::a_software_adapter_is_no_gpu` |
| The test feature accepts a software device | Run the lib tests with `--features software-gpu` | `accelerates(DeviceType::Cpu)` is true | `pictura-render::software_gpu_accepts_a_cpu_device` |
| The application build leaves the feature off | Run the `pictura-app` tests with default features | `pictura_render::SOFTWARE_GPU` is false | `pictura-app::app_never_enables_software_gpu` |
| A default run reports GPU tests as skipped | Run `scripts/test-report.sh` | The GPU-dependent parity tests appear in the skipped count | manual: inspect the unified report's skipped count |
| A run without a device fails | Call `require_device(false)` | It panics with a message naming the missing GPU device | `pictura-render::require_gpu_panics_without_a_device` |
| Device-free GPU checks always run | Run the default test suite | Eligibility declines pass without a device | `pictura-render::plan_is_device_independent` |
| The job exercises the GPU | Run the Linux `gpu-parity` job | `gpu_available()` is true and the parity tests pass | `pictura-render::gpu_is_available_under_a_software_device` |
| A broken driver setup fails the job | Run `gpu-parity` without the software Vulkan driver | The job fails instead of passing on skipped tests | manual: CI negative case; needs a runner without the driver |
| `gpu_clippy_in_lint` | Read the `lint` job log | `cargo clippy -p pictura-render --features software-gpu` runs and passes | manual: CI job log |
