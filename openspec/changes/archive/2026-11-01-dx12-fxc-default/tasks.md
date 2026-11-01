# Tasks

## 1. Test seams

- [x] 1.1 Write `dx12_compiler_defaults_to_fxc` and `resolve_dx12_compiler_prefers_the_explicit_value` in `crates/pictura-render/src/gpu/select.rs` against a `resolve_dx12_compiler(Option<Dx12Compiler>)` helper, covering `Some(Dx12Compiler::Auto)`; use `matches!` because `Dx12Compiler` derives no `PartialEq`; verify they fail before the helper exists.
- [x] 1.2 Write `probe_detail_names_the_adapter_and_compiler` and `probe_detail_uses_the_request_error` against a pure `format_probe_detail` helper; verify they fail before the formatter exists.
- [x] 1.3 Write `selectGpuBackendWritesTheProbeDetail` and `probeDetailIsBounded` in `crates/pictura-app/cpp/tests/tst_gpu_backend.cpp`; verify they fail before the detail plumbing exists.

## 2. DX12 compiler policy

- [x] 2.1 Add `dx12_compiler()`, `resolve_dx12_compiler()`, and `dx12_compiler_label()` (`FXC`, `DXC`, `STATICDXC`, `AUTO` for `Auto`) to `crates/pictura-render/src/gpu/select.rs`, re-export them from `gpu/mod.rs` and `lib.rs`, and verify `cargo nextest run -p pictura-render --lib dx12_compiler`.
- [x] 2.2 Set `backend_options.dx12.shader_compiler = super::dx12_compiler()` in `request_device` (`crates/pictura-render/src/gpu/backend.rs:246`) and verify the existing `cargo nextest run -p pictura-render --lib` passes.
- [x] 2.3 Set `backend_options.dx12.shader_compiler = pictura_render::dx12_compiler()` in `create_device` (`crates/pictura-app/src/gpu.rs:70`) and verify `cargo check -p pictura_app`; confirm by inspection that both `InstanceDescriptor` sites call the helper.

## 3. Probe detail

- [x] 3.1 Add `ProbeReport { outcome, detail }`, make `probe_backend_outcome` return it, record `RequestAdapterError` / `RequestDeviceError` / software-adapter / storage-buffer rejections in a process-local slot, and record the validation mismatch in `gpu_composite_verified`; verify tests 1.2 and the updated existing probe assertions.
- [x] 3.2 Add the Rust cxx bridge struct `GpuProbeReply { code, detail }` to `crates/pictura-app/src/gpu_probe_bridge.rs`; have `gpu_probe_child` cap the detail at 512 characters before printing it, spawn the child with `Stdio::piped()` stdout, read the line only when the child exited on its own (never after a timeout kill, since a grandchild can hold the pipe), and synthesize the timeout detail; return `{ 1, "probe could not start: <io error>" }` from the spawn-error arm. Add `gpu_probe_captures_the_child_detail` and `gpu_probe_timeout_does_not_block` with an injected child command; verify `cargo test -p pictura_app --lib gpu_probe`.
- [x] 3.3 Change the C++ callback to return the C++ struct `GpuProbeResult { code, detail }`, write `entry.detail = detail.left(512)` in `selectGpuBackend`, and set `crashed during parent init` / `parent device init failed` in the marker and parent-init paths. Update all existing `selectGpuBackend` lambdas in `tst_gpu_backend.cpp` (195, 213, 227, 244, 266, 278, 296, 314, 389) and the `gpu_probe.h` callback comment; verify tests 1.3 and `ctest --test-dir build -R '^tst_gpu_backend$' --output-on-failure`.
- [ ] 3.4 Manually run the staged Windows binary `pictura.exe --gpu-probe dx12` and confirm one detail line on stdout naming the adapter, driver, and FXC.

## 4. WARP parity

- [x] 4.1 Rename the `software-vulkan` cargo feature to `software-gpu`, the `SOFTWARE_VULKAN` const to `SOFTWARE_GPU`, the `pictura-app` guard test to `app_never_enables_software_gpu`, `software_vulkan_accepts_a_cpu_device` to `software_gpu_accepts_a_cpu_device`, `gpu_is_available_under_software_vulkan` to `gpu_is_available_under_a_software_device`, and the `software_vulkan_*` tests and messages. Also rename the `ci.yml:131-132` `lint` clippy flag, `DEVELOPING.md:244,251`, `docs/dev/STATE.md:145,250`, and the `.config/nextest.toml` comment; verify `cargo nextest run -p pictura-render -p pictura_app`.
- [x] 4.2 Gate `vulkan_probe_is_clean_on_a_software_device` to non-Windows and strengthen `dx12_probe_is_clean_with_or_without_an_adapter` to assert `Ok` plus an FXC detail when the feature is on. That test is `#[cfg(target_os = "windows")]`, so local verification is the non-Windows gate plus the `--features software-gpu` build; its Windows assertion is verified by 6.3. Verify `cargo test -p pictura-render --lib --features software-gpu`.
- [x] 4.3 Add nextest to `windows-render` and a step running `cargo nextest run -p pictura-render --features software-gpu --profile gpu --run-ignored all`; keep the existing default-feature test step; raise `timeout-minutes` if the first run approaches the 45-minute limit; verify the YAML parses.
- [x] 4.4 Update the `.config/nextest.toml` comment and `docs/dev/testing-conventions.md` for the feature rename and the WARP use; verify both mention `software-gpu`.

## 5. Documentation

- [x] 5.1 Update `docs/01-architecture/gpu-rendering-pipeline.md` (FXC default, `WGPU_DX12_COMPILER`, probe detail, WARP CI), `docs/GLOSSARY.md` (`gpu-backend.json` detail, `dx12_compiler`), and `docs/dev/STATE.md`.
- [ ] 5.2 Verify `bash scripts/guard.sh` and commit the docs with `TASK-ALLOWS-DOCS`.

## 6. Integration and archive

- [x] 6.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo clippy -p pictura-render --all-targets --features software-gpu -- -D warnings`, and `cargo nextest run -p pictura-render -p pictura_app`; fix every failure.
- [x] 6.2 Run `python3 scripts/check-test-plan.py` and update any renamed test reference in `test-plan.md`; confirm every automated row names an existing test.
- [ ] 6.3 Run the Windows CI job and confirm the WARP parity step passes; record the run URL in the PR body.
- [ ] 6.4 Run `openspec validate --all --strict`, then `openspec archive dx12-fxc-default`, confirm the delta replaces the main spec's `software-vulkan` requirement, and confirm `scripts/check-openspec-archived.sh` passes.
- [ ] 6.5 After the archive, verify the rename has no stale `software-vulkan` references in `crates/ docs/ DEVELOPING.md .github .config` (the main spec is updated by the archive); fix any.
