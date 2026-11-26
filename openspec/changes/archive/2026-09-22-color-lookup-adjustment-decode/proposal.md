## Why

`clrL` (Color Lookup) is the only whitelisted PSD adjustment key that
`decode_adjustment` still returns `None` for: its layers render as a no-op and
the block is preserve-only. Finishing it closes the adjustment-key set and lets
a real Photoshop Color Lookup layer render, matching the P3 goal of rendering
preserved data.

## What Changes

- Decode the `clrL` block — a `u16` version (`1`) followed by a version-16
  descriptor — into a new `Adjustment::ColorLookup(ColorLookupParams)`.
- Apply a `3DLUT` lookup whose embedded `LUT3DFileData` is a `.cube` file:
  parse it and sample trilinearly, honoring `dataOrder`/`tableOrder`.
  `abstractProfile`/`deviceLinkProfile`, a `.3DL`/`.LOOK` payload, or a
  malformed/oversized LUT is a no-op (documented ceiling, no Adobe pixel
  parity).
- Expose `encode_color_lookup(...) -> AdjustmentData` that authors a valid
  `clrL` block, and add an app `color-lookup` kind whose default is an identity
  LUT (neutral composite), plus an Adjustments panel `Color Lookup` row.
- **BREAKING** (spec-level): the "Deferred adjustment payloads remain no-ops"
  requirement drops `clrL`; only version-3 `phfl` stays deferred.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `adjustment-layer-rendering`: `clrL` decodes to typed params, the deferred
  requirement drops `clrL`, and `clrL` gains encode / fixture-oracle / app
  requirements.
- `image-adjustments`: a new Color Lookup adjustment requirement and a row in
  the ImageMagick no-equivalent oracle table.

## Impact

- `crates/pictura-adjust`: `Adjustment::ColorLookup` + `ColorLookupParams`
  (`src/types.rs`), a new `src/lut.rs` (`.cube` parser + trilinear sampler),
  and dispatch in `src/apply.rs`; re-export from `src/lib.rs`.
- `crates/pictura-render`: new `src/color_lookup.rs`
  (`decode_color_lookup`/`encode_color_lookup`), a `composite.rs` arm, and
  `src/lib.rs` re-exports.
- `crates/pictura-codec`: a round-trip case in `src/tests.rs` (the key is
  already in `ADJUSTMENT_KEYS`).
- `scripts/generate-fixtures.py` + `crates/pictura-codec/tests/fixtures/color_lookup.psd`;
  oracles in `crates/pictura-render/tests/oracle/adjustment_oracle.rs` and
  `crates/pictura-codec/tests/psd/agpsd_oracle.rs`.
- `crates/pictura-app`: `helpers.rs` kind, `panels/panel_group_menu.cpp`,
  `command_tree.cpp`, and a C++ self-test check (next free code 455).
- No new dependency; no `docs/` change.
