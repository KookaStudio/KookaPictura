# Tasks: hex-shorthand-swatches

## 1. Tests first

- [x] 1.1 New `tst_color_picker` suite: hex prefixes, shorthand commit, invalid
  input, Add to Swatches append/cancel/disabled; register it in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 1.2 `tst_swatches_panel`: New uses the generated `Swatch N` name.

## 2. Picker

- [x] 2.1 Hex field: `#`/`0x` prefixes, three-digit shorthand, commit and
  normalise, invalid revert.
- [x] 2.2 Add to Swatches: name prompt (default `Swatch N`), append the current
  colour, disabled without a panel.
- [x] 2.3 `ColorPickerDialog::setDefaultSwatchGrid`; `PicturaMainWindow`
  registers the Swatches grid.

## 3. Swatches panel

- [x] 3.1 `SwatchGrid::nextSwatchName`; New uses it.

## 4. Docs and verification

- [x] 4.1 Remove `CLR-6` from the parity register and record the change in
  `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 4.2 `openspec validate --all --strict`; `bash scripts/verify-fast.sh`;
  archive the change.
