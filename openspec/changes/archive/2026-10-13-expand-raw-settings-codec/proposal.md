# Proposal

## Why

Pictura Raw (`filterID 2683`) currently models only the 11 PV2012 Basic controls, in
both the settings type and the on-disk `Fltr` codec. The reference Camera Raw Filter
carries a full tab set (Tone Curve, Detail, HSL/Grayscale, Split Toning, Lens, Effects,
Camera Calibration, White Balance and mode) — a reference Photoshop file shows **111
`Fltr` keys**. Until the settings model and codec cover them, an Adobe-authored filter
cannot be read into typed fields or authored back, and the later render (#394) and
dialog (#395) work has nothing to build on.

## What Changes

- **BREAKING (settings type)**: replace the `PicturaRawSettings = CrsSettings` alias
  with an owning `PicturaRawSettings` of per-tab sub-structs. `CrsSettings` stops being
  the Pictura Raw settings type; it is reused as the `basic` sub-struct and keeps its
  role as the `crs:` XMP view.
- Extend `encode_pictura_raw_fltr` / `decode_pictura_raw_settings` from 11 keys to the
  full verified key map, with per-key value types (`Long`, `Double`, `List<Long>` for
  point curves, `Text`, `Bool`, `Enumerated`). Only `Some`/non-empty fields emit a key,
  so encoding default settings remains a byte-identical no-op.
- Model the color and grayscale HSL variants (`HA_*`/`SA_*`/`LA_*` and `GM_*`) and the
  `CtoG` mode flag that selects between them.
- Preserve unmapped / later-CC keys (`Dhze`, `Upri`, `GuUr`, `Rtch`, `REye`, `LCs `,
  `AuCA`, `TMMs`) byte-for-byte on re-attach.
- Regenerate the synthetic `assets/test_with_smart_object02.psd` fixture from scratch so
  it carries the full modeled key set, and round-trip every modeled key through
  write/read.
- Correct the `docs/dev/camera-raw-cc-notes.md` §Fltr key map (trailing-space key names,
  the `GM_*` and `LNRD`/`LNRC`/`CNRD`/`CNRS`/`VigM`/`PC_*` keys, curve value type).

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `codec/pictura-raw`: the "Pictura Raw settings alias" requirement becomes an owning
  settings model with per-tab sub-structs (no longer an alias, no longer forbidden to
  define a second struct), and the "Pictura Raw Fltr codec" requirement grows from the
  11 Basic keys to the full verified key map with per-key value types and
  preserve-unmodeled behavior.

## Impact

- **Code**: `crates/pictura-core/src/crs.rs` (+ re-exports in `lib.rs`), a new
  `pictura-core` settings module; `crates/pictura-codec/src/pictura_raw.rs` (split into
  a module dir); `crates/pictura-codec/examples/make_smart_fixtures.rs`;
  `crates/pictura-adjust/src/pictura_raw.rs`; call sites in
  `pictura-render`, `pictura-app`, and codec tests.
- **Assets**: regenerated `assets/test_with_smart_object02.psd` (synthetic; no bytes or
  values copied from any external reference).
- **Docs**: `docs/dev/camera-raw-cc-notes.md` §Fltr key map (carries `TASK-ALLOWS-DOCS`).
- **Out of scope**: rendering the new tabs (#394), the tabbed dialog / live preview /
  histogram (#395), and local adjustments / snapshots / workflow options.
