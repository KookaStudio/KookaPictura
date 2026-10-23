# Design

## Context

- `docs/06-filters/filters-overview.md` records the only behavioural fact: the filter "restricts the gamut of colors to those acceptable for television reproduction, to prevent oversaturated colors from bleeding across television scan lines". It lists NTSC Colors for 8-, 16- and 32-bpc documents.
- The filter engine is planar `u8` (`pictura_core::PixelBuffer`, 3 or 4 channels). No filter has 16/32-bpc plumbing, so this change follows the existing engine depth.
- Filter kinds are recorded in three places that the existing guards keep in step: the engine's `Filter`/`FilterKind` tables, the app's parameter descriptor, and the mapping. Every variant also needs a row in the exhaustive test tables (`tests/common/instances.rs`, `tests/kernel_digests.rs`, `tests/oracle/mapping.rs`).
- `pictura-color` owns colour management (lcms2 profiles, conversion, assignment). It has no path dependency on the other engine crates. `pictura-filters` currently depends on `pictura-core` only.
- Issue #269 moves colour-model conversions (sRGB↔Lab, CMYK) from the C++ picker into `pictura-color`, establishing it as the home of colour math.

## Goals / Non-Goals

**Goals:**

- A deterministic, documented approximation of NTSC Colors that reduces saturation without shifting hue or luma.
- The colour math in `pictura-color` with its own tests, and the pixel-loop kernel in `pictura-filters`.
- A parameterless menu entry that applies immediately and is enabled exactly when the active layer is a filter target.
- Engine tests that pin the clamp, alpha preservation and refusal, plus a Qt Test for the menu wiring.

**Non-Goals:**

- Reproducing Adobe's closed kernel bit for bit. There is no oracle, so the spec records an approximation.
- 16- and 32-bpc filter support. That is an engine-wide gap shared with every other filter.
- De-Interlace (#344). It extends the same `imaging/video-filters` capability later.
- A dialog. CS6's NTSC Colors entry has none.

## Decisions

### The colour math lives in `pictura-color`, the pixel loop in `pictura-filters`

`pictura_color::ntsc` exposes the sRGB↔YIQ matrices, the chroma limit `NTSC_MAX_CHROMA` and `clamp_to_ntsc_gamut`. `pictura_filters::video::ntsc_colors` validates the buffer, runs the helper per pixel and writes the three colour planes back.

This follows the issue's direction and #269's split: colour-model math belongs to the colour crate, and the filter crate keeps the buffer traversal and error contract. The alternative — keeping both in `pictura-filters` — would leave the YIQ conversion unavailable to the colour picker and any later gamut display feature.

`pictura-filters` gains a dependency on `pictura-color`. That pulls `lcms2` into the filter crate's build, which is the cost of the split. There is no cycle, because `pictura-color` depends only on `lcms2` and `thiserror`.

### The clamp is a YIQ chroma-radius reduction

Adobe's kernel is closed; the docs specify only the effect. The chosen approximation:

1. Normalise the 8-bit samples to `0..=1`.
2. Convert sRGB to YIQ with the SMPTE/NTSC matrix:
   - `Y = 0.299 R + 0.587 G + 0.114 B`
   - `I = 0.595716 R - 0.274453 G - 0.321263 B`
   - `Q = 0.211456 R - 0.522591 G + 0.311135 B`
3. If `sqrt(I² + Q²) > 0.5`, scale both `I` and `Q` by `0.5 / sqrt(I² + Q²)`.
4. Convert back with the inverse matrix and clamp each channel to `0..=1` for floating-point safety.

Uniformly scaling `(I, Q)` keeps their ratio, so hue is unchanged, and `Y` is untouched, so luma is unchanged. Scaling toward the achromatic axis is an interpolation between two points inside the RGB cube, so the result stays in range without tone clipping. `0.5` is the video-safe chroma radius: fully saturated red, green, magenta and cyan exceed it, while a neutral or muted colour does not.

Rejected alternatives:

- **Broadcast-legal Y′CbCr (Y′ 16–235, Cb/Cr 16–240).** This legalises signal levels but barely desaturates; it does not match the documented "prevent oversaturated colors from bleeding" effect.
- **Per-axis `I`/`Q` clamp to ±0.5.** The axes have different ranges and symmetry, so clamping them independently shifts hue on the diagonal hues.
- **`I`/`Q` clamped to their theoretical sRGB-cube extrema (`±0.5957`, `±0.5226`).** That is a no-op on every colour inside the cube.

### One descriptor row drives the menu

The descriptor carries `kind: "ntsc-colors"`, the path `Filter/Video/NTSC Colors`, an empty control list and the shared preview flags. `filterCommands()` builds the row from it, `wireFilterMenu()` marks it implemented, and `applyFilterCommand()` sees no parameters and commits with the engine defaults. The existing leaf in `command_tree.cpp` stays as the menu placement.

CS6 has no NTSC Colors dialog, so the label has no ellipsis; the generic `dialogEntriesEndWithEllipsis` test enforces that.

### 8-bit scope is explicit

The requirement says "planar 8-bit buffer". The docs' 16/32-bpc availability stays an engine-wide gap; this change does not claim it.

## Risks / Trade-offs

- **Approximation without an oracle.** The spec records the formula and the tests pin it, but it cannot prove Adobe parity. Marked as an approximation in the spec and the module docs (rule 5).
- **Dependency direction.** Pulling `pictura-color` into `pictura-filters` enlarges every filter build with `lcms2`. Accepted for the colour-math home.
- **Clamp constant.** `0.5` is a documented choice, not an Adobe value. Changing it later is a spec change with test updates, not a silent tuning knob.
