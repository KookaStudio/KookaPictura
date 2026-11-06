# Proposal: color-balance-lab-cmyk

## Why

Issue #572, from the docs↔code parity audit (#486), row ADJ-2. The Color
Balance contract (`docs/04-image-ops/adjustments/color-balance.md`) says the
numeric values above the colour bars are the red, green, and blue channel
changes, and that for Lab images they are the **A and B** channel changes. The
engine (`crates/pictura-adjust/src/color.rs`) and its native-depth twin apply
the three shifts to the working RGB planes unconditionally, so a Lab document
gets an RGB shift rather than an `a`/`b` shift, and a CMYK document has no
C/M/Y semantics. The dialog (`color_balance_dialog.cpp`) shows the three
editable per-bar values but no mode-aware channel readout.

The working raster is always RGB: a Lab or CMYK file is converted on read and
keeps its source mode (`Document::source_mode`). The mapping therefore runs in
the adjustment kernel, selected from the document's visible mode, and the RGB
path stays byte-identical.

## What Changes

- **Engine mode.** `pictura-adjust` gains `ColorBalanceMode { Rgb, Lab, Cmyk }`
  and `apply_in_mode` / `apply_native_in_mode`. Color Balance converts the
  working RGB to the document's channel space, applies the band-weighted shift
  there, and converts back:
  - **Lab:** `a += cyanRed − magentaGreen`, `b −= yellowBlue` (positive slider
    toward red, green, or blue moves the named axis; negative moves the
    complement). Preserve Luminosity leaves `L` unchanged.
  - **CMYK:** each pair lowers its right-hand colour's ink with `K` held
    (`C −= cyanRed`, `M −= magentaGreen`, `Y −= yellowBlue`). The working
    raster has no retained K plate, so the model uses `K = 0` (no black
    generation); on a neutral grey this yields red, green, and blue correctly.
  - **RGB:** the existing kernel, bit-identical.
- **Render plumbing.** `apply_adjustment_region` takes the mode; the
  adjustment-layer compositor derives it from `document_color_mode(doc)`. The
  destructive dialog, filter-job, and preview paths pass the same mode.
- **Dialog readout.** A read-only row above the bars shows the active tone's
  channel changes in the document's mode: `R/G/B` for RGB, `A/B` for Lab,
  `C/M/Y` for CMYK. It follows the tone radio and the per-bar values.
- **Docs.** Tick the roadmap item and close parity gap ADJ-2.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `imaging/image-adjustments`: the Color Balance requirement gains the
  mode-aware channel mapping and the mode-aware numeric readout.

## Impact

- `crates/pictura-adjust` (`types.rs`, `apply.rs`, `color.rs`, `native.rs`,
  `tests.rs`, `tests/native_depth.rs`) and its `Cargo.toml`.
- `crates/pictura-render` (`filter.rs`, `composite_native.rs`, tests).
- `crates/pictura-app` (`image_adjust/jobs/imp.rs`, `impl_filters.rs`,
  `filter_tools/jobs/imp.rs`, `color_balance_dialog.{h,cpp}`,
  `tests/tst_image_adjustments.cpp`).
- `crates/pictura-color` (`model.rs`): f64 unit Lab conversions so the native
  path shares the one colour model.
- New workspace dependency: `pictura-adjust` depends on `pictura-color` for
  the shared D50 sRGB↔Lab math (no new external crate; the binary already
  links lcms2 through the colour-management stack).
- RGB documents remain byte-identical; no golden baseline changes.
- The Lab and CMYK models are approximations of Adobe's closed kernel and are
  marked `ponytail:` in code; no parity is claimed.
