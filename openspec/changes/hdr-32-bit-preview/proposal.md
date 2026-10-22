# Proposal

## Why

A 32-bit document has no `View > 32-Bit Preview Options` command, its Info panel "32-bit" depth only scales the 8-bit composite, and the color picker cannot express values above `1.0`. The parity audit records all three as missing (`IMG-18`, issue #570); the HDR picker is also tracked separately as `CLR-5` (#602). A 32-bit workflow therefore cannot preview its dynamic range or read and choose HDR values.

## What Changes

- Add `View > 32-Bit Preview Options…` with `Exposure and Gamma` and `Highlight Compression`, plus a per-view `32-Bit Exposure` status-bar slider. The transform rebuilds the canvas frame from the current 32-bit source without touching the document composite, history, or saved pixels. CS6 stores the View preview white point in PSD/PSB/TIFF; persisting it is a recorded ceiling in this change.
- Make the preview and the Info panel use the current source: the retained `f32` store while it is current, and the current composite after an edit, so neither shows pre-edit pixels.
- Make the Info panel's 32-bit colour depth read the current unclamped `f32` samples (including colour samplers) when the document has them, and keep the existing scaled 8-bit fallback.
- Add an HDR mode to the color picker: an Intensity slider in stops, 32-bit float RGB fields, preview swatches with `Preview Stop Size` and `Relative to Document`, and Add to Swatches. `ColorState` and the swatch library store the float triple alongside the clamped `QColor` mirror.
- Route the foreground/background swatch entry points to the HDR picker for 32-bit documents and keep the ordinary picker for 8/16-bit documents.
- Keep the ordinary 8-bit picker unchanged. Painting and tool colour consumption remain 8-bit; the HDR picker authors the colour state, and this ceiling is recorded in the specs and the parity register.

## Capabilities

### New Capabilities

- `ui/hdr-preview`: the 32-bit display-only preview dialog, the per-view status-bar exposure, and the per-view transform that feeds the canvas.

### Modified Capabilities

- `ui/info-histogram-panel`: the colour bit-depth requirement gains true current-source f32 sampling, unclamped 32-bit readouts, and sampler coverage.
- `ui/color-picker`: the picker entry points route 32-bit documents to the HDR picker, and the HDR picker requirement covers the new controls.
- `ui/color-swatches-panel`: HDR swatches keep the float triple across selection and session persistence.

## Impact

- `crates/pictura-adjust`: a reusable display transform for Exposure & Gamma and Highlight Compression (shared with the conversion methods).
- `crates/pictura-app` Rust: per-view preview state, the display-frame builder, a retained-store freshness signal, and a free-function float sampling bridge (the `cxxqt_object.rs` root is at the 1200-line cap).
- `crates/pictura-app` C++: a new `HdrPreviewOptionsDialog`, a new `HdrColorPickerDialog` (kept out of the already-large `color_picker_dialog.cpp`), a status-bar exposure control, Info panel 32-bit path, `ColorState`/swatch changes.
- Tests: new `tst_hdr_preview`; updates to `tst_info_panel`, `tst_color_picker`, `tst_swatches_panel`, `tst_session_store`, and the command-tree/menu tests.
- Records: `ui/color-swatches-panel` gains the float-swatch requirement; #570 and #602 are both closed by this change.
- No new dependencies. The file-persisted 32-bit preview and 32-bit tool colour consumption are recorded ceilings.
