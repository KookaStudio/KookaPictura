## 1. Codec: confirm classification and exercise the whitelist

- [x] 1.1 Confirm `*b"mixr"` is already in `ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:49`); no classification change is needed, so do not edit `common.rs`.
- [x] 1.2 Add a `(*b"mixr", vec![0; 44])` case to `crates/pictura-codec/src/tests.rs::adjustment_layers_round_trip_key_and_bytes` so the whitelist is exercised by the read/write equality check.

## 2. Renderer: decode Channel Mixer

- [x] 2.1 Add `crates/pictura-render/src/channel_mixer.rs` (new module mirroring `color_balance.rs`, keeping `composite.rs` inside its file-size budget). Implement `pub(crate) fn decode_channel_mixer(d: &[u8]) -> Option<Adjustment>`: require `be_i16`/`be_u16` version `== 1`; read the `u16` monochrome flag; when clear read the `red`, `green`, `blue` channels, then always the `gray` channel; each channel is three big-endian `i16` source percentages, two skipped reserved bytes, and one big-endian `i16` constant. A payload shorter than the declared channels is `None`; every trailing byte is ignored. Reject (do not clamp) any source percentage or constant outside `-200..=200`, matching `pictura-adjust`'s `ChannelMixerParams` validation.
- [x] 2.2 Map the decoded channels: non-monochrome -> `monochrome: false`, `red`/`green`/`blue` the three RGB rows, `constant` their constants; monochrome -> `monochrome: true`, `red = gray.rgb`, `constant = [gray.constant, 0.0, 0.0]`, and the unused `green`/`blue` rows set to the identity defaults `[0,100,0]`/`[0,0,100]`. Add a short comment stating that the op's monochrome branch reads only `red`/`constant[0]`, so the other rows are unobservable.
- [x] 2.3 Wire `b"mixr" => crate::channel_mixer::decode_channel_mixer(&data.data)` into the `decode_adjustment` match (`crates/pictura-render/src/composite.rs`), add `mixr` (Channel Mixer) to the doc-comment committed-key list, and remove it from the deferred list in that comment.
- [x] 2.4 Confirm the existing `nvrt`/`invr`, `post`, `thrs`, `brit`, `levl`, `hue2`/`hue `, `expA`, `phfl`, `vibA`, `blwh`, `grdm`/`gdrm`, `blnc`, `SoCo`, `GdFl`, and `PtFl` arms are unchanged.

## 3. Renderer: encode Channel Mixer

- [x] 3.1 Add `pub fn encode_channel_mixer(monochrome: bool, red: [f64; 3], green: [f64; 3], blue: [f64; 3], constant: [f64; 3]) -> AdjustmentData` to `channel_mixer.rs`: clamp every source percentage and constant to `-200.0..=200.0`, round to `i16`, and write the version/flag and the channels so the block is 44 bytes in both forms (non-monochrome: `red`/`green`/`blue` then a `gray` row derived as the red row plus `constant[0]`; monochrome: the `gray` row plus 30 zero bytes). Each channel writes three source `i16`, two zero reserved bytes, and one constant `i16`, under key `mixr`.
- [x] 3.2 Re-export `encode_channel_mixer` from `crates/pictura-render/src/lib.rs` beside `encode_color_balance`.
- [x] 3.3 Add round-trip cases to `crates/pictura-render/src/tests/adjustment.rs::encode_decode_round_trips`: a non-monochrome call decodes back with the same rows/constants, a monochrome call decodes back with `red`/`constant[0]`, and out-of-range weights are clamped and still decode. Assert both forms' encoded length is 44 and the non-monochrome key is `mixr`.

## 4. Renderer: decode and composite tests

- [x] 4.1 Add a decoder unit test in `crates/pictura-render/src/tests/adjustment.rs` with a hand-built non-monochrome `mixr` payload (version 1, flag 0, four 10-byte channels) decoding to the expected `ChannelMixerParams`, including that the `gray` channel is ignored and trailing bytes are ignored; a hand-built monochrome payload (version 1, flag 1, one gray channel) decoding to `monochrome: true` with `red`/`constant[0]`.
- [x] 4.2 Reject cases: a `mixr` payload truncated before it declares its channels, `version != 1`, and a source percentage or constant outside `-200..=200` each return `None`, never a panic.
- [x] 4.3 Add a composite test that a `mixr` adjustment layer with a non-identity row (e.g. `red = [0, 100, 0]`) over a non-uniform backdrop differs from the backdrop-only composite (the "no longer a no-op" proof; the app default is identity, so build non-neutral params directly).
- [x] 4.4 Add `fixture_channel_mixer_decodes` reading `crates/pictura-codec/tests/fixtures/channel_mixer.psd`, finding the `Channel Mixer` and `Channel Mixer Mono` layers, and asserting `decode_adjustment` yields the authored non-monochrome and monochrome parameters.
- [x] 4.5 Update `deferred_keys_still_none`: drop `mixr` from the deferred key list (it is now committed); keep `curv`, `selc`, `clrL`, and version-3 `phfl`.

## 5. Oracle: fixture, ag-psd, and psd-tools partial

- [x] 5.1 In `scripts/generate-fixtures.py`, add `import struct` and a `_mixr_data(monochrome, red, green, blue, gray)` helper that hand-builds the raw bytes (`struct.pack(">HH", 1, 1 if monochrome else 0)` then per channel `struct.pack(">hhh", *rgb) + b"\x00\x00" + struct.pack(">h", constant)`, with `gray` written last for non-monochrome and alone for monochrome), and a `channel_mixer()` builder that creates a Base layer plus a non-monochrome `Channel Mixer` layer (`red=(30,-10,50)`, `green=(10,90,0)`, `blue=(0,20,110)`, constants `(5,-20,40)`, `gray=(100,0,0,0)`) and a monochrome `Channel Mixer Mono` layer (`gray=(20,40,60)`, constant `-15`, written in ag-psd's 44-byte shape). Register `"channel_mixer.psd": channel_mixer` in `FIXTURES` and regenerate.
- [x] 5.2 Confirm empirically (e.g. in `/tmp`) that psd-tools writes raw `bytes` passed as `TaggedBlock.data` verbatim for `Tag.CHANNEL_MIXER`, and record the result in the design note. The committed fixture's bytes must match the hand-built layout exactly.
- [x] 5.3 Add `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: run `node -e "<script>" <fixture>` where the script does `require('ag-psd').readPsd(bytes, { skipLayerImageData: true, skipCompositeImageData: true })` and prints one whitespace-separated line per `channel mixer` child (`0` followed by the 16 red/green/blue/gray numbers, or `1` followed by the 4 gray numbers). Parse the tokens in Rust (no JSON, no new dependency) and assert the authored `monochrome`/`red`/`green`/`blue`/`gray` values. Self-skip with a clear message when `node` or `ag-psd` is unavailable, mirroring `crates/pictura-render/tests/document/document_oracle.rs`. Document the install in the test header: `npm i ag-psd` at the repo root, or `NODE_PATH=<node_modules>`.
- [x] 5.4 In `.github/workflows/ci.yml`'s `oracles` job, install `ag-psd` (e.g. `npm i ag-psd` at the repo root) beside the existing ImageMagick/psd-tools steps so the oracle runs in CI.
- [x] 5.5 Add a psd-tools partial check to `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: feed the fixture's non-monochrome `mixr` bytes to `psd_tools.psd.adjustments.ChannelMixer.read` and assert `version == 1`, `monochrome == 0`, and the first five shorts equal the red row plus its constant. Self-skip when psd-tools is absent. Add a comment noting psd-tools reads only that prefix (the red row) and cannot see green/blue/gray, which is why ag-psd is the full-field oracle.

## 6. App wiring

- [x] 6.1 Add a `"channel-mixer"` arm to `adjustment_layer` in `crates/pictura-app/src/cxxqt_object/helpers.rs` returning `("Channel Mixer", encode_channel_mixer(false, [100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 100.0], [0.0; 3]))`, and import the encoder. The identity rows and zero constants are the neutral Photoshop default.
- [x] 6.2 Add the `Channel Mixer` / `adjustment:channel-mixer` `imp(...)` row to the `adjustmentsPanel` table in `crates/pictura-app/cpp/panels/panel_group_menu.cpp` after the Color Balance row.
- [x] 6.3 Leave `crates/pictura-app/cpp/command_tree.cpp` unchanged: the `Image > Adjustments > Channel Mixer` and `Layer > New Adjustment Layer > Channel Mixer` leaves already exist (`:215`, `:339`). Confirm only.
- [x] 6.4 Create `crates/pictura-app/cpp/selftest_layers_adjustments.{cpp,h}` exposing `int runLayersAdjustmentChecks(pictura::PicturaMainWindow& frame)`, and move the existing adjustment checks (`lpr_photo_filter` 283, `adjustments_photo_filter_menu` 284, `lpr_gradient_map` 285, `lpr_color_balance` 293) into it unchanged (pure move). `selftest_layers_controls.cpp` is 1185/1200 code LOC and cannot take another check.
- [x] 6.5 Register the new `.cpp`/`.h` in `CMakeLists.txt` (no globbing) and call `pictura::runLayersAdjustmentChecks(frame)` from `selftest_layers_controls.cpp` next to the other sub-runners, so the moved checks still run.
- [x] 6.6 Add the `lpr_channel_mixer` check (exit code **294**) in `selftest_layers_adjustments.cpp`: add a `channel-mixer` adjustment layer, assert it is added and reported as an adjustment, assert the neutral default leaves the sampled composite unchanged, and assert the Adjustments panel menu contains `Channel Mixer`. Names carry no milestone. Keep every touched file inside its size budget (`selftest_layers_controls.cpp` must shrink below 1200; the new files are small).

## 7. Gates

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 7.2 `bash scripts/verify-full.sh` and a headless self-test; record counts. Confirm the ag-psd oracle ran (install `ag-psd`) rather than self-skipped, and confirm the fixture is byte-stable across a `scripts/generate-fixtures.py` rerun.
- [x] 7.3 `openspec validate channel-mixer-adjustment-decode --strict` and `openspec validate --all --strict`.
- [x] 7.4 Commit. No `docs/` change is expected; any roadmap/`STATE.md` update stays a separate `TASK-ALLOWS-DOCS` commit.
