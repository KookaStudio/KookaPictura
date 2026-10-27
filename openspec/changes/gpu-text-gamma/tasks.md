# Tasks

## 1. Change artifacts

- [x] 1.1 Write `test-plan.md` with one row per spec scenario and a coverage test or a justified manual check; run `scripts/check-test-plan.py` on the change

## 2. GPU text-gamma curve

- [x] 2.1 Expose the cached table through `TextCurve::samples()` and verify the existing `text_gamma` unit tests still pass
- [x] 2.2 Add the `text_on` params field, the read-only curve-table binding, the shader interpolation, and the per-dispatch flag for pixel and adjustment layers; verify with `cargo check -p pictura-render --features software-vulkan`
- [x] 2.3 Add the `gpu_parity` text scenes at γ 1.00, 1.45, 2.20, and off with opacity, Fill, and a mask, plus isolated/pass-through group scenes with a proxied type child and a proxied type region scene; verify it passes with `cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all` (lavapipe/CI)

## 3. Eligibility and metrics

- [x] 3.1 Remove the text-gamma eligibility decline and update the `GpuError::UnsupportedText` wording; verify proxy-less type still declines
- [x] 3.2 Rename the `unsupported_text` decline reason to `type_source` and add a metrics unit test asserting the old key is gone and `type_source` is present; verify `type_source_decline_is_counted` passes
- [x] 3.3 Update the eligibility tests for a proxied type layer with the gamma on and rename `gpu_defers_type_while_gamma_is_on` to `gpu_accepts_proxied_type_while_gamma_is_on`; verify the device-free eligibility tests pass

## 4. Documentation

- [x] 4.1 Update the stale text-gamma GPU sentence in `docs/dev/STATE.md`; verify the note names the proxied-type GPU path and the proxy-less CPU path

## 5. Integration verification

- [x] 5.1 Run `openspec validate --all --strict` and verify the change has no validation errors
- [x] 5.2 Run `cargo fmt --all`, `cargo clippy -p pictura-render --all-targets --features software-vulkan -- -D warnings`, and the `pictura-render` unit tests; verify all pass
- [x] 5.3 Run the opt-in text-gamma benchmark on a Vulkan device and record the old/new numbers in `benchmark.md`; verify the GPU path is faster and the curve overhead is small

## 6. Delivery

- [x] 6.1 Commit the `docs/dev/STATE.md` update separately with `TASK-ALLOWS-DOCS`
- [ ] 6.2 Archive the change with `openspec archive gpu-text-gamma`; verify only `archive/` remains under `openspec/changes/`
