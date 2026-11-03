# Tasks

## Tests

- [x] Tests: Rust store round-trip, reserved names, malformed lines, kind
      mapping, decoded matching, and the save/menu kind sets
- [x] Tests: `tst_adjustment_presets` menu, application history, `Custom`,
      save/load, whitelist, dialog loading, built-ins, Photo Filter density,
      restart, and malformed store

## Engine

- [x] Add `pictura-render::adjustment_presets` with kind mapping, decoded block
      matching, the save whitelist, and the XDG store
- [x] Export the module from `pictura-render/src/lib.rs`

## Bridge

- [x] Add the `adjustment_presets` cxx-qt bridge and register it in
      `crates/pictura-app/build.rs`
- [x] Add `adjustment_block` and `adjustment_set_block` to `adjustment_edit.rs`

## Shell

- [x] Add `cpp/adjustment_presets.{h,cpp}`: built-in registry and `PresetCombo`
- [x] Register the new source in `CMakeLists.txt` and the test suite in
      `crates/pictura-app/cpp/tests/CMakeLists.txt`
- [x] Properties panel: preset combo, Save/Load panel-menu actions, and the
      menu hook
- [x] `AdjustmentDialog`: `reloadFromBlock`, `addPresetRow`, and generic preset
      row
- [x] Levels, Curves, Photo Filter, Exposure, Hue/Saturation, Black & White,
      Channel Mixer, Selective Color, and Vibrance dialogs list and apply saved
      presets

## Docs

- [x] Tick `ROADMAP.md` item for #576
- [x] Close `ADJ-6` in `docs/dev/doc-code-parity-gaps.md`
- [x] Record the change in `docs/dev/STATE.md`

## Verify

- [x] `openspec validate --all --strict`
- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo nextest run --workspace` (the only failure is the unrelated
      `pictura-render::oracle::imagemagick_fixtures_reproduce` fixture)
- [x] `bash scripts/verify-fast.sh` (same unrelated oracle failure)
