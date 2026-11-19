# Tasks: native-depth-tone-tools

## 1. Tests

- [x] 1.1 Add `pictura-paint` tests for 16-bpc Dodge, Burn, Sponge, native precision, Grayscale, and cancel restore.
- [x] 1.2 Add a `pictura-paint` test that a retained 32-bit source store is refused.
- [x] 1.3 Add `tst_retouch_tools::sixteenBitDodgeLightsTheCanvas`.

## 2. Engine

- [x] 2.1 Add `tone_native::NativeToneBrush` reading and writing the layer's retained `u16` colour planes, mirroring the 8-bit result, and applying selection coverage at native depth.
- [x] 2.2 Route `Stroke::begin_tone` through `fill_native::native_plan`; refuse 32-bit working and retained depths; keep the 8-bit path for `Copy`.
- [x] 2.3 Add `DabEngine::ToneNative` dispatch and growth in `stroke`/`stroke/grow`.
- [x] 2.4 Snapshot the native planes a dab changes in `Saved`, restore them on cancel, and pad them on growth.
- [x] 2.5 Union native dirty rectangles so a native-only change commits and refreshes.

## 3. App

- [x] 3.1 Remove the stale `#1421` ceiling comment from `tool_catalog.cpp` and keep the 32-bpc gate.

## 4. Verification

- [x] 4.1 Run the focused `pictura-paint` tone tests and the Qt retouch suite.
- [x] 4.2 Run `cargo fmt --all`, `cargo clippy`, and `scripts/verify-fast.sh`.
- [x] 4.3 Update `docs/dev/STATE.md` with the shipped behavior and archive the change.
