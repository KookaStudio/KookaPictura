# Auto Adjustments (Auto Tone, Auto Contrast, Auto Color)

- **Spec ID:** `ADJ-033`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Auto Tone, Auto Contrast, Auto Color, and the Auto Color Correction Options dialog are carried from CS5; CS6 only relocates the options to the **Properties panel menu** (CS5: Adjustments panel menu).
- **Depends on:** `04-image-ops/adjustments-overview.md`, `04-image-ops/adjustments/levels.md`, `04-image-ops/adjustments/curves.md`, `04-image-ops/image-modes.md`, `02-ui-ux/panels/properties-panel.md`, `11-cross-cutting/preference-storage.md`, `01-architecture/undo-history.md` (`ARCH-009`).

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

CS6 exposes three one-click automatic corrections under the `Image` menu, plus
the shared **Auto Color Correction Options** dialog that defines the defaults
they use. Source: CS6 reference, "Making quick tonal adjustments".

### Auto Tone

- `Image > Auto Tone` adjusts the black and white points **per channel**.
- Clips a portion of shadows/highlights in **each** channel and maps the lightest
  and darkest pixels in each channel to 255 and 0; intermediate values are
  redistributed. Contrast increases.
- Because channels are adjusted independently it "may remove color or introduce
  color casts."
- Default clipping: **0.1%** of either extreme.
- Uses the **Enhance Per Channel Contrast** algorithm.

### Auto Contrast

- `Image > Auto Contrast` adjusts contrast **without** adjusting channels
  individually, so it does **not** introduce or remove color casts.
- Clips shadow/highlight values, then maps the remaining lightest/darkest pixels
  to pure white (255) / pure black (0).
- Default clipping: **0.5%** of either extreme.
- Improves many photographic/continuous-tone images; "does not improve flat-color
  images."
- Uses the **Enhance Monochromatic Contrast** algorithm.

### Auto Color

- `Image > Auto Color` adjusts contrast and color by searching the image for
  shadows, midtones, and highlights.
- Defaults: **neutralises midtones using a target color of RGB 128 gray** and
  clips shadows/highlights by **0.5%**.
- Uses the **Find Dark & Light Colors** algorithm **plus Snap Neutral Midtones**.

All three warn that applying them via `Image > …` edits the image layer directly
and **discards image information**, and that no options can be adjusted in that
path. The non-destructive route is a Levels or Curves adjustment layer.

### Auto Color Correction Options

Reached from the Levels/Curves **Properties panel menu** (CS6) and by
`Alt`/`Option`-clicking the **Auto** button. It controls the Auto button in
Levels/Curves and the three `Image > Auto …` commands.

Algorithms (radio buttons):

| Algorithm | Behaviour | Used by |
|---|---|---|
| Enhance Monochromatic Contrast | Clips all channels identically, preserving the overall color relationship while lightening highlights and darkening shadows. | Auto Contrast |
| Enhance Per Channel Contrast | Maximises tonal range in each channel; may remove/introduce color casts. | Auto Tone |
| Find Dark & Light Colors | Finds the average lightest and darkest pixels and maximises contrast while minimising clipping. | Auto Color |

Additional controls:

- `Snap Neutral Midtones` — finds an average nearly-neutral color and adjusts
  midtone (gamma) values to make it neutral. Auto Color uses it.
- `Clip` percentages for shadows and highlights; **0.0%–1% recommended**.
  Photoshop's stock default is **0.1%** (the Help notes this may be too high for
  modern cameras/scanners).
- Target color swatches for shadows, midtones, and highlights.
- `Save as Defaults` — persists the settings so the Auto button and the three
  commands use them.

Important nuance (Help verbatim intent): when settings are saved as defaults, the
**algorithm radio choice does not matter** for the three `Image > Auto …`
commands; they use only the saved target colors and clipping — **except** Auto
Color, which also uses `Snap Neutral Midtones`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Auto Tone` | Menu command | none documented | Destructive to the image layer; automatic. |
| `Image > Auto Contrast` | Menu command | none documented | Destructive; automatic. |
| `Image > Auto Color` | Menu command | none documented | Destructive; automatic. |
| Levels/Curves Properties panel menu | Menu | — | Opens **Auto Color Correction Options**. |
| Levels/Curves Auto button | Button | `Alt`/`Option`-click opens options | Applies the current/saved auto settings. |
| Auto Color Correction Options dialog | Dialog | — | Algorithm radios, Snap Neutral Midtones, Clip fields, target swatches, Save as Defaults. |
| Camera Raw Preferences | Preference | — | `Apply Auto Tone Adjustments` is a **Camera Raw** preference, not the Photoshop auto commands. |

There is **no Preferences pane** for the Photoshop auto corrections: the
persistent "preferences settings" are the auto-correction **defaults** saved from
the Auto Color Correction Options dialog (`Save as Defaults`), stored in the
application preferences store, not the document.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Algorithm | radio | Enhance Per Channel Contrast *(inferred)* | Mono Contrast / Per Channel Contrast / Find Dark & Light | Auto Tone/Contrast/Color map to these. |
| Snap Neutral Midtones | bool | off except Auto Color | on / off | Ignored by Auto Tone/Contrast. |
| Shadow Clip | percent | 0.1% stock; 0.1% (Auto Tone) / 0.5% (Auto Contrast/Color) per-command | 0.0%–1% recommended | Clipping of the dark end. |
| Highlight Clip | percent | as above | 0.0%–1% recommended | Clipping of the light end. |
| Shadow target color | color | black (0,0,0) *(inferred)* | Color Picker | Target for the darkest areas. |
| Midtone target color | color | RGB 128 gray | Color Picker | Target for neutral midtones. |
| Highlight target color | color | white (255,255,255) *(inferred)* | Color Picker | Target for the lightest areas. |
| Save as Defaults | button | — | — | Persists settings for the commands + Auto button. |

Note: the Help describes per-command clipping defaults (0.1% vs 0.5%) but the
Auto Color Correction Options dialog itself documents 0.1% as the stock default;
this apparent inconsistency is flagged in `## Open questions`.

## Algorithms & pipeline

The commands are histogram/percentile operations over the active layer or
selection. Adobe's exact maths is not published; the following matches the Help
and the best community analysis (Gerald Bakker, secondary).

Let `c` be the clip fraction. For **Enhance Monochromatic Contrast** (Auto
Contrast):

1. Find the darkest channel value across the composite image after discarding the
   lowest `c` of the histogram (and symmetrically the lightest).
2. Move the black endpoints of the R, G, B curves by the **same** amount so the
   composite darkest maps to the shadow target (default 0).
3. Similarly move the white endpoints to the highlight target (default 255).
   The composite (RGB) curve is left unchanged.

For **Enhance Per Channel Contrast** (Auto Tone): steps 1–3 above are applied
**independently per channel** — a per-channel histogram stretch — so white/black
balance (and therefore cast) can shift.

For **Find Dark & Light Colors** (Auto Color):

1. Sample/average the darkest pixels to a "dark color" and the lightest to a
   "light color".
2. Move the R/G/B curves so the dark average is neutralised toward the shadow
   target and the light average toward the highlight target.
3. If `Snap Neutral Midtones` is on, add midtone control points on the channel
   curves to force an average near-neutral color to the midtone target (128).

**Behavioural parity only, algorithm TBD** for the exact sampling count, the
"near-neutral" detection, and the gamma solve. Community analysis observed a
fourth algorithm ("Enhance Brightness and Contrast", content-aware, RGB-curve
only) in **later** Photoshop versions; the CS6 Help documents only the three
above and it is **not** treated as CS6 behaviour.

## Rust module mapping

Proposed:

- `pictura-core::adjust::auto` —
  `AutoAlgorithm { MonoContrast, PerChannelContrast, FindDarkLight }`.
- `AutoCorrectionOptions { algorithm, snap_neutral_midtones: bool, shadow_clip: f32, highlight_clip: f32, shadow_target: Color, mid_target: Color, highlight_target: Color }`.
- `fn auto_tone(buf, opts)`, `fn auto_contrast(buf, opts)`,
  `fn auto_color(buf, opts)` — thin wrappers that force the documented
  algorithm/snap combinations.
- `fn histogram(buf, selection) -> Histogram` (reused by Levels/Curves).
- `AutoDefaults` persisted in the application preferences store, not the document
  (`11-cross-cutting/preference-storage.md`).
- `AutoAdjustCommand` implements `EditCommand`.

## Qt6 component mapping

- `ImageAutoMenu` — three `QAction`s (`Auto Tone`, `Auto Contrast`,
  `Auto Color`).
- `AutoColorCorrectionDialog` (`QDialog`) — three algorithm radios, `Snap
  Neutral Midtones` checkbox, shadow/highlight `Clip` `QDoubleSpinBox`es, three
  `ColorSwatchButton`s, `Save as Defaults`.
- Reached from `LevelsPropertiesWidget` / `CurvesPropertiesWidget`
  panel menus and the Auto button's `Alt`/`Option` handler.
- A shared `HistogramView` renders the preview histogram.

## Data-model impact

- The three `Image > Auto …` commands are **destructive** single-undo edits on
  the active layer/selection; no node type.
- Current Auto Color Correction settings are application state, persisted via
  `AutoDefaults` in preferences on `Save as Defaults`; they are **not** stored in
  PSD/PSB.
- The Auto button in Levels/Curves is not a separate document entity: it writes
  the resulting control-point values into the adjustment layer's parameters
  (undo granularity = one step on the adjustment layer, `05-layers/adjustment-layers.md`).
- `Image > Auto …` undo records store a pre-image/tile diff because the mapping
  is lossy (`ARCH-009`).

## Edge cases

- **Selection active**: the command operates on the selection only (general
  adjustment rule).
- **Flat-color image**: Auto Contrast "does not improve" it; per-channel gains
  can be degenerate when a channel's range is zero — guard the stretch.
- **Already-clipped images**: with the stock 0.1%/0.5% clip, saturated images can
  suffer large color shifts (especially Auto Color with Snap Neutral Midtones).
- **CMYK / Lab / Grayscale**: Auto operations target per-channel black/white
  points; whether all three commands are enabled in every mode is unverified.
- **Bitmap / Indexed / Duotone / Multichannel**: adjustments restricted; disable
  or convert mode.
- **32-bit/channel**: Levels is supported in 32-bit, but whether the three
  `Image > Auto …` commands are enabled is unverified.
- **High bit depth (16-bit)**: histogram bins and clip percentiles must be
  computed at the document depth, not at 8-bit.
- **Color-managed documents**: auto corrections act on encoded values in the
  document space, not linear light (inferred) — this affects highlight/shadow
  behaviour.
- **Huge/PSB**: histogram pass streams by tile.
- **Fade/Undo**: the commands are not previewable (unlike the dialog the Auto
  button opens); they apply immediately and are single undo steps.

## Parity acceptance criteria

1. Given a synthetic RGB ramp with a known 0.5% black tail, Auto Contrast maps
   the composite black point to 0 and the white point to 255 within ±1 LSB and
   introduces no channel-dependent change (R, G, B get the same endpoint move).
2. Given a strong color cast, Auto Tone (per channel) changes the cast, while
   Auto Contrast does not change the neutral-gray axis beyond ±1 LSB.
3. Given a known near-neutral midtone cast, Auto Color with Snap Neutral
   Midtones on neutralises the midtone toward RGB 128; with it off, it does not.
4. Given a document with 0% clipping and pure black/white present, Auto
   Contrast is a near-identity within tolerance.
5. Given `Save as Defaults` with new clip percentages, a subsequent
   `Image > Auto Tone` / `Auto Contrast` uses the new percentages (verifiable by
   the resulting black/white point positions).
6. Given `Image > Auto …`, exactly one history state is added and Undo restores
   the original pixels bit-for-bit.
7. Given a flat single-color image, all three commands are no-ops or safe (no
   NaN/division-by-zero) and add one state.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, "Making quick tonal adjustments": Auto Contrast
  (0.5% default, no casts, flat-color note), Auto Color (RGB 128 midtone target,
  0.5% default, shadows/midtones/highlights search), "Set Auto adjustment
  options" (three algorithms, Snap Neutral Midtones, 0.0%–1% clip guidance,
  0.1% stock default, target swatches, Save as Defaults, algorithm-choice
  nuance), "Adjust black and white points with the Auto option" (0.1% default,
  per-channel behaviour); "Color adjustment commands" and Properties-panel menu
  placement.
- `https://geraldbakker.nl/psnumbers/auto-options.html` — detailed community
  public analysis of the three (and a later fourth) Auto algorithms, the
  endpoint-move model, and the Auto Tone/Contrast/Color → algorithm mapping.
  Secondary/community source.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD color-mode enumeration (context).
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  "Camera Raw preferences" cross-reference establishing `Apply Auto Tone
  Adjustments` as an ACR setting, distinct from these commands.

## Open questions

- **Clipping-default inconsistency.** The Help gives 0.1% as the stock default
  and also 0.5% for Auto Contrast/Auto Color; which value a fresh CS6 install
  actually uses per command? *Resolves with:* a fresh-install CS6 observation.
- **Does the algorithm radio matter for the commands?** The Help says it does
  not (except Snap Neutral Midtones), but community testing suggests small
  differences. *Resolves with:* CS6 controlled tests.
- **Default algorithm selection** in a fresh Auto Color Correction Options
  dialog. *Resolves with:* CS6 screenshot.
- **Target-color defaults** (shadow = black, highlight = white) are inferred.
  *Resolves with:* CS6 dialog capture.
- **Availability in 32-bit / CMYK / Lab / Grayscale** and in Indexed/Duotone.
  *Resolves with:* mode-by-mode CS6 inspection.
- **Exact "near-neutral" search** used by Snap Neutral Midtones and Find Dark &
  Light Colors (sample count, space). *Resolves with:* deconvolution tests or an
  Adobe statement.
- **16-bit histogram precision** and rounding. *Resolves with:* CS6 16-bit tests.
