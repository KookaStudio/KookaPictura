# Tasks

## 1. Validation gate

- [x] Add `validation_document()`: an 8×8 opaque gradient backdrop plus a
  half-transparent gradient `Multiply` source.
- [x] Add `gpu_composite_verified()` (`OnceLock<bool>`): composite the scene
  through the ungated kernel and diff against `composite_rgba` within ±1 LSB.
- [x] Split `composite_gpu_region` into a gated entry and
  `composite_gpu_region_raw`, so validation does not recurse.
- [x] `gpu_available()` requires `gpu_composite_verified()`.
- [x] `composite_gpu_region` returns `GpuError::Unavailable` after the
  CPU-only-mode check when the device fails validation.
- [x] `shared_device()` returns `None` when the device fails validation.

## 2. Checks

- [x] Unit test: the validation scene carries non-255 alpha and non-uniform
  pixels.
- [x] Unit test: `gpu_available()` and `composite_gpu` agree on the validation
  scene (available ⇒ `Ok`; unavailable ⇒ `Unavailable`).
- [x] `gpu_parity` and `gpu_filter_parity` suites pass (skip on a failing
  device).
- [x] `./build/pictura --headless --self-test` passes `fresh_white`.

## 3. Verification

- [x] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`.
- [x] `cargo nextest run --workspace` and `bash scripts/verify-fast.sh`.
- [x] `openspec validate gpu-compositor-oracle-validation --strict` and
  `openspec validate --all --strict`.
