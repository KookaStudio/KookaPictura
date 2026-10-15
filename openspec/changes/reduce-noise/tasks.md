# Tasks: reduce-noise

## 1. Engine

- [x] 1.1 `ReduceNoiseParams` / `ChannelDetail`, `Filter::ReduceNoise`, `noise::reduce_noise`.
- [x] 1.2 Bilateral + chroma + sharpen + deblock passes; alpha preserved, deterministic.

## 2. App

- [x] 2.1 `reduce-noise` kind (arity 12) and defaults in `filter_map.rs`.
- [x] 2.2 `def` row and the dedicated `ReduceNoiseDialog` (Basic / Advanced, Channel menu).

## 3. Verification

- [x] 3.1 Property tests: monotonic Strength, Preserve Details, chroma variance, deblock, determinism, alpha, range rejection, tiny images; a kernel digest.
- [x] 3.2 `filter_map_tests` defaults and per-channel mapping; `tst_reduce_noise_dialog`; `tst_filter_menu` flips the stub check.
- [x] 3.3 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
