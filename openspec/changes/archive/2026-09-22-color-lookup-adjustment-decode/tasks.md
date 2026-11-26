## 1. Engine model and LUT kernel (`pictura-adjust`)

- [x] 1.1 Add `ColorLookupKind { ThreeDLut, AbstractProfile, DeviceLinkProfile }`, `Lut3d { size: usize, points: Vec<[f32; 3]> }`, and `ColorLookupParams { kind, lookup: Option<Lut3d> }` to `crates/pictura-adjust/src/types.rs`; add `Adjustment::ColorLookup(ColorLookupParams)` to the enum.
- [x] 1.2 Add `crates/pictura-adjust/src/lut.rs` with `parse_cube(bytes: &[u8]) -> Option<Lut3d>` (skip comments/blank lines, require `LUT_3D_SIZE` in `2..=64`, require exactly `size³` finite RGB triples, red index fastest) and `sample(lut, rgb) -> [f32; 3]` trilinear.
- [x] 1.3 Add the `color_lookup` kernel to `crates/pictura-adjust/src/color.rs` (or `lut.rs`) and dispatch `Adjustment::ColorLookup` in `src/apply.rs`: `lookup: None` is a no-op; invalid `Lut3d` (size `<2`/`>64`, wrong count, non-finite) is `AdjustError::InvalidParams`.
- [x] 1.4 Re-export `ColorLookupParams`/`ColorLookupKind`/`Lut3d` from `crates/pictura-adjust/src/lib.rs`.
- [x] 1.5 Unit tests: identity cube within 1 LSB; exact corner node; red-fastest node at size 3 red 1 green 0 blue 0; trilinear mid-cube value; `None` no-op; invalid params; alpha preserved.

## 2. Decode and encode (`pictura-render`)

- [x] 2.1 Add `crates/pictura-render/src/color_lookup.rs` with `decode_color_lookup(d: &[u8]) -> Option<Adjustment>`: require `be_u16(d,0)? == 1`, call `pictura_codec::read_descriptor(&d[2..])`, read `lookupType`, `LUTFormat`, `LUT3DFileData` (from `DescValue::Raw` `tdta`), and `parse_cube`; decode failure is `None`.
- [x] 2.2 Add `encode_color_lookup(file_data: &[u8], name: &str) -> AdjustmentData` and `identity_cube() -> Vec<u8>` in the same module, building `[u16 = 1]` + `write_descriptor` of the `lookupType`/`LUTFormat`/`dataOrder`/`tableOrder`/`Dthr`/`Nm  `/`LUT3DFileName`/`LUT3DFileData` object.
- [x] 2.3 Add the `b"clrL" => crate::color_lookup::decode_color_lookup(&data.data)` arm in `crates/pictura-render/src/composite.rs` and update the doc comment; re-export `encode_color_lookup`/`identity_cube` from `src/lib.rs`.
- [x] 2.4 Update the malformed/deferred tests: move `clrL` out of `deferred_keys_still_none` in `src/tests/adjustment/part_decode.rs`, add a malformed-`clrL` no-op case, and assert a valid identity cube renders a non-no-op-free composite.
- [x] 2.5 Ensure the GPU path declines a `ColorLookup` layer (falls back to CPU) as it does for other unsupported adjustments; confirm the adjustment GPU parity test.

## 3. Codec round-trip (`pictura-codec`)

- [x] 3.1 Add a `clrL` `(key, payload)` case to `adjustment_layers_round_trip_key_and_bytes` in `crates/pictura-codec/src/tests.rs`, proving the block re-emits byte-for-byte.

## 4. Fixture and oracles

- [x] 4.1 Add a `color_lookup()` builder to `scripts/generate-fixtures.py` (a `ColorLookup(version=1, data_version=16, …)` or raw `TaggedBlock` embedding a small `LUT_3D_SIZE 2` identity `.cube`), register it in `FIXTURES`, and commit `crates/pictura-codec/tests/fixtures/color_lookup.psd`.
- [x] 4.2 Add a psd-tools oracle test in `crates/pictura-render/tests/oracle/adjustment_oracle.rs` (`fixture_color_lookup_decodes`) asserting the decoded `lookupType`, `LUTFormat`, `DThr`, and the `LUT3DFileData` bytes.
- [x] 4.3 Add an ag-psd oracle test in `crates/pictura-codec/tests/psd/agpsd_oracle.rs` (`ag_psd_reads_color_lookup_fixture`) asserting `lookupType`/`lutFormat`/`dataOrder`/`tableOrder`/`dither`/`lut3DFileData`; self-skip without node/ag-psd.
- [x] 4.4 Update `crates/pictura-codec/tests/fixtures/README.md` with the new fixture row.

## 5. App wiring and self-test

- [x] 5.1 Map kind `color-lookup` in `crates/pictura-app/src/cxxqt_object/helpers.rs` to a `clrL` layer carrying `identity_cube()` and the name `Color Lookup`.
- [x] 5.2 Add the `Color Lookup` / `adjustment:color-lookup` row to `crates/pictura-app/cpp/panels/panel_group_menu.cpp` and the `Image > Adjustments` / `Layer > New Adjustment Layer` entries to `crates/pictura-app/cpp/command_tree.cpp`.
- [x] 5.3 Add self-test `lpr_color_lookup` (code 455) in `crates/pictura-app/cpp/selftest_layers_adjustments.cpp`: adds the kind, asserts it is an adjustment layer, asserts the default composite is neutral, and asserts the panel menu contains `Color Lookup`.

## 6. Gates and commit

- [x] 6.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-adjust -p pictura-render -p pictura-codec`.
- [x] 6.2 `bash scripts/verify-full.sh` (CMake build + self-test + fast gates) and `openspec validate --all --strict`.
- [x] 6.3 Confirm `scripts/check-file-size.sh` and `scripts/guard.sh` are green; no file over its cap.
- [x] 6.4 Archive the change (`openspec archive color-lookup-adjustment-decode -y`) and commit the implementation, spec, fixture, and tests.
- [x] 6.5 Update `docs/dev/psd-support-roadmap.md` and `docs/dev/STATE.md` in a separate `docs:` commit carrying the `TASK-ALLOWS-DOCS` marker.
