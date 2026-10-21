# Match Color

- **Spec ID:** `ADJ-031`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Match Color command and its dialog are carried forward from CS5 unchanged in the CS6 Help.
- **Depends on:** `04-image-ops/adjustments-overview.md`, `04-image-ops/adjustments/hue-saturation.md`, `04-image-ops/adjustments/levels.md`, `04-image-ops/image-modes.md`, `08-selection/selection-model.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/color-management.md` (`ARCH-007`), `01-architecture/undo-history.md` (`ARCH-009`).

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

`Image > Adjustments > Match Color` matches colors between **multiple images,
multiple layers, or multiple selections**, and can also adjust an image's
luminance, its color range, and neutralise a color cast. Source: CS6 reference,
"Match the color in different images".

- **RGB only**: "The Match Color command works only in RGB mode."
- While the dialog is open the pointer becomes the **Eyedropper**; the Info panel
  shows live color values as the user adjusts.
- The command matches one image (the **source**) to another (the **target**).
  Typical use: make colors consistent across photos, or match skin tones.
- It can also match **layers within the same image**.

Workflow documented by the Help:

1. (Optional) Make a selection in the source and/or target image. With no
   selection, the command matches **overall image statistics**.
2. Activate the image/layer to change and choose `Image > Adjustments > Match
   Color`. If applying to a specific layer, that layer must be active.
3. In the **Image Statistics** area's `Source` menu choose the source image
   (`None` makes target and source the same), and the `Layer` menu selects the
   source layer or `Merged`.
4. Selection handling options:
   - `Ignore Selection When Applying Adjustment` (Destination Image area) —
     applies to the whole target ignoring its selection.
   - `Use Selection In Source To Calculate Colors` — use the source selection
     for statistics.
   - `Use Selection In Target To Calculate Adjustment` — use the target
     selection for statistics.
5. `Neutralize` removes a color cast in the target automatically.
6. `Luminance` adjusts brightness (min 1, max 200, default 100).
7. `Color Intensity` adjusts saturation (min 1 = grayscale, max 200, default
   100).
8. `Fade` controls how much of the adjustment is applied; moving right
   **reduces** the adjustment.
9. `Preview` shows the change live.
10. `Save Statistics` / `Load Statistics` store and reapply the saved statistics
    in the Image Statistics area.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Match Color` | Menu command + modal dialog | none documented | Destructive to the active layer/document; no adjustment-layer icon. |
| Dialog > Image Statistics area | Group | — | `Source` combo, `Layer` combo, `Use Selection In Source…`, `Use Selection In Target…`, `Save Statistics`, `Load Statistics`. |
| Dialog > Destination Image area | Group | — | `Ignore Selection When Applying Adjustment`. |
| Dialog > Image Options area | Group | — | `Luminance`, `Color Intensity`, `Fade`, `Neutralize`. |
| Dialog | Checkbox | — | `Preview` (on by default per the Help's instruction to keep it selected). |
| Canvas | Cursor | — | Pointer becomes the Eyedropper; Info panel shows values. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Source | combo | `None` (target itself) | `None` + open documents | Source image for statistics. |
| Layer | combo | `Merged` *(inferred)* | any source layer / `Merged` | Enabled when a real source is chosen. |
| Use Selection In Source To Calculate Colors | bool | off *(inferred)* | on / off | Uses the source selection's colors. |
| Use Selection In Target To Calculate Adjustment | bool | off *(inferred)* | on / off | Uses the target selection's colors. |
| Ignore Selection When Applying Adjustment | bool | off *(inferred)* | on / off | Applies to the entire target. |
| Neutralize | bool | off *(inferred)* | on / off | Auto-removes a color cast. |
| Luminance | slider + field | 100 | 1–200 (Help-stated) | Brightness of the target. |
| Color Intensity | slider + field | 100 | 1–200 (Help-stated); 1 = grayscale | Saturation of the target. |
| Fade | slider + field | 0 *(resolved from the Help direction by the `add-match-color` change)* | 0–100 | Moving right reduces the adjustment; 100 withholds it entirely (identity). |
| Preview | bool | on | on / off | Live preview. |
| Save / Load Statistics | buttons | — | `.sta` binary payload *(in-tree: `PCTMCST` magic, version byte, six little-endian f64s; Adobe's own format is undocumented)* | Stores the computed statistics only. |

The Help gives exact bounds only for Luminance and Color Intensity (1–200).
The Fade default and range are resolved by the `add-match-color` change in
favour of the Help-sourced direction sentence (0 = full match, 100 =
identity); the checkbox defaults remain *(inferred)* and listed in
`## Open questions`.

## Algorithms & pipeline

Adobe's exact algorithm is closed. The command is universally described as a
**statistics-transfer** operation: compute per-channel (typically in a
perceptually-uniform space such as CIEL\*a\*b\* or the Ruderman lαβ space) means
and standard deviations of the source and target, then remap the target so its
statistics match the source:

```
out = (in - mean_target) * (std_source / std_target) + mean_source
```

This is the operation popularised by Reinhard et al., *Color Transfer between
Images* (2001); Adobe's implementation and its color space are not published.
**Behavioural parity only, algorithm TBD.**

Documented / observable modifiers:

- `Neutralize` biases the result so the target's near-neutral colors become
  neutral (a gray-world-style correction).
- `Luminance` scales the luminance component of the transfer.
- `Color Intensity` scales the chroma component; at 1 the output is grayscale.
- `Fade` interpolates between the original target and the fully matched result.

## Rust module mapping

Proposed:

- `pictura-core::adjust::match_color` — `MatchColorSettings { source: Option<SourceRef>, use_source_selection: bool, use_target_selection: bool, ignore_selection: bool, neutralize: bool, luminance: f32, color_intensity: f32, fade: f32 }`.
- `SourceRef { document: DocumentId, layer: LayerRef | Merged }`.
- `ImageStatistics { space: ColorSpace, per_channel_mean: [f32; N], per_channel_std: [f32; N] }` —
  computed over the document/selection; serialisable for `Save/Load Statistics`.
- `fn compute_statistics(doc, layer, selection) -> ImageStatistics`;
  `fn match_color(target: &mut PixelBuffer, stats: &ImageStatistics, settings: &MatchColorSettings)`.
- `MatchColorCommand` implements `EditCommand`.

## Qt6 component mapping

- `MatchColorDialog` (`QDialog`) — `Source`/`Layer` `QComboBox`es populated from
  the open-document registry and the source layer tree; three selection
  checkboxes; `Neutralize` checkbox; `Luminance`, `Color Intensity`, `Fade`
  sliders with spin boxes; `Preview`; `Save Statistics…` / `Load Statistics…`
  `QFileDialog` hooks.
- A reusable `DocumentLayerChooser` widget shared with other cross-document
  dialogs.
- Live preview runs the core statistics match off the UI thread and streams a
  downsampled preview.

## Data-model impact

- Destructive `Image > Adjustments` command; **no** adjustment-layer type and no
  PSD node. (CS6 has no Match Color adjustment layer.)
- Undo: one history state. Because the transform is stateless given the saved
  statistics, the record may store the `ImageStatistics` plus settings and the
  pre-image for exact reversal; the lazy option is a pre-image of the edited
  region (`ARCH-009`).
- `Save Statistics` writes a statistics file independent of the document; the
  `.sta`/settings format is not published by Adobe (`## Open questions`), so
  this implementation stores an in-tree binary payload (`PCTMCST` magic,
  version byte, six little-endian f64s).
- When source and target have different document profiles, statistics should be
  computed in a common space; whether CS6 uses each document's space or the
  Color Settings working space is unverified (`ARCH-007`).

## Edge cases

- **Non-RGB documents** (CMYK, Lab, Grayscale, Bitmap, Indexed, Duotone): the
  command is RGB-only; disable elsewhere.
- **32-bit/channel**: not in the supported-adjustment list
  (`04-image-ops/32-bit-hdr.md`); disable *(inferred)*.
- **Source = target with no selection**: matches the image to its own overall
  statistics — usually a near-no-op, but Neutralize/Fade still apply.
- **Different color profiles** between source and target: behavior unspecified;
  compute in a defined common space.
- **Empty/global selection**: fall back to overall image statistics, as the Help
  states.
- **Fade extremes**: 0 (the default) applies the full match; moving right
  reduces it and 100 withholds it entirely (identity).
- **1-px / flat-color documents**: standard deviation is zero; guard against
  divide-by-zero in the transfer.
- **Huge/PSB documents**: statistics pass streams by tile; preview uses a
  downsampled proxy.
- **Indexed/Bitmap**: statistics on an index buffer are meaningless; disable.

## Parity acceptance criteria

1. Given two RGB images and no selections, applying Match Color with source =
   image B shifts the target's per-channel mean toward B's within the tolerance
   defined in `11-cross-cutting/testing-strategy.md` when Neutralize is off and
   Color Intensity/Luminance are 100.
2. Given `Color Intensity = 1`, the target becomes grayscale (R = G = B per
   pixel) within ±1 LSB.
3. Given `Luminance = 100` and `Fade = 100`, mean luminance of the target is
   unchanged within tolerance; Luminance >100 brightens and <100 darkens
   monotonically.
4. Given `Fade = 100`, the target is unchanged (identity); `Fade = 0` applies
   the full match.
5. Given a target selection and `Ignore Selection When Applying Adjustment` on,
   the whole target is modified; with it off, only the selection is modified.
6. Given `Source = None`, the command still runs against the target's own
   statistics and Neutralize/Fade apply.
7. Given `Save Statistics` then `Load Statistics` in a fresh session on the same
   target, the result reproduces the first run bit-for-bit.
8. The command is disabled for non-RGB documents.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, section "Match the color in different images" (and
  "Color adjustment commands"): RGB-only restriction, source/target model,
  layer matching, Image Statistics and Destination Image options, Neutralize,
  Luminance 1–200/100, Color Intensity 1–200/100 (1 = grayscale), Fade
  behaviour, Save/Load Statistics, Eyedropper/Info-panel feedback.
- `https://www.photo.net/...` — not reachable (HTTP 403); the commonly cited
  statistics-transfer basis is instead attributed to the Reinhard et al. paper
  named below. Secondary.
- `Reinhard, Ashikhmin, Gooch, Shirley, *Color Transfer between Images*, IEEE
  CG&A 2001` — the standard mean/std color-transfer algorithm referenced as a
  behavioural model. Not fetched; cited as a standard algorithm.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD color-mode enumeration (context for the RGB-only restriction).

## Open questions

- **Exact algorithm and color space.** Is CS6's Match Color a mean/std transfer
  in Lab, lαβ, or a proprietary space? *Resolves with:* controlled two-image
  tests + a deconvolution of the transfer, or an Adobe statement.
- **Fade range and default.** The fetched Help states the direction only.
  *Resolved for this implementation by the `add-match-color` change* (0–100,
  default 0 = full match); the CS6 default stays a parity open question.
  *Resolves with:* a CS6 dialog capture.
- **Default states of the option checkboxes** (`Neutralize`,
  `Use Selection In …`, `Ignore Selection …`). *Resolves with:* CS6 default
  screenshots.
- **Cross-profile statistics.** Which profile space is used when source and
  target documents have different embedded profiles? *Resolves with:* CS6 test
  with mismatched profiles.
- **`Save/Load Statistics` file format.** The extension and serialization are
  undocumented. *Resolved for this implementation with an in-tree binary
  format (`PCTMCST` magic, version byte, six little-endian f64s); Adobe's own
  serialization stays a parity open question. Resolves with:* inspecting a
  file saved by CS6.
- **32-bit availability.** Whether Match Color is enabled for 32-bpc documents.
  *Resolves with:* CS6 observation / the supported-adjustment list.
- **Selection partial-coverage semantics** for statistics and application.
  *Resolves with:* a feathered-selection test.
