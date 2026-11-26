## 1. Codec: classify `blnc`

- [x] 1.1 Add `*b"blnc"` to `ADJUSTMENT_KEYS` in `crates/pictura-codec/src/common.rs` (20 → 21 entries) so a real Color Balance block becomes `Layer.adjustment` instead of an opaque extra block.
- [x] 1.2 Add a `(*b"blnc", vec![0; 20])` case to `crates/pictura-codec/src/tests.rs::adjustment_layers_round_trip_key_and_bytes` so the whitelist is exercised by the read/write equality check.

## 2. Renderer: decode Color Balance

- [x] 2.1 Add `decode_color_balance(d: &[u8]) -> Option<Adjustment>` in `crates/pictura-render/src/composite.rs`: read nine big-endian `i16` with `be_i16` at offsets 0..18, read the `u8` luminosity at offset 18, require at least 19 bytes, reject any shift outside `-100..=100`, and return `Adjustment::ColorBalance(ColorBalanceParams { shadows, midtones, highlights, preserve_luminosity })` with `preserve_luminosity = luminosity != 0`. Ignore every trailing byte. Mark the collapsed tri-state luminosity model with a `ponytail:` note.
- [x] 2.2 Wire `b"blnc" => decode_color_balance(&data.data)` into the `decode_adjustment` match and add `blnc` (Color Balance) to the doc-comment committed-key list.
- [x] 2.3 Confirm the existing `nvrt`/`invr`, `post`, `thrs`, `brit`, `levl`, `hue2`/`hue `, `expA`, `phfl`, `vibA`, `blwh`, `grdm`/`gdrm`, `SoCo`, and `GdFl` arms are unchanged.

## 3. Renderer: encode Color Balance

- [x] 3.1 Add `pub fn encode_color_balance(shadows: [f64; 3], midtones: [f64; 3], highlights: [f64; 3], preserve_luminosity: bool) -> AdjustmentData` that clamps each shift to `-100.0..=100.0`, rounds to `i16`, writes the nine big-endian values, the luminosity byte, and one pad byte (20 total) under key `blnc`.
- [x] 3.2 Re-export `encode_color_balance` from `crates/pictura-render/src/lib.rs` alongside the other encoders.
- [x] 3.3 Add a round-trip case to `crates/pictura-render/src/tests/adjustment.rs::encode_decode_round_trips`: a non-neutral shift decodes back with the same bands and flag, and an out-of-range shift is clamped and still decodes.

## 4. Renderer: decode and composite tests

- [x] 4.1 Add a decoder unit test in `crates/pictura-render/src/tests/adjustment.rs`: a hand-built 20-byte `blnc` payload decodes to the expected `ColorBalanceParams`; a truncated payload and a shift above 100 or below -100 each return `None`.
- [x] 4.2 Add a composite test that a Color Balance adjustment layer with a non-zero midtone shift over a non-uniform backdrop differs from the backdrop-only composite (the "no longer a no-op" proof; the app default is neutral, so the test builds non-neutral params directly).
- [x] 4.3 Assert `decode_adjustment_subset_and_unknown` still passes; `blnc` is no longer in any deferred set.

## 5. Renderer: psd-tools parity

- [x] 5.1 Add `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: call `encode_color_balance` with known bands, get the 20 raw bytes, and run a `python3` script that does `from psd_tools.psd.adjustments import ColorBalance; ColorBalance.read(BytesIO(data))`, printing `shadows`, `midtones`, `highlights`, and `luminosity`; assert they equal the encoder inputs. Self-skip with a message when `python3` or `psd_tools` is absent, mirroring `crates/pictura-render/tests/document/document_oracle.rs`.

## 6. App wiring

- [x] 6.1 Add a `"color-balance"` arm to `adjustment_layer` in `crates/pictura-app/src/cxxqt_object/helpers.rs` returning `("Color Balance", encode_color_balance([0.0; 3], [0.0; 3], [0.0; 3], true))`, and import the encoder. The all-zero shifts are the neutral Photoshop default.
- [x] 6.2 Add the `Color Balance` / `adjustment:color-balance` `imp(...)` row to the `adjustmentsPanel` table in `crates/pictura-app/cpp/panels/panel_group_menu.cpp` after Gradient Map.
- [x] 6.3 Add one C++ self-test check (exit code **293**) in `crates/pictura-app/cpp/selftest_layers_controls.cpp`: add a `color-balance` adjustment layer, assert it is added and reported as an adjustment, and assert the Adjustments panel menu contains `Color Balance`. Do not assert a pixel change, because the authored layer is the neutral default. Keep the file inside its `scripts/file-size-allowlist.txt` ceiling.

## 7. Gates

- [ ] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [ ] 7.2 `bash scripts/verify-full.sh` and a headless self-test; record counts.
- [ ] 7.3 `openspec validate color-balance-adjustment-decode --strict` and `openspec validate --all --strict`.
- [ ] 7.4 Commit. No golden fixture changes and no `docs/` change is expected.
