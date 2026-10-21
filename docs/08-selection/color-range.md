# Select > Color Range

- **Spec ID:** `SEL-005`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds **Skin Tones** selection and **Detect Faces** to the Color Range dialog; Detect Faces is the CS6-only option that makes skin-tone selection more accurate. Saving Skin Tones settings as a preset is marked **Creative Cloud only** in the fetched corpus. Adjustable Shadows/Midtones/Highlights ranges arrive in CC, not CS6. The command itself, Sampled Colors, Fuzziness, Localized Color Clusters, and the selection previews are CS4/CS5-era.
- **Depends on:** `SEL-001` selection-model, `SEL-002` selection-tools-overview, `SEL-003` refine-edge, `TOOL-004` quick-selection/magic-wand, `TOOL-044` quick-mask-tool, `04-image-ops/color-profiles-and-assignment.md` (`ARCH-007` color-management), `05-layers/layer-masks.md` (`LAY-004`), `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `10-workflow-io/color-settings.md`.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help corpus;
> the full Select-menu range list and slider ranges are community-reported and
> marked.

## CS6 behavior

`Select > Color Range` selects a specified color or color range, either within the existing selection or across the whole image. CS6 rules:

- **Replace vs refine:** to replace a selection, deselect everything before running the command. To refine, run the command again on the current selection to narrow it to a subset of colors (e.g. select Cyans in a cyan selection, then Greens).
- **32-bpc:** the command is unavailable for 32-bits-per-channel images.
- **It also refines layer masks:** Color Range can refine a layer mask; invoke it from the Masks section of the CS6 Properties panel (choose **Sampled Colors** there; `LAY-004`).

Dialog workflow (CS6 Help):

1. **Select menu** (what to match):
   - **(CS6) Skin Tones** — selects colors that resemble common skin tones. **Detect Faces** makes that selection more accurate.
   - **Sampled Colors** — enable the Eyedropper tool and click sample colors in the image; when sampling several color ranges, turn on **Localized Color Clusters** for a more accurate result.
   - **A color or tonal range** — choosing a preset means the selection cannot be adjusted afterwards.
   - The CS6 Help doesn't enumerate the preset list in the fetched text; the CS6-era/community list is **Reds**, **Yellows**, **Greens**, **Cyans**, **Blues**, **Magentas**, **Highlights**, **Midtones**, **Shadows**, and **Out Of Gamut**.
2. **Display option** — **Selection** previews the mask as white for selected pixels, black for unselected, gray for partially selected; **Image** previews the whole image. `Ctrl`/`Cmd` toggles between them.
3. **Sampling** — position the Eyedropper over the image/preview and click. The **plus** eyedropper adds colors, the **minus** eyedropper removes them; `Shift` temporarily activates plus, `Alt`/`Option` minus.
4. **Fuzziness** — sets how wide a color range the selection covers and how many partially selected pixels it produces (the gray areas in the selection preview). A low value narrows the color range, a high value widens it.
5. **Localized Color Clusters** — when on, the **Range** slider sets how near a pixel's color must be to the sampled points to be included. This separates similar colors that are spatially far apart (e.g. foreground flowers vs background flowers).
6. **Selection Preview** (in the image window) — **None** (original image), **Grayscale** (white = fully selected, gray = partially selected, black = unselected), **Black Matte** (selected pixels keep the original image, unselected pixels are black; suits bright images), **White Matte** (selected pixels keep the original image, unselected pixels are white; suits dark images), **Quick Mask** (unselected areas shown as a rubylith overlay in the Quick Mask color; `SEL-004`).
7. **Reset** — `Alt`/`Option`-click Reset to revert to the original selection.
8. **Save / Load** the Color Range settings; **(Creative Cloud only)** Skin Tones settings can be saved as a preset, and **Detect Faces** can be saved when Skin Tones or Sampled Colors is selected.

Warning behavior: when the message "No pixels are more than 50% selected" appears, the selection border is invisible — for example a Select-menu preset such as Reds was chosen on an image with no sufficiently saturated red hues.

CS6 limitation: selecting Shadows/Midtones/Highlights in CS6 yields fixed, non-adjustable tonal ranges; CC later adds an adjustable range plus Fuzziness. (Community source; the fetched corpus does not describe the CS6 tonal behavior.)

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Select > Color Range…` | Menu | — | Opens dialog; `Alt+S` then `C` in some locales (community) |
| `Select > Color Range` | Menu | `Ctrl`/`Cmd` (in dialog) | Toggle Image / Selection preview |
| Properties panel → Masks section | Button | — | "Color Range" refines a layer mask (`LAY-004`) |
| Dialog | Combo | — | Select: Sampled Colors / Skin Tones / ranges |
| Dialog | Radio | — | Selection vs Image preview |
| Dialog | Checkbox | — | Localized Color Clusters |
| Dialog | Slider | — | Fuzziness |
| Dialog | Slider | — | Range (enabled with Localized Color Clusters) |
| Dialog | Checkbox | — | Invert (below the eyedropper samplers) |
| Dialog | Checkbox | — | Detect Faces (Skin Tones; community: with Localized Color Clusters) |
| Dialog | Buttons | `Shift` / `Alt` | Plus / minus eyedropper (temporary) |
| Dialog | Combo | — | Selection Preview: None / Grayscale / Black Matte / White Matte / Quick Mask |
| Dialog | Buttons | — | Save / Load settings; Reset (Alt-click) |
| Dialog | Button | — | OK / Cancel |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Select | enum | Sampled Colors | Sampled Colors / Skin Tones / Reds / Yellows / Greens / Cyans / Blues / Magentas / Highlights / Midtones / Shadows / Out Of Gamut | Preset list community-reported |
| Preview | enum | Selection | Selection / Image | `Ctrl`/`Cmd` toggles |
| Fuzziness | int | 40 *(community)* | 0–200 *(community)* | Wider = more partial coverage |
| Localized Color Clusters | bool | off | on / off | Enables Range |
| Range | int | 100 *(community)* | 0–100 *(community)* | Spatial selectivity; lower = nearer samples only |
| Invert | bool | off | on / off | Below the eyedropper samplers |
| Detect Faces | bool | off | on / off | CS6 skin-tone option |
| Selection Preview | enum | None | None / Grayscale / Black Matte / White Matte / Quick Mask | Image-window preview |
| Sampled colors | set | — | any number of clicks | Plus/minus eyedroppers |
| Save/Load settings | file | — | `.axt`-style setting files *(community)* | Reuse presets |

Defaults and the 0–200 / 0–100 ranges are community-reported; the fetched CS6 Help describes the controls qualitatively without numeric bounds except the dialog's own slider extents (see Sources).

## Algorithms & pipeline

### Color-distance mask (Sampled Colors)

For each pixel the working-space color is compared to the sampled color set; coverage follows a distance-to-threshold curve controlled by Fuzziness.

```text
sample_set S = {c1..cn}
for each pixel p:
    d = min over s in S of distance(color(p), s)      # e.g. Euclidean in RGB or Lab
    coverage(p) = clamp01((fuzziness - d) / fuzziness) # 0..255 quantized
```

- The exact color space and distance metric (RGB Euclidean vs perceptual/Lab) are unspecified — **behavioral parity only, algorithm TBD**.
- Fuzziness widens the band and increases partial (gray) pixels; low Fuzziness produces hard edges (CS6 Help).
- Plus/minus eyedropper clicks add/remove samples, rescaling coverage.

### Localized Color Clusters (spatial term)

The Range slider sets how near a pixel's color must be to the sample points to be included. A standard model adds a spatial weight:

```text
for each pixel p:
    spatial(p) = proximity(p, S)      # function of distance to nearest sample location
    coverage(p) = color_term(p) * spatial_term(p, Range)
```

Lowering Range rejects matching colors far from the samples. The exact spatial falloff and the combination with the color term are closed — **behavioral parity only**.

### Preset color and tonal ranges

- **Color ranges** (Reds…Magentas): hue-window selection in the working space; exact hue centers and widths unspecified.
- **Tonal ranges** (Highlights/Midtones/Shadows): luminance-band selection; CS6 uses fixed bands (not adjustable), per the community source.
- **Out Of Gamut:** marks pixels not reproducible in the current CMYK/proof gamut, i.e. where the RGB→CMYK round trip exceeds a tolerance. CS6 Help does not document the algorithm; implement against `ARCH-007` color management. Selection works on RGB documents; CMYK documents do not offer the option (community/adobe community reports).
- **Skin Tones / Detect Faces:** skin-tone pixels are clustered in a chroma space; Detect Faces restricts/boosts selection to detected face regions. CS6 Help documents the option but not the algorithm — **behavioral parity only, algorithm TBD**.

### Preview and output

Previews are display-only renderings (grayscale, mattes, Quick Mask overlay) of the candidate coverage. On OK, the resulting coverage is combined with the existing selection by the dialog's operation semantics (replace when nothing selected; refine a subset when re-applied) and committed once. `Invert` yields `255 − coverage`.

## Rust module mapping

- `pictura_selection::color_range::ColorRangeSettings` — `{ select: ColorRangeSelect, fuzziness: u8, localized: bool, range: u8, invert: bool, detect_faces: bool, preview }`.
- `pictura_selection::color_range::ColorRangeSelect` — `{ Sampled(Vec<Color>), SkinTones, Red, Yellow, Green, Cyan, Blue, Magenta, Highlights, Midtones, Shadows, OutOfGamut }`.
- `pictura_selection::color_range::evaluate(&Image, &ColorRangeSettings) -> Mask` — color-distance + spatial term; tiled.
- `pictura_selection::color_range::tonal_band(&Image, band) -> Mask` — luminance-band selection.
- `pictura_selection::color_range::out_of_gamut(&Image, &TargetProfile) -> Mask` — via `pictura_color`/lcms2 (`ARCH-007`).
- `pictura_selection::color_range::skin::detect(&Image) -> Mask` — skin-tone cluster + face detection regions; heuristic.
- `pictura_selection::color_range::presets` — save/load settings files.
- Shared: `Mask` tiles, `SelectionOp`, `Color`.

Boundary types: `ColorRangeSettings`, `Color`, `Mask` tiles, and preview frames. Face detection is a separate concern that returns region masks the color term can be restricted to.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ColorRangeDialog` | `QDialog` | The full dialog; live preview |
| `SelectCombo` | `QComboBox` | Select-mode list; disables sliders for preset ranges |
| `PreviewToggle` | `QRadioButton`/`QButtonGroup` | Selection vs Image; `Ctrl`/`Cmd` shortcut |
| `EyedropperButtons` | `QToolButton` | Plus/minus pickers; `Shift`/`Alt` temporary activation |
| `FuzzinessSlider` | `QSlider` + spin | 0–200 |
| `LocalizedGroup` | `QCheckBox` + `RangeSlider` | Clusters + Range; Range disabled when unchecked |
| `InvertCheck` | `QCheckBox` | Below the samplers |
| `DetectFacesCheck` | `QCheckBox` | Visible for Skin Tones (community: with Localized Color Clusters) |
| `SelectionPreviewCombo` | `QComboBox` | None / Grayscale / Black Matte / White Matte / Quick Mask |
| `ColorRangePreview` | custom widget | Renders the chosen preview mode over the canvas (`ARCH-006`) |
| `SaveLoadButtons` | `QPushButton` | Persist/load settings; preset file dialog |

Widgets over QML: it is a dense modal dialog with a live canvas preview and eyedropper capture; a `QDialog` gives predictable focus, shortcut, and cursor handling, and the preview is a single GPU-backed widget.

## Data-model impact

- Produces a **selection** (undo: `SelectionChange`, `SEL-001`) or, when invoked from the Properties panel, edits a **layer mask** (`LAY-004`) — one undo state per OK.
- No new document fields. Sampled colors live only for the dialog session; Save/Load writes a settings file, not document data.
- **Invert**, when present, is applied to the produced coverage, not stored.
- **Detect Faces** may use a transient face-region cache, keyed by document revision, never serialized.
- Color Range settings are **session/preference state**; the CC-only skin-tone preset is out of CS6 scope (may be implemented as an extension, but must not be claimed as CS6 parity).
- Out-of-gamut selection depends on the current proof/profile (`ARCH-007`, `10-workflow-io/color-settings.md`) but does not modify it.

## Edge cases

- **32-bpc:** the command is disabled/absent. Do not partially run it.
- **Bitmap/Grayscale/Indexed:** color-range matching is meaningless or restricted; disable for Bitmap and Indexed, and define Grayscale behavior (tonal ranges only) — CS6 Help does not spell this out.
- **CMYK/Lab:** sample in the working space; `Out Of Gamut` is offered on RGB documents against a CMYK proof/profile (community/adobe community report that it is absent on CMYK documents). Confirm exact rule.
- **No matching colors:** emit the CS6 "No pixels are more than 50% selected" outcome — invisible ants, but an existing selection — rather than an error.
- **Refining an existing selection:** re-applying Color Range selects a subset of what is currently selected; the dialog operates within the existing selection, not the whole image, when one is active.
- **Localized Color Clusters off:** Range is disabled; ensure the setting is remembered per session.
- **Huge/PSB:** evaluate in tiles; a global out-of-gamut conversion is expensive — cache the proof conversion.
- **GPU unavailable:** preview falls back to CPU compositing; mask evaluation is CPU.
- **Face detection on a small/1-px image:** no faces; Skin Tones still works as a plain chroma match.
- **Cancel:** restores the pre-dialog selection/mask exactly; no history entry.
- **Undo:** one state per OK, even though several slider moves happened; live previews are not history states.

## Parity acceptance criteria

- Given a solid-red patch on white, `Sampled Colors` clicked on the red with low Fuzziness selects only the patch; raising Fuzziness includes more of the red range and produces partial (gray) coverage.
- Given multiple sampled colors (plus eyedropper), the selection includes all sampled clusters; the minus eyedropper removes a cluster.
- Given `Localized Color Clusters` on and a small `Range`, a same-colored region far from the samples is excluded; increasing `Range` includes it.
- Given `Selection` preview, selected pixels are white, unselected black, partial gray; given `Grayscale` the image window matches; `Black/White Matte` and `Quick Mask` previews match their CS6 descriptions.
- Given `Invert`, the produced selection equals `255 − coverage`.
- Given `Out Of Gamut` on an RGB image with a CMYK proof, pixels that fail the CMYK round trip are selected; on a CMYK document the option is absent (confirm).
- Given Skin Tones in CS6 with Detect Faces off, skin-like pixels are selected; enabling Detect Faces restricts the result toward detected faces.
- Given `Ctrl`/`Cmd` in the dialog, the preview toggles between Image and Selection.
- Given `Alt`/`Option`-click Reset, the dialog returns to the pre-open selection state.
- Given Save then Load settings, the re-opened settings reproduce the same selection on the same image.
- Given a 32-bpc document, Color Range is unavailable.
- Given no matching colors, OK still succeeds with an invisible but existing selection.
- Given Cancel, the document selection/mask is byte-identical to the pre-open state.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: Color Range's purpose and replace/refine rules; unavailability on 32-bpc; refining a layer mask from the CS6 Properties panel; the Select-menu choices behavior (Skin Tones CS6-only, Detect Faces, Sampled Colors, Localized Color Clusters, preset ranges being non-adjustable); Selection vs Image preview with `Ctrl`/`Cmd` toggle; plus/minus eyedroppers and `Shift`/`Alt` temp activation; Fuzziness behavior; Localized Color Clusters + Range description with the foreground/background flowers example; the Selection Preview list (None/Grayscale/Black Matte/White Matte/Quick Mask); Alt-click Reset; Save/Load settings; Creative-Cloud-only Skin Tones preset saving and Detect Faces saving; the "No pixels are more than 50% selected" note; and the CS6 new-feature bullet ("Face detection enhancements in Color Range", "Skin tone selection and face detection").
- `https://creativepro.com/photoshop-how-to-selecting-with-color-range` — CS6-era (Dec 2013) professional article. Established: CS6 has Skin Tones and Detect Faces but CS6 tonal selections (Shadows/Midtones/Highlights) are **not** adjustable (that control arrives in CC); Detect Faces is described as becoming available/used with Localized Color Clusters; CS6 tonal bands are auto-created.
- SearXNG result snippets (community, corroborating, not all individually fetched): the full Select-menu list (Reds/Yellows/Greens/Cyans/Blues/Magentas/Highlights/Midtones/Shadows/Skin Tones/Out Of Gamut), Fuzziness slider reaching 200 (Highlander), Range slider usage ~20–30 (Tourbox), and multiple Adobe-community threads confirming `Out Of Gamut` on RGB and its absence on CMYK.

Not used in this pass:

- `https://manualzz.com/doc/o/mw0x0/adobe-photoshop-cs6-user-manual-selecting-a-color-range-in-an-image` — a mirror of the same CS6 Help text, surfaced as a search hit; the identical text was available directly in the fetched PDF.
- `helpx.adobe.com` Color Range pages — HTTP 403.
- `html.duckduckgo.com` / `search.brave.com` result HTML — JS/anti-bot shells.

## Open questions

- **Exact Fuzziness range and default.** 0–200 with a default around 40 is community-reported; not in the CS6 text. Resolve with a CS6 UI capture.
- **Range range/default.** 0–100 is community-reported; not in the CS6 text.
- **Color distance metric and space.** Whether matching is RGB Euclidean, Lab, or a perceptual metric is closed. Resolve only if pixel comparison requires it.
- **Localized Color Clusters algorithm.** The spatial falloff and its combination with the color term are closed. Resolve with a sample-point/range sweep against CS6.
- **Preset range hue centers/widths.** The exact Reds…Magentas windows and the Highlights/Midtones/Shadows luminance bands are unspecified. Resolve with color-ramp comparisons.
- **Out Of Gamut rule.** Which profile/gamut is used, the round-trip threshold, and the exact document-mode availability (RGB only vs with proof on) are not fully documented. Resolve in `10-workflow-io/color-settings.md` / `ARCH-007` with a CS6 test.
- **Detect Faces availability condition.** The CS6 Help implies it is available with Skin Tones; a professional source ties it to Localized Color Clusters. Resolve with a CS6 capture.
- **Skin-tone preset file format.** The Save/Load settings file format (extension, contents) is not documented. Resolve by inspecting a CS6-saved file.
- **Behavior on Grayscale/Indexed/Bitmap.** Undocumented. Resolve with CS6 tests.
