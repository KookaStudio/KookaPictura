# Replace Color

- **Spec ID:** `ADJ-032`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Replace Color dialog is carried forward from CS5 unchanged in the CS6 Help (it already had `Localized Color Clusters`).
- **Depends on:** `04-image-ops/adjustments-overview.md`, `04-image-ops/adjustments/hue-saturation.md`, `04-image-ops/image-modes.md`, `08-selection/color-range.md`, `03-tools/color-replacement.md` (`TOOL-021`), `07-color-painting/color-picker.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`).

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

`Image > Adjustments > Replace Color` "combines tools for selecting a color
range with HSL sliders for replacing that color." Source: CS6 reference, "Replace
the color of objects in an image" (section "Use the Replace Color dialog box").

- The dialog has **two preview modes**:
  - `Selection` — shows the mask: masked areas black, unmasked white, partially
    masked areas varying gray by opacity.
  - `Image` — shows the image itself.
- `Localized Color Clusters` (optional) builds a more accurate mask for
  selecting similar, **contiguous** colors.
- **Eyedropper sampling**: click in the image or preview to select the color(s)
  to replace. `Shift`-click or the **Add To Sample** eyedropper adds areas;
  `Alt`/`Option`-click or the **Subtract From Sample** eyedropper removes them.
  The `Selection Color` swatch opens the Color Picker to target a color; the
  preview mask updates live.
- `Fuzziness` controls how related colors are included in the selection.
- **Replacement** by either the `Hue`, `Saturation`, `Lightness` sliders (or
  their text boxes) or by double-clicking the `Result` swatch and choosing a
  color in the Color Picker.
- **Important constraint** (Help verbatim): "You cannot replace pure gray, black,
  or white with a color. However, you can change the Lightness setting. (The Hue
  and Saturation settings are relative to existing color, so they have no
  effect.)"
- `Save` stores settings for reuse on other images.
- Replace Color **lacks the Colorize option** of the Hue/Saturation adjustment;
  the Help recommends the adjustment-layer technique for object-specific changes
  and positions Replace Color for **global** changes, including out-of-gamut
  colors for printing.

Replace Color is a destructive `Image > Adjustments` command; it has no
adjustment-layer icon. It is contrasted with the Color Replacement **tool**
(`TOOL-021`) and with a Hue/Saturation adjustment.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Replace Color` | Menu command + modal dialog | none documented | Destructive; no adjustment-layer icon. |
| Dialog > preview | Toggle | — | `Selection` (mask) / `Image`. |
| Dialog | Checkbox | — | `Localized Color Clusters`. |
| Dialog | Eyedropper | click | Select the target color in image/preview. |
| Dialog | Eyedropper +/- | `Shift`-click / `Alt`-click | Add / Subtract sample; explicit Add/Subtract buttons too. |
| Dialog | Color swatch | — | `Selection Color` (target) opens the Color Picker. |
| Dialog | Slider | — | `Fuzziness`. |
| Dialog | Sliders | — | `Hue`, `Saturation`, `Lightness` (replacement). |
| Dialog | Color swatch | — | `Result` (replacement) opens the Color Picker. |
| Dialog | Buttons | — | `Save` (and implied `Load`) settings. |
| Dialog | Checkbox | — | `Preview` *(inferred; not named in the fetched steps)*. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Preview mode | radio | `Image` *(inferred)* | `Selection` / `Image` | Mask view vs image view. |
| Localized Color Clusters | bool | off *(inferred)* | on / off | Restricts the mask to contiguous clusters. |
| Selection Color (target) | color | current sample | Color Picker | The color to be replaced. |
| Samples | list | one sample on open | unlimited | Add/Subtract via eyedroppers. |
| Fuzziness | slider | 0 *(inferred)* | 0–200 *(inferred)* | "degree to which related colors are included". |
| Hue | slider | 0 | −180…+180 *(inferred)* | Relative to existing color. |
| Saturation | slider | 0 | −100…+100 *(inferred)* | Relative; no effect on neutral pixels. |
| Lightness | slider | 0 | −100…+100 *(inferred)* | Only replacement control that affects gray/black/white. |
| Result (replacement) | color | sampled color | Color Picker | Alternative to the HSL sliders. |
| Save / Load | buttons | — | settings file *(format inferred)* | Reuse across images. |

Defaults and numeric clamps are not stated in the fetched Help for Fuzziness and
the HSL sliders; they are *(inferred)* from the general Hue/Saturation ranges and
listed in `## Open questions`.

## Algorithms & pipeline

The Help gives the interaction contract but not the math. Behavioural model:

1. **Mask computation.** Build a mask of pixels close to the sampled
   target color(s). The Help's `Fuzziness` is the tolerance; the classic
   implementation threshold is a **3D distance in RGB/HSV** between each pixel
   and the sampled color, with `Localized Color Clusters` adding a spatial
   connectivity constraint (similar to Color Range / magic-wand clustering).
2. **Replacement.** With the mask `m ∈ [0,1]`, shift each masked pixel's HSL:
   `H' = H + hue`, `S' = S + saturation`, `L' = L + lightness`, then convert
   back. Because H and S are relative, neutral pixels (`S = 0`) are unaffected
   by them — exactly the Help's "pure gray/black/white" note; only `Lightness`
   changes them.
3. **Application.** Blend original and replacement by the mask; where
   `Localized Color Clusters` is on, isolated matching pixels are excluded.
4. `Save` serialises the samples, fuzziness, clusters flag, and HSL values.

Adobe's exact color-distance metric (with or without gamma/working-space
handling) is unpublished: **behavioural parity only, algorithm TBD**.

## Rust module mapping

Proposed:

- `pictura-core::adjust::replace_color` —
  `ReplaceColorSettings { samples: Vec<Color>, localized_clusters: bool, fuzziness: u8, hue: i32, saturation: i32, lightness: i32, result: Option<Color> }`.
- `fn color_sample_mask(pixels: &PixelBuffer, samples: &[Color], fuzziness: u8, localized: bool) -> Mask`
  — reuse the Color Range masker (`08-selection/color-range.md`).
- `fn replace_color(buffer: &mut PixelBuffer, mask: &Mask, settings: &ReplaceColorSettings)`.
- `ReplaceColorCommand` implements `EditCommand`.
- Boundary types: `Color`, `Mask`, `PixelBuffer`, `Selection`.

## Qt6 component mapping

- `ReplaceColorDialog` (`QDialog`) — preview `QWidget` with a mask/image toggle,
  an eyedropper tool group (select/add/subtract) wired to the canvas,
  `Localized Color Clusters` checkbox, `Fuzziness` slider, the three HSL
  sliders, `Selection Color` and `Result` swatches (`ColorSwatchButton` opening
  the shared color picker), and `Save`/`Load` buttons.
- A reusable `ColorRangeMaskView` shared with `08-selection/color-range.md` and
  `07-color-painting/color-picker.md`.
- Live preview recomputes the mask incrementally on the GPU/worker pool.

## Data-model impact

- Destructive `Image > Adjustments` command; no adjustment-layer type and no PSD
  node.
- Undo: one history state; store a pre-image diff of the affected region.
- `Save` writes a settings file (independent of the document); its format is not
  published by Adobe (`## Open questions`).
- If a selection is active, the command modifies only the selected pixels
  (general adjustment rule).

## Edge cases

- **Neutral pixels**: Hue/Saturation have no effect; only Lightness can change
  gray/black/white, per the Help's explicit note.
- **Non-RGB modes**: the dialog operates on HSL; exact availability in
  CMYK/Lab/Grayscale is unverified *(inferred: allowed in color modes that have
  chroma)*.
- **Bitmap/Indexed/Duotone** (and Multichannel): adjustments are restricted;
  disable or convert mode first.
- **32-bit/channel**: not in the supported-adjustment list; disable *(inferred)*.
- **Fuzziness = 0**: only the exact sampled color (within rounding) is masked.
- **Localized clusters on**: thin/disconnected matching regions are dropped.
- **Multiple samples**: mask is the union of per-sample matches.
- **Selection + mask**: intersect the selection with the color mask.
- **Empty document / 1-px**: trivial; no special casing.
- **Huge/PSB**: stream per tile; no whole-canvas float copy.

## Parity acceptance criteria

1. Given a flat region of color C and adjacent colors, increasing `Fuzziness`
   monotonically increases the masked area (mask is a superset at larger
   Fuzziness).
2. Given a target sample and a replacement HSL shift, only pixels passing the
   Fuzziness distance test change; unmasked pixels are byte-identical.
3. Given a neutral pixel (`R=G=B`) inside the mask and a Hue/Saturation-only
   replacement, the pixel is unchanged; with a non-zero Lightness it changes
   accordingly.
4. Given `Localized Color Clusters` on, an isolated matching pixel separated from
   the main cluster is excluded from the mask.
5. Given `Shift`-click a second color and `Alt`-click the first, the mask equals
   mask(second) minus mask(first).
6. `Save` then `Load` in a fresh session reproduces the same mask and replacement
   within the tolerance of the testing strategy.
7. The `Selection` preview renders masked pixels black, unmasked white, and
   partial mask as gray, matching the mask alpha.
8. The command adds exactly one history state and is reversible by Undo.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, "Replace the color of objects in an image" → "Use the
  Replace Color dialog box": selection/image preview modes, Localized Color
  Clusters, eyedropper add/subtract and modifier keys, Selection Color swatch,
  Fuzziness, Hue/Saturation/Lightness replacement and Result swatch, the
  gray/black/white limitation, Save, the missing Colorize option, and the
  positioning versus Hue/Saturation and the Color Replacement tool; also
  "Replacing colors" and "Color adjustment commands".
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD color-mode enumeration (context for mode gating).

## Open questions

- **Color-distance metric.** Is the mask an RGB Euclidean distance, an HSV
  distance, or a color-managed/Lab distance? *Resolves with:* a CS6 mask test on
  a controlled color chart.
- **Fuzziness range/default.** The fetched Help gives no numeric bounds.
  *Resolves with:* a CS6 dialog capture.
- **HSL slider ranges/defaults.** *Resolves with:* CS6 UI capture.
- **Preview default mode** (`Selection` vs `Image`) and `Preview` checkbox
  presence. *Resolves with:* CS6 default screenshot.
- **Availability in CMYK/Lab/Grayscale** and in 32-bit. *Resolves with:* CS6
  mode-by-mode inspection.
- **`Save` settings file format.** *Resolves with:* inspecting a CS6-saved file.
- **Exact `Localized Color Clusters` connectivity rule** (radius, neighbour
  count). *Resolves with:* controlled mask tests on CS6.
