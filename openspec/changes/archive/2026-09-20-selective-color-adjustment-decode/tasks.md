## 1. Codec: confirm classification and whitelist

- [x] 1.1 Confirm `*b"selc"` is already in `ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:49`); no `common.rs` change is needed.
- [x] 1.2 Add a `selc` case (`(*b"selc", vec![0u8; 84])`) to the adjustment round-trip whitelist `cases` in `crates/pictura-codec/src/tests.rs:432` so read/write equality is exercised for the key.

## 2. Adjust: Selective Color op and kernel

- [x] 2.1 In `crates/pictura-adjust/src/types.rs`, add `SelectiveColorMethod { Relative, Absolute }`, `SelectiveRange { c, m, y, k: i16 }` (deriving `Copy`, `Default`, `PartialEq`, `Eq`), and `SelectiveColorParams { method, ranges: [SelectiveRange; 9] }` with a `Default` (relative, all-zero ranges). Add the `Adjustment::SelectiveColor(SelectiveColorParams)` variant.
- [x] 2.2 Re-export `SelectiveColorMethod`, `SelectiveColorParams`, and `SelectiveRange` from `crates/pictura-adjust/src/lib.rs`.
- [x] 2.3 In `crates/pictura-adjust/src/color.rs`, implement `selective_color(p, buf, n)` following design D3: validate every correction in `-100..=100` (`AdjustError::InvalidParams` otherwise), return `Ok(())` unchanged when all nine ranges are zero (mark it a `ponytail:` ceiling naming libpsd's unconditional lossy round-trip), then per pixel run the integer CMYK conversion, the integer hue, the six hue-family windows (`r0 = -105 + i*60`, core `[r1, r2)`, feather ramps), and the whites/neutrals/blacks selection, using truncating `i32` division exactly as design D3 records. Add `rgb_to_intcmyk`, `intcmyk_to_rgb`, and `rgb_to_int_hue` helpers.
- [x] 2.4 Add the `Adjustment::SelectiveColor(p) => selective_color(p, buf, n)` arm in `crates/pictura-adjust/src/apply.rs`.
- [x] 2.5 Add kernel known-value tests to `crates/pictura-adjust/src/tests.rs`: all-zero ranges is bit-exact identity; relative reds `m = 50` on `(200, 100, 50)` yields `(200, 61, 51)`; absolute reds `y = 100` on `(200, 100, 50)` yields `(200, 101, 1)`; relative blacks `k = -100` on `(0, 0, 0)` yields `(255, 255, 255)`; absolute whites `c = 100` on `(255, 255, 255)` yields `(1, 255, 255)`; a correction outside `-100..=100` returns `InvalidParams`.
- [x] 2.6 Add `Adjustment::SelectiveColor` to the alpha-preservation variant list in `crates/pictura-adjust/src/tests.rs` (the list grows from 19 to 20).
- [x] 2.7 Add a `SelectiveColor` row to the `MAPPING` table and the `NO_EQUIVALENT` list in `crates/pictura-adjust/tests/oracle.rs` (tolerance 0, note "no faithful IM operator; profile-free integer CMYK round-trip"), taking the table from 18 to 19 rows and `NO_EQUIVALENT` from 15 to 16. Confirm the `MAPPING.len()`/count assertions still hold.

## 3. Renderer: decode Selective Color

- [x] 3.1 Add `crates/pictura-render/src/selective_color.rs` (new module mirroring `curves.rs`/`color_balance.rs`). Implement `pub(crate) fn decode_selective_color(d: &[u8]) -> Option<Adjustment>`: require at least 84 bytes, version at offset 0 equal to 1, map the method at offset 2 (`0` → Relative, nonzero → Absolute), skip the reserved plate 0 (offsets 4..12) without validating it, read the nine range plates (offsets 12..84) as four big-endian `i16` each, reject any correction outside `-100..=100`, and ignore every trailing byte. Add a `ponytail:` comment naming the reserved plate and the lossy conversion. Put the decode/reject unit tests in this module's `#[cfg(test)] mod tests` (not `tests/adjustment.rs`, which is at 1398/1400 LOC).
- [x] 3.2 Wire `b"selc" => crate::selective_color::decode_selective_color(&data.data)` into the `decode_adjustment` match (`crates/pictura-render/src/composite.rs:338`), add `selc` (Selective Color) to the doc-comment committed-key list, and remove it from the deferred list in that comment.
- [x] 3.3 Confirm the existing `nvrt`/`invr`, `post`, `thrs`, `brit`, `levl`, `hue2`/`hue `, `expA`, `phfl`, `vibA`, `blwh`, `grdm`/`gdrm`, `blnc`, `mixr`, `curv`, `SoCo`, `GdFl`, and `PtFl` arms are unchanged.
- [x] 3.4 Add decode tests in `selective_color.rs::tests`: a relative payload decodes to the expected nine ranges and method; an absolute payload decodes to `Absolute`; a non-zero reserved plate and trailing bytes are ignored; `version != 1`, a short payload, and a correction outside `-100..=100` each return `None` without panicking.

## 4. Renderer: encode Selective Color

- [x] 4.1 Add `pub fn encode_selective_color(method: SelectiveColorMethod, ranges: &[SelectiveRange; 9]) -> AdjustmentData` to `selective_color.rs`: write the `u16` version 1, the `u16` method (`0` relative, `1` absolute), an 8-byte zero reserved plate, then the nine range plates in reds…blacks order as four big-endian `i16` each, under key `selc`. Clamp every correction to `-100..=100`. Decode on the output must equal the input method and ranges.
- [x] 4.2 Re-export `encode_selective_color` from `crates/pictura-render/src/lib.rs` beside `encode_curves`.
- [x] 4.3 Add an encoder round-trip test in `selective_color.rs::tests`: the neutral default is 84 bytes with method 0 and an all-zero reserved plate; a non-neutral call decodes back with the same method and nine ranges; out-of-range input is clamped.

## 5. Renderer: fixture decode and deferred set

- [x] 5.1 Update `deferred_keys_still_none` (`crates/pictura-render/src/tests/adjustment.rs:882`): drop `selc` from the deferred key list (it is now committed); keep `clrL` and version-3 `phfl`.
- [x] 5.2 Add `fixture_selective_color_decodes` to `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: read `crates/pictura-codec/tests/fixtures/selective_color.psd`, find the `Selective Color` and `Selective Color Abs` layers, and assert `decode_adjustment` yields the authored relative and absolute params.
- [x] 5.3 Add a psd-tools partial/field check to `adjustment_oracle.rs` feeding the fixture's relative `selc` block to `psd_tools.psd.adjustments.SelectiveColor.read` and asserting `version == 1`, `method == 0`, and `data[1..]` equals the nine authored plates. Self-skip without psd-tools; comment that psd-tools names no plates, so ag-psd is the full-field oracle.

## 6. Oracle: fixture and ag-psd

- [x] 6.1 In `scripts/generate-fixtures.py`, import `SelectiveColor` from `psd_tools.psd.adjustments`, add a `selective_color()` builder with a Base layer plus the relative and absolute `selc` layers from design D7 (plate 0 zero; reds through blacks authored), register `"selective_color.psd": selective_color` in `FIXTURES`, and regenerate.
- [x] 6.2 Confirm empirically that psd-tools writes the `SelectiveColor` object verbatim (10 plates, 84 bytes) and that reopening the fixture yields `version=1` and the authored plates; record that regeneration is byte-stable and no existing golden changes.
- [x] 6.3 Add an ag-psd oracle test to `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: run `node -e "<script>" <fixture>` where the script reads with `require('ag-psd').readPsd(bytes, { skipLayerImageData: true, skipCompositeImageData: true })` and prints one whitespace-separated line per `selective color` adjustment (mode plus the nine named ranges). Parse the tokens in Rust (no JSON, no new dependency) and assert the authored relative and absolute values. Self-skip with a clear message when `node` or `ag-psd` is unavailable. Update the file header to describe the `selc` oracle alongside `mixr`/`curv`.
- [x] 6.4 Confirm `.github/workflows/ci.yml`'s `oracles` job already installs `ag-psd`; note the result.

## 7. App: kind, panel entry, self-test check

- [x] 7.1 In `crates/pictura-app/src/cxxqt_object/helpers.rs`, add a `"selective-color"` arm to `adjustment_layer` using `encode_selective_color(SelectiveColorMethod::Relative, &[SelectiveRange::default(); 9])` and the layer name `Selective Color`; import the new types.
- [x] 7.2 In `crates/pictura-app/cpp/panels/panel_group_menu.cpp`, add `imp(QStringLiteral("Selective Color"), QStringLiteral("adjustment:selective-color"))` to the `adjustmentsPanel` block after `Channel Mixer`.
- [x] 7.3 Add the `lpr_selective_color` check to `crates/pictura-app/cpp/selftest_layers_adjustments.cpp` taking exit code **296** (295 is taken by `present_cache_edge` in `selftest_canvas.cpp`): create a 4x4 RGB document, sample before, add `selective-color`, assert the last layer's kind is `adjustment`, the composite is unchanged (neutral default), and the Adjustments panel menu contains `Selective Color`. Register no new file (the sub-runner already exists) and keep the file inside its size budget.

## 8. Gates

- [x] 8.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 8.2 `bash scripts/verify-full.sh` and a headless self-test; record counts. Confirm the ag-psd oracle ran (install `ag-psd`) rather than self-skipped, and confirm the fixture is byte-stable across a `scripts/generate-fixtures.py` rerun.
- [x] 8.3 `openspec validate selective-color-adjustment-decode --strict` and `openspec validate --all --strict`.
- [x] 8.4 Commit code + openspec + fixture. No `docs/` change is expected; any roadmap/`STATE.md` update stays a separate `TASK-ALLOWS-DOCS` commit.
