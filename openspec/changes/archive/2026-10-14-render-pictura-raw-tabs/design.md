# Design

## Context

See `proposal.md` — Why. Current state that shapes the approach:

- `crates/pictura-adjust/src/pictura_raw.rs` (378 LOC) renders the 11 Basic controls over
  shared `f32` planes and short-circuits to a byte-identical no-op when all Basic controls
  are zero. It is the only render path: `render_pictura_raw` is called from
  `pictura-render`'s smart-filter chain and the document op.
- The settings model (`crates/pictura-core/src/raw_settings.rs`) already carries every tab,
  but `Hsl` holds `legacy_red/green/blue_{hue,saturation}` and `PicturaRawSettings` holds
  `legacy_blacks` (`BlkB`). A reference filter produced with the Camera Calibration
  primaries and shadow tint set off-default changes exactly `RHue`, `RSat`, `GHue`, `GSat`,
  `BHue`, `BSat` and `BlkB` and nothing else, so those seven fields are the Camera
  Calibration tab, not HSL/legacy blacks.
- The 111-key baseline reference omits the calibration keys because Adobe drops
  default-valued keys; only the off-default probe reveals them.
- `Dhze` (Dehaze) is a real integer `Fltr` key (`14` in the baseline), parked in the
  preserved set because the stated settings target is earliest-CC/PV2012.
- The renderer must stay deterministic and CPU-only (golden comparison runs on CPU).

## Goals / Non-Goals

**Goals:**
- Render every modeled tab, in tab order, from the same settings record that drives the
  preview, with a default value still a byte-identical no-op.
- Correct the settings model so the Camera Calibration primaries and shadow tint live under
  `CameraCalibration`, and model Dehaze.
- Keep each render file and the tests under the file-size caps.
- One runnable check per stage.

**Non-Goals:**
- Byte or numeric parity with Adobe: the PV2012 curves, primary math, noise model and grain
  synthesis are closed and every stage is an approximation.
- Local adjustments, Presets, Snapshots, Workflow Options, or the tabbed dialog (#395).
- Changing the bridge signatures or the public no-argument render entry point.

## Decisions

### D1 — Re-home the Camera Calibration controls and model Dehaze

Move `legacy_red_hue`, `legacy_red_saturation`, `legacy_green_hue`,
`legacy_green_saturation`, `legacy_blue_hue`, `legacy_blue_saturation` from `Hsl` to
`CameraCalibration` as `red_hue`, `red_saturation`, `green_hue`, `green_saturation`,
`blue_hue`, `blue_saturation`; move `PicturaRawSettings.legacy_blacks` to
`CameraCalibration.shadow_tint`; add `Effects.dehaze: Option<i32>`.

- **Why**: the calibration probe proves the `Fltr` vocabulary repurposes `RHue`/`RSat`,
  `GHue`/`GSat`, `BHue`/`BSat` and `BlkB` for the Camera Calibration tab. The existing
  field names describe the wrong tab and would render the wrong stage.
- **Alternatives**: keep the fields and layer a second alias — rejected as duplicate state
  with two names for one key. Invent new keys for primaries — rejected: no reference key
  exists and the keys already carry the values.
- The on-disk keys do not change; only the owning sub-struct does, so the codec's write
  bytes for a given control value are unchanged.

### D2 — Split the renderer into a module over shared `f32` planes

`crates/pictura-adjust/src/pictura_raw.rs` becomes `pictura_raw/` with `mod.rs` (validate,
planes, no-op guard, ordered stage calls, write-back), one file per tab, and `tests.rs`.

- **Why**: 378 LOC plus seven stages of code and tests exceeds the 800-LOC target. Splitting
  along the tab seam mirrors the codec module split.
- Stages are functions over the shared `r`/`g`/`b` `f32` planes (plus dimensions), **not**
  `PixelBuffer -> PixelBuffer`, so no stage quantizes to 8-bit and each stage's `None`
  controls leave the planes untouched.

### D3 — Pipeline order is the tab order

Basic (WB, exposure, contrast, highlights/shadows/whites/blacks, clarity, vibrance,
saturation) → Tone Curve → Detail → HSL/Grayscale → Split Toning → Lens Corrections →
Effects → Camera Calibration. Presets has no render stage.

- **Why**: the project targets the dialog's tab order; keeping render and UI order aligned
  makes the two comparable. Adobe applies the calibration colour transform earlier, but the
  primaries approximation is small and order-insensitive at the tolerance we claim.

### D4 — One bounded approximation and one check per stage

| Stage | Inputs (modeled) | Approach | Check |
|---|---|---|---|
| Tone Curve | `PC_S/D/L/H`, splits, `Crv*` | parametric region weights added to a monotone spline through the point curve, baked to a 256 LUT | output monotone on a ramp; identity at empty/identity |
| Detail | `Shrp`, `ShpR`, `ShpD`, `ShpM`, `LNR `, `CNR ` | f32 blur-difference USM with an edge mask (`ShpM`), luma detail weighting, luma/chroma amount reduction | edge contrast rises with amount, masking gates flat areas |
| HSL/Grayscale | `HA/SA/LA_*`, `GM_*`, `CtoG` | weighted-range HSL (hue/sat/lum) as in the shipped Hue/Saturation, gated by chromatic presence; grayscale mix as eight weighted luma terms | each range moves its own hue; a neutral is untouched; grayscale mix weights by hue |
| Split Toning | `STSH/SS/HH/HS`, `STB ` | luminance-gated hue/sat tint with balance biasing the crossover | shadows and highlights tint oppositely |
| Lens Corrections | `MDis`, `DfPA/GA` + hue windows, `VigA/M` | radial resample (distortion), hue-window desaturation (defringe), radial gain (vignette) | distortion moves a feature radially; defringe desaturates only its hue band; vignette darkens corners |
| Effects | `GRNA/S/F`, `Dhze`, `PCVA/M/F/S/H` | seeded grain scaled by size/frequency, contrast-stretch dehaze, post-crop vignette with the three styles | grain deterministic at a fixed seed; vignette corners darken; dehaze raises midtone contrast |
| Camera Calibration | `RHue/RSat/GHue/GSat/BHue/BSat`, `BlkB` | per-primary hue rotate + sat scale via the weighted-range model; shadow tint as a green↔magenta shift weighted by `1 - luma`; `CamP`/`CP_D` text and `PrVe` are metadata | each primary rotates its own hue; shadow tint biases dark pixels while highlights hold |

Some modeled controls are carried in the settings model but **not applied** by this
change, and are treated as metadata or out of scope: the noise-reduction sub-controls
`LNRD`/`LNRC`/`CNRD`/`CNRS` (only the two amounts are used), the lens profile flag
`LPEn` and the perspective/rotate/offset fields (`PerV/H/S/A/U`, `PerR`, `PerX`, `PerY`,
out of scope per `FILT-100`), and the post-crop vignette roundness `PCVR`. They round-trip
through the codec and leave the render untouched.

- **Why**: the algorithms are closed, so the cheapest defensible model is the one that
  reuses the shipped weighted-range and blur helpers and is monotone in its control.
  Every stage carries a `ponytail:` comment naming the ceiling.

### D5 — Whole-settings no-op guard

`mod.rs` returns `input.clone()` when every field of every sub-struct is unset or a
zero/default, computed by a single `is_default()`-style check over the whole value, not
per stage.

- **Why**: the spec requires default settings to be byte-identical, and per-channel curves,
  grayscale mix and grain all risk drifting pixels at defaults. A stage may also self-skip
  when its own controls are unset, but the top-level guard is what the test pins.

### D6 — Regenerate the synthetic fixture and correct the docs

`examples/make_smart_fixtures.rs` authors Dehaze as a modeled control and puts the
primaries/shadow tint under `calibration`, dropping the HSL legacy fields; the fixture is
regenerated by running the example. `docs/dev/camera-raw-cc-notes.md` §Fltr key map moves
`RHue`/`RSat`/`GHue`/`GSat`/`BHue`/`BSat` and `BlkB` to Camera Calibration and `Dhze` to
modeled.

- **Why**: acceptance is "every modeled key round-trips on the fixture" and the notes are
  the observed key-map record. No bytes or values are copied from a reference file.

## Risks / Trade-offs

- **[Calibration/grain/noise math is invented, not Adobe's]** → accepted and stated; the
  checks prove the documented behaviour (monotonicity, no-op, direction), never Adobe
  parity.
- **[Distortion resampling can expose edge gaps]** → clamp to the nearest edge sample and
  document the ceiling; no tiling or interpolation cleverness.
- **[The no-op guard is easy to get subtly wrong]** → one test asserts default settings are
  byte-identical for both the old Basic-only value and a value with a single zero-set
  control in each tab.
- **[Re-homing changes the public settings shape]** → internal only; no on-disk format
  change, and the call sites already use struct-update syntax.
- **[Rendering is O(pixels × stages)]** → fine for the 8-bit smart-filter path; the
  large-image tiling work is tracked separately.

## Migration Plan

1. Re-home the settings fields and add Dehaze in `pictura-core`; update the codec and its
   tests.
2. Regenerate the synthetic fixture; correct the CC notes.
3. Move `pictura_raw.rs` into a module directory as a pure move, keeping the existing tests
   green.
4. Add the stage modules one tab at a time, each with its check.
5. Run fmt, clippy, nextest, doctests, `openspec validate --all --strict`, and
   `scripts/verify-fast.sh`.

Rollback: revert the change commit; the fixture is regenerated by the example.

## Open Questions

- Whether `AuCA` (auto chromatic aberration) and `TMMs` warrant modeling later; both stay
  preserve-only and observed at `0`.
- Whether the Dehaze approximation is good enough to surface in the dialog (#395) or should
  be marked experimental there.
