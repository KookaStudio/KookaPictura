# Tasks

## 1. Settings model (`pictura-core`)

- [x] 1.1 Add a `raw_settings` module defining `PicturaRawSettings` as per-tab sub-structs (Basic, mode/white balance, tone curve, detail, HSL + grayscale mix, split toning, lens, effects, calibration), with each control `Option`-typed, curve lists as `Vec<i32>`, enum codes as `[u8; 4]`, and text verbatim; reuse `CrsSettings` as `basic`. Verify with a unit test that two values differing only by unset-vs-default compare unequal, and that a multi-tab value is cloneable/equatable.
- [x] 1.2 Re-export the new type from `lib.rs` and keep `CrsSettings` exported unchanged. Verify `cargo test -p pictura-core` passes and `cargo clippy -p pictura-core --all-targets -- -D warnings` is clean.

## 2. Type-aware `Fltr` codec (`pictura-codec`)

- [x] 2.1 Split `src/pictura_raw.rs` into `src/pictura_raw/{mod.rs,codec.rs,tests.rs}` (pure move of the existing attach/merge logic and tests) and confirm `cargo nextest run -p pictura-codec pictura_raw` still passes.
- [x] 2.2 Implement the type-aware decode/encode key map in `codec.rs` for every key in the spec's table (int, double, curve list, text, bool, enum), emitting only set/empty-distinct controls; verify with a round-trip unit test over a value populated in every tab, including a variable-length `Crv ` list and an enum code.
- [x] 2.3 Add codec tests for: default settings encode to a control-free `Fltr`; decode tolerance of missing keys, wrong-typed numeric keys, and non-descriptor bytes; and exact four-byte names for the trailing-space keys (`Crv `, `LNR `, `CNR `, `STB `). Verify with `cargo nextest run -p pictura-codec`.

## 3. Synthetic fixture regeneration

- [x] 3.1 Extend `examples/make_smart_fixtures.rs` so the Camera Raw options carry every modeled key with values authored here (none copied from `reference-fixtures/`), then regenerate `assets/test_with_smart_object02.psd` via `cargo run -p pictura-codec --example make_smart_fixtures`; confirm the example runs and the file is written.
- [x] 3.2 Add a codec test that reads the regenerated fixture, decodes it, re-encodes, writes, and re-reads, asserting every modeled control round-trips and the eight preserve-only keys (`Dhze`, `Upri`, `GuUr`, `Rtch`, `REye`, `LCs `, `AuCA`, `TMMs`) survive byte-for-byte. Verify with `cargo nextest run -p pictura-codec`.

## 4. Call sites and default no-op

- [x] 4.1 Update `crates/pictura-adjust/src/pictura_raw.rs` to read `settings.basic`; verify its default byte-identical no-op test and the exposure/temperature/clarity/vibrance tests still pass with `cargo nextest run -p pictura-adjust`.
- [x] 4.2 Update remaining call sites (`pictura-render/src/smart_filter.rs`, `pictura-render/src/tests/smart_object.rs`, `pictura-render/src/document_ops/pictura_raw.rs`, `pictura-app/src/cxxqt_object/{impl_pictura_raw.rs,layers_smart_filters.rs}`, codec `smart_writer_tests.rs`, `tests/oracle/pictura_raw.rs`, `src/tests.rs`) to construct/read the nested type while keeping the 11-value bridge signatures; verify `cargo clippy --workspace --all-targets -- -D warnings` is clean and `cargo nextest run --workspace` passes.

## 5. Documentation correction

- [x] 5.1 Correct `docs/dev/camera-raw-cc-notes.md` §Fltr key map: the trailing-space key names, the added `GM_*` / `LNRD` / `LNRC` / `CNRD` / `CNRS` / `VigM` / `PC_1..3` keys, and the `List<Long>` value type of the point curves; keep the note that the target is earliest-CC/PV2012 and mark the value types as observed. Verify `bash scripts/guard.sh` passes with the commit carrying `TASK-ALLOWS-DOCS`.

## 6. Integration verification

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`, and `openspec validate --all --strict`; all must pass.
- [x] 6.2 Run `bash scripts/verify-fast.sh` (or `bash scripts/verify-full.sh` if a CMake build is needed) and confirm the `pictura` self-test still reports the same passed/failed/skipped summary.
