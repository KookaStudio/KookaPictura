# Shadows/Highlights

- **Spec ID:** `ADJ-024`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the command dates to Photoshop CS (2003). CS6 can apply it as a **Smart Filter** (the Help: "you can apply the Shadow/Highlight and Variations adjustments as Smart Filters"), and its advanced options use the CS6 names **Tonal Width**, **Color Correction**, and **Midtone Contrast** (later CC 2014 renamed them Tone/Color/Midtone).
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-008` document-model, `ARCH-009` undo-history, `05-layers/smart-filters.md`, `05-layers/smart-filters.md`, `04-image-ops/image-modes.md`, `04-image-ops/32-bit-hdr.md`, `01-architecture/color-management.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Shadows/Highlights** is a local tonal correction: "The Shadow/Highlight command
does not simply lighten or darken an image; it lightens or darkens based on the
surrounding pixels (local neighborhood) in the shadows or highlights. For this
reason, there are separate controls of the shadows and the highlights. The
defaults are set to fix images with backlighting problems."

- **Menu.** `Image > Adjustments > Shadow/Highlight`. There is **no adjustment
  layer** for it; the Help states the edit "applies adjustments directly to the
  image and will discard image information." Non-destructive use is via a Smart
  Filter or by duplicating the layer.
- **Progressive disclosure.** On open, only **Shadows Amount** and **Highlights
  Amount** are visible; **Show More Options** reveals Tonal Width, Radius, and the
  Adjustments section (the Help: "For finer control, select Show More Options to
  make the additional adjustments").
- **Live preview.** A **Preview** checkbox updates the canvas as sliders move.
- **Guidance in the Help.** "To increase shadow detail in an otherwise well-exposed
  image, try values in the 0-25% range for Shadows Amount and Shadows Tonal
  Width." The Help also warns of **crossover**: "Extreme Amount values may lead to
  a crossover, where what started as a highlight becomes darker than something
  that started as a shadow."
- **Grayscale-only control.** **Brightness** "is available only for grayscale
  images."
- **Defaults management.** **Save As Defaults** stores the current settings as the
  command defaults; holding `Shift` while clicking restores the original defaults.
  **Save** / **Load** persist settings to a file (the Help: "you can reuse
  Shadow/Highlight settings by clicking the Save button … and later using the Load
  button").
- **Smart Filter.** On a Smart Object, `Image > Adjustments > Shadow/Highlight`
  becomes an editable, maskable Smart Filter (re-editable and removable), which is
  the supported non-destructive path.
- **32 bpc.** Shadow/Highlights is **not** in the CS6 32-bpc supported list (the
  32-bpc adjustment-layer whitelist is Levels, Vibrance, Hue/Saturation, Channel
  Mixer, Photo Filter, Exposure) and is expected to be unavailable at 32 bpc;
  supported at 8 and 16 bpc. *(32-bpc unavailability inferred)*

### Controls (CS6 names)

| Section | Control | Function (Help wording) |
|---|---|---|
| Shadows | Amount | "how much of a correction to make" (lightens shadows) |
| Shadows | Tonal Width | "range of tones in the shadows … that are modified" |
| Shadows | Radius | "size of the local neighborhood around each pixel" |
| Highlights | Amount | darkens highlights |
| Highlights | Tonal Width | range of tones treated as highlights |
| Highlights | Radius | neighborhood size for highlight detection |
| Adjustments | Brightness | grayscale-image brightness (grayscale only) |
| Adjustments | Midtone Contrast | contrast in midtones |
| Adjustments | Color Correction | saturation compensation |
| Adjustments | Black Clip / White Clip | percentage clipped to level 0 / level 255 |
| Footer | Save As Defaults / Reset Defaults | defaults management |

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Shadow/Highlight` | Menu command | — | Destructive direct adjustment |
| Dialog — Amount sliders | Slider + numeric | — | Shadows and Highlights, always visible |
| Dialog — Show More Options | Disclosure | — | Reveals Tonal Width, Radius, Adjustments |
| Dialog — Preview | Checkbox | `P` *(inferred)* | Live canvas update |
| Dialog — Save As Defaults / Reset Defaults | Button | `Shift`+click resets | Persists defaults |
| Dialog — Save / Load | Button | — | Settings file |
| Smart Object + `Image > Adjustments > Shadow/Highlight` | Smart Filter | — | Non-destructive, maskable |
| Duplicate layer (`Ctrl+J`) | Workflow | `Ctrl+J` | Manual non-destructive path recommended by the Help |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Shadows Amount | percent | 35 *(secondary; see Open questions)* | 0–100 | Lightening strength |
| Shadows Tonal Width | percent | 50 | 0–100 | Help: "set to 50% by default" |
| Shadows Radius | pixels | 30 *(secondary)* | see Open questions | Neighborhood size |
| Highlights Amount | percent | 0 *(secondary)* | 0–100 | Darkening strength |
| Highlights Tonal Width | percent | 50 *(inferred)* | 0–100 | Same default as shadows |
| Highlights Radius | pixels | 30 *(inferred)* | same as shadows | Neighborhood size |
| Brightness | integer | 0 *(inferred)* | −100…+100 *(inferred)* | Grayscale documents only |
| Color Correction | integer | +20 *(secondary)* | −100…+100 *(inferred)* | Saturation compensation |
| Midtone Contrast | integer | 0 *(secondary)* | −100…+100 *(inferred)* | Midtone S-curve |
| Black Clip | percent | 0.01 *(secondary)* | 0–10 *(inferred)* | Clip to level 0 |
| White Clip | percent | 0.01 *(secondary)* | 0–10 *(inferred)* | Clip to level 255 |
| Preview | bool | On | on/off | Dialog behavior |

The CS6 Help prints no numeric defaults or ranges; defaults come from a
CS6-compatible secondary tutorial (Shadows Amount 35 %, Highlights 0 %, Tonal
Width 50 %, Color Correction +20, clips at defaults) and a 2025 secondary article
(which reports a newer 50 % shadows default). See `## Open questions`.

## Algorithms & pipeline

Behavioral parity only. The Help describes the *semantics* (local-neighborhood
lightening/darkening) but not the kernel; everything below is *(inferred)*.

### Local surround and tonal membership

For each pixel with luminance `Y ∈ [0,1]`:

1. Compute a blurred luminance `B = GaussianBlur(Y, radius)` — the local
   neighborhood average controlled by **Radius**.
2. Define a shadow membership `w_s(B)` and highlight membership `w_h(B)`. **Tonal
   Width** controls the width/pivot of these memberships: smaller widths restrict
   the effect to the darkest (shadows) or lightest (highlights) regions; larger
   widths reach into midtones; at 100 % shadows reach the midtones but "the
   brightest highlights are not affected."
3. The Help's default (50 %) places the shadow/highlight split near middle gray,
   so the two Amounts act on the darker and lighter halves respectively.

### Amount application

A family of local gain/gamma curves is consistent with "lightens or darkens based
on the surrounding pixels"; one workable model:

```
shadow lift:      Y_s = Y + A_s · w_s(B) · (1 − Y)
highlight pull:   Y_h = Y · (1 − A_h · w_h(B))
Y' = clamp( Y_h after Y_s )      # order to be calibrated
```

where `A_s, A_h ∈ [0,1]` are the Amounts. Alternatives (local gamma
`Y' = Y^(1/(1+k·w))`) are equally plausible; the exact Adobe curve is closed.
Because both masks are broad, large Amounts can make a dark pixel overtake a
lighter one — the Help's **crossover** warning — so no monotonicity guarantee
should be enforced.

### Adjustments section

- **Color Correction** (a saturation control): scales chroma by roughly
  `chroma' = chroma · (1 + color/100)` (in Lab/HSB), used to restore saturation lost
  when shadows are lifted. Range and pivot *(inferred)*.
- **Midtone Contrast**: an S-curve pivoted at midtones; "Increasing midtone
  contrast produces greater contrast in the midtones while tending to darken the
  shadows and lighten the highlights."
- **Black Clip / White Clip**: histogram-percentile clipping. Compute the `p`-th
  percentile of the luminance histogram (`p` = clip value) and map it to 0 (black)
  or 255 (white), stretching the remainder. This matches "how greatly the shadows
  and highlights are clipped to the new extreme shadow (level 0) and highlight
  (level 255) colors."
- **Brightness**: grayscale-only additive lift/darken.

### Precision

- 8-bpc: integer in/out; clip percentages operate on the 256-bin histogram.
- 16-bpc: 65536-bin histogram; same relative behavior.
- 32-bpc: unavailable *(inferred)*.
- Because the operator is local and multi-stage, double application is not the
  identity and must not be assumed associative.

## Rust module mapping

- `pictura_adjust::shadow_highlight` — `ShadowHighlightOp { shadows: BandParams,
  highlights: BandParams, brightness: Option<i32>, midtone_contrast: i32,
  color_correction: i32, black_clip: f32, white_clip: f32 }`.
- `pictura_adjust::shadow_highlight::surround` — `fn local_surround(luma: &TilePatch, radius_px: f32) -> BlurredLuma`
  (separable Gaussian; radius ties into the shared blur primitive).
- `pictura_adjust::shadow_highlight::bands` — `BandParams { amount, tonal_width, radius }`
  and tunable `shadow_weight` / `highlight_weight` curves (calibration knobs).
- `pictura_adjust::shadow_highlight::clip` — histogram-percentile clip helper
  shared with Levels semantics.
- `pictura_adjust::traits` — `Adjustment`, `AdjustmentKind::ShadowHighlight`.
- Smart-filter path reuses `pictura_filters::smart` to wrap the op as a cached,
  maskable, re-editable filter node.
- `pictura_render::adjust` — optional GPU variant; the blur is the dominant cost,
  so GPU blur is worthwhile, with the CPU path as reference.

Crossing types: `ShadowHighlightParams`, `BandParams { amount: u8, tonal_width: u8, radius_px: f32 }`,
`ClipPercent(f32)`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ShadowHighlightDialog` | `QDialog` | Amount sliders + Show More Options + Preview + Save/Load/Defaults |
| `BandControlGroup` | `QWidget` | Reusable Amount / Tonal Width / Radius triple for Shadows and Highlights |
| `AdjustmentsGroup` | `QWidget` | Brightness (grayscale-gated), Midtone Contrast, Color Correction, Black/White Clip |
| `SmartFilterPropertiesProxy` | `QWidget` | Re-edits the same params when applied as a Smart Filter |
| `BlurRadiusField` | `QWidget` | Pixel-radius scrubber shared with blur filters |

A `QDialog` (not a dockable panel) matches CS6, which is a modal modeless-style
dialog rather than an adjustment layer.

## Data-model impact

- **No adjustment layer.** The op is not a layer content type. It is either a
  destructive pixel edit on the active layer or a Smart Filter node attached to a
  Smart Object.
- **Smart Filter record.** Parameters serialize as a filter node with the same
  parameter block; the filter mask is a standard mask channel. The record must
  carry the filter version for re-editability.
- **Settings files.** Save/Load writes the parameter block in a small settings file
  (CS6 format) separate from the PSD; define our own stable serialization and a CS6
  import shim.
- **Defaults.** Save As Defaults writes to the application preference store
  (`11-cross-cutting/preference-storage.md`), not the document.
- **Undo.** Destructive apply = one history state of tile deltas. Smart Filter
  param changes = filter-node records; mask edits are separate.
- **Grayscale gating.** Brightness availability is a function of document mode, so
  the parameter block carries an optional Brightness only for grayscale documents.

## Edge cases

- **8 / 16 / 32 bpc** — 8 and 16 supported; 32 expected unavailable. 16-bit clip
  uses a 65536-bin histogram.
- **Grayscale** — Brightness control enabled; Color Correction should be a no-op or
  hidden (no chroma).
- **CMYK / Lab** — the operator should run on luminance and preserve the color
  model; the chroma scaling on Color Correction needs a defined model-specific
  mapping (Lab is natural; CMYK needs care).
- **Crossover** — with large Amounts and/or overlapping Tonal Widths, highlights
  may end up darker than shadows; CS6 allows this, so do not clamp it away.
- **Halos** — the Help warns "too large a [Tonal Width] value may introduce halos
  around dark or light edges", and large Radius "tends to brighten (or darken) the
  whole image." These are expected artifacts, not bugs to fix.
- **Radius vs image size** — radius is in pixels, so results are resolution
  dependent; a huge document with a tiny radius is effectively a point operator.
- **Clip percentiles** — `0 %` must be a no-op; large values must not produce NaN
  or divide-by-zero when the histogram has empty bins.
- **Selection** — destructive path applies only within the active selection.
- **Empty / 1-px / huge documents** — tile-local surround with halo margins at tile
  borders; a 1×1 document degenerates to a point operator.
- **GPU unavailable** — identical CPU result; the blur margin must match exactly.
- **Undo/redo** — destructive exact round-trip; Smart Filter re-edits are
  reversible at the node level.

## Parity acceptance criteria

- Given a backlit image, the default Shadows Amount lifts shadow detail while the
  default Highlights Amount (0 %) leaves highlights unchanged.
- Given a grey ramp and a large Shadows Tonal Width with a large Radius, dark tones
  are lifted far more than light tones; the effect follows the local surround, not
  a global curve (two identical pixels in different surrounds change differently).
- Given a flat mid-grey image, Shadows/Highlights produces little change (no local
  shadow/highlight population).
- Given an otherwise well-exposed image, Shadows Amount and Tonal Width near
  0–25 % increase shadow detail without washing midtones (per the Help's guidance).
- Given Color Correction at +20 (default), saturation roughly returns to the
  pre-lift level after a shadow lift; decreasing it desaturates.
- Given Midtone Contrast > 0, midtone contrast rises while shadows darken and
  highlights lighten; < 0 does the inverse.
- Given Black/White Clip at 0.01 %, no visible clipping occurs; raising them to a
  known percentile clips that fraction of pixels to pure black/white.
- Given a Grayscale document, the Brightness control is available; given an RGB
  document, it is hidden/disabled.
- Given a Smart Object, `Image > Adjustments > Shadow/Highlight` appears as a
  re-editable Smart Filter whose mask limits the effect.
- Given a 32-bpc document, the command is unavailable.
- Given a destructive apply, History records exactly one new state and undo
  restores pixels exactly at the document bit depth.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  local-neighborhood ("based on the surrounding pixels") definition and
  backlighting default purpose; the separate Shadows/Highlights controls; the
  Midtone Contrast / Black Clip / White Clip / Color Correction existence; the
  0–25 % shadow guidance; the crossover warning; the Tonal Width 50 % default and
  semantics including halos; the Radius semantics; the grayscale-only Brightness;
  Save As Defaults / Shift-reset / Save / Load behavior; "applies adjustments
  directly to the image and will discard image information"; the Smart Filter
  support statement; the 32-bpc adjustment whitelist that omits Shadow/Highlight.
- `https://www.photoshopessentials.com/photo-editing/restoring-hidden-detail-with-shadows-highlights-in-photoshop`
  — secondary source explicitly stated to be "fully compatible with Photoshop CS6".
  Establishes: the CS6 option names (Tonal Width, Color Correction, Midtone
  Contrast); the two-slider default view; Shadows Amount default 35 %, Highlights
  Amount default 0 %, Tonal Width default 50 %, Color Correction default +20;
  Black Clip / White Clip semantics; that it is not an adjustment layer.
- `https://note.com/mofp/n/nce90fd3c2e17` — 2025 secondary article: reports Shadows
  Amount 50 %, Radius 30 px, Color Correction +20, Midtone Contrast 0, clip 0.01 %
  in a newer Photoshop build; used only to flag the default drift (see Open
  questions), not asserted as CS6.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Shadows Amount default.** 35 % (CS6-compatible secondary) vs 50 % (2025
  secondary). Which is the CS6 factory default? Resolves with: a first-run CS6
  dialog capture.
- **Radius range.** Default 30 px is secondary; the slider maximum and whether it
  is documented are unknown (community values reach ~250). Resolves with: a CS6
  slider capture.
- **Exact local algorithm.** The surround blur, tonal-weight curve, and
  amount→gain mapping are closed. Resolves with: fitting to CS6 output on a
  controlled grey-ramp + surround test.
- **Order of operations.** Whether shadows then highlights, or a combined solve, and
  where Midtone Contrast / clips apply in the chain. Resolves with: staged CS6
  probes.
- **Midtone Contrast / Color Correction formulas.** Not documented. Resolves with:
  a contrast-sweep and saturation-sweep comparison.
- **Black/White Clip range.** 0–10 % is inferred; confirm. Resolves with: a CS6
  capture.
- **CMYK / Lab behavior.** Whether the operator is available and how Color
  Correction maps. Resolves with: a mode test.
- **32-bpc availability.** Confirm the command is disabled at 32 bpc. Resolves
  with: a CS6 mode test.
- **`P` preview shortcut.** Whether `P` toggles Preview in CS6 as in later
  versions. Resolves with: a CS6 keyboard test.
