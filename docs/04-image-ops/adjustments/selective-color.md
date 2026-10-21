# Selective Color

- **Spec ID:** `ADJ-023`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the command is long-standing. CS6 changes: settings live in the Properties panel, which also carries the Colors menu and a Preset menu; in CS6 the color to adjust is chosen from the Colors menu there (CS5 used the Adjustments panel). Selective Color is in the CS6 preset-save whitelist.
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-008` document-model, `ARCH-009` undo-history, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `07-color-painting/color-models.md`, `05-layers/adjustment-layers.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Selective Color** changes the amount of process (CMYK) ink in individually
selected color components. The CS6 Help describes it as a correction technique
borrowed from high-end scanners and separation software: it changes the amount
of each process color within one primary color component without disturbing the
others. For example, the cyan in an image's green component can be cut sharply
while the cyan in its blue component stays the same.

- **Color families (the Colors menu).** The Help directs you to choose the color
  to adjust from the Colors menu; the UI exposes **nine** entries: six hue
  families — **Reds, Yellows, Greens, Cyans, Blues, Magentas** — plus three tonal
  entries — **Whites** (highlights), **Neutrals** (midtones), **Blacks**
  (shadows). (Note: the request brief's "nine color families + neutral/black/
  white" is a misread of the UI; the nine *are* the six hues plus the three tonal
  entries.) *(hue list: secondary source; tonal mapping: secondary source)*
- **Per-family controls.** For the selected entry, four sliders adjust **Cyan,
  Magenta, Yellow, Black**. All four are editable for all nine entries.
- **Method.** Choose **Relative** or **Absolute**:
  - **Relative** (default per conventional Photoshop behavior; the Help documents
    the semantics but not the default): scales the existing amount of an ink by
    its percentage of that total. The Help's example: a pixel that is 50% magenta
    plus 10% gains 5% magenta (10% of 50% = 5%), reaching 55%. Pure specular white
    contains no color components, so Relative cannot move it.
  - **Absolute**: adjusts the ink in absolute values. The Help's example: a pixel
    that is 50% magenta plus 10% sets the magenta ink to 60%.
- **Interpolation between families.** The correction is weighted by how close a
  color is to each Colors-menu option. The Help's example: 50% magenta sits midway
  between white and pure magenta, so it gets a proportionate mix of the
  corrections defined for those two colors. A pixel therefore never belongs
  wholly to one family; it receives a weighted blend of the corrections of its
  nearest families.
- **RGB support.** Although Selective Color corrects an image using CMYK inks, the
  Help confirms it can operate on RGB images too.
- **Channel requirement.** The composite channel must be selected in the Channels
  panel; the Help states the adjustment is available only while viewing the
  composite channel.
- **Paths.** Adjustments panel icon (CS5) / Properties panel (CS6) → non-destructive
  **Selective Color adjustment layer**; `Layer > New Adjustment Layer > Selective
  Color`; `Image > Adjustments > Selective Color` (destructive). CS6 also exposes
  saved Selective Color presets in the Properties panel Preset menu.
- **Settings persistence.** Selective Color is in the preset-save whitelist: its
  settings can be saved via the panel menu's Save Preset option, which the Help
  restricts to Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel
  Mixer, and Selective Color.

### Behavior in RGB vs CMYK

| Document mode | Documented behavior | Internal model |
|---|---|---|
| CMYK | Adjusts the four process inks directly | Direct CMYK arithmetic; each family applies its ink corrections to the pixel's CMYK values, weighted by family membership |
| RGB | Works, per the Help | The RGB pixel is transformed to an equivalent CMYK ink amount (via the document CMYK working space / profile) to compute corrections, then transformed back to RGB. The exact round-trip path, intent handling, and profile are undocumented *(inferred)* |
| Grayscale | Available (single ink-equivalent channel) *(inferred)* | Only Black-family / neutral corrections are meaningful; colors behave as neutral *(inferred)* |
| Lab | Available? *(inferred / open)* | Requires a Lab↔CMYK bridge; behavior not documented |

Because the Relative method can only scale existing ink, a pure specular white
(`0/0/0/0`) cannot be moved by Relative; that is called out by the Help, not a bug.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Selective Color` | Menu command | — | Destructive |
| Adjustments panel (CS5) / Properties panel (CS6) | Panel icon | — | Non-destructive adjustment layer |
| `Layer > New Adjustment Layer > Selective Color` | Menu | — | Non-destructive |
| Properties panel — Colors menu | Combo | — | Nine entries (6 hues + 3 tonal) |
| Properties panel — Method | Radio/combo | — | Relative / Absolute |
| Properties panel — four sliders | Slider + numeric | — | Cyan, Magenta, Yellow, Black; −100…+100 |
| Properties panel — Preset menu | Combo | — | CS6 saved/preset Selective Color settings |
| Channels panel | Requirement | — | Command active only with composite channel selected |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Colors | enum | Reds *(inferred)* | Reds / Yellows / Greens / Cyans / Blues / Magentas / Whites / Neutrals / Blacks | One correction set per entry |
| Method | enum | Relative *(inferred)* | Relative / Absolute | Semantics per the Help |
| Cyan amount | percent | 0 | −100 … +100 | Per current Colors entry |
| Magenta amount | percent | 0 | −100 … +100 | Per current Colors entry |
| Yellow amount | percent | 0 | −100 … +100 | Per current Colors entry |
| Black amount | percent | 0 | −100 … +100 | Per current Colors entry |
| Blend mode | enum | Normal | 27 CS6 modes | Adjustment-layer path |
| Opacity | percent | 100 | 0–100 | Adjustment-layer path |
| Layer mask | gray | white | per-pixel | Adjustment-layer path |
| Clip to layer below | bool | Off | on/off | Adjustment-layer path |

The Help documents the Relative/Absolute semantics but not the slider range; the
−100…+100 range and the default color/method are *(inferred; secondary source
confirms −100…+100)*.

## Algorithms & pipeline

The Help gives the two method formulas and the interpolation principle exactly;
the family-weighting and RGB↔CMYK internals are closed. Mark behavioral parity
only where noted.

### Per-family ink correction

Let `x ∈ {C, M, Y, K}` be the current ink amount of the pixel for the active
family, `a` the slider amount in percent (`−100 … +100`), and clamp to
`[0, 100]`:

```
Relative:  x' = clamp( x + (a/100)·x ,  0, 100 )
Absolute:  x' = clamp( x + a          ,  0, 100 )
```

- Relative matches the Help's 50 % + 10 % → 55 % example; Absolute matches
  50 % + 10 % → 60 %.
- Negative amounts symmetrically reduce: Relative 50 % − 10 % → 45 %; Absolute
  50 % − 10 % → 40 %.
- Relative is idempotent only trivially; Absolute is not (two +10 passes give +20
  as two distinct layers).

### Family weighting / interpolation

Each pixel is assigned membership weights `w_f ≥ 0`, `Σ w_f = 1`, over the nine
families, based on hue for the six chromatic entries and tone for
Whites/Neutrals/Blacks. The pipeline computes:

```
cmyk_out = Σ_f w_f · cmyk_f(x, a_f)
```

where `cmyk_f` is the per-family Relative/Absolute correction. The Help's
midway-color example (50% magenta lying between white and pure magenta and
receiving a proportionate mix of the two corrections) is the two-family special
case.
**The exact membership functions (hue angles, tone windows, overlap) are not
documented**; implement as tunable calibration curves and mark behavioral parity
only. *(inferred)*

### RGB ↔ CMYK

- CMYK documents: apply the arithmetic directly in the document CMYK space.
- RGB documents: the correction is defined in CMYK ink amounts, so the engine must
  convert RGB→CMYK (using the document's CMYK working space and rendering intent),
  apply the weighted correction, clamp ink to `[0,100]`, and convert back
  CMYK→RGB. Because round-tripping RGB through CMYK is lossy and profile-dependent,
  the rendered result depends on the assigned CMYK working space even though the
  document is RGB. **Adobe does not document this path**; behavioral parity only.
  *(inferred)*
- Rendering intent, black-point compensation, and whether the conversion is
  absolute or relative colorimetric are undocumented. *(open question)*

### Order in the layer stack

Selective Color is an ordinary adjustment layer: its result is a function of the
accumulated color below it, composited with blend mode, opacity, mask, and
clipping. It does not read other layers directly.

## Rust module mapping

- `pictura_adjust::selective_color` — `SelectiveColorOp` holding a map
  `[Family; 9] -> CmykDelta { c, m, y, k: i16 }` plus `method: CorrectionMethod`.
- `pictura_adjust::selective_color::weights` — `FamilyWeights` with `fn weights_for(cmyk_or_rgb, space) -> [f32; 9]`
  (calibratable curves; the knob lives here).
- `pictura_color::convert` — RGB↔CMYK conversion via the document profile
  (`lcms2`/`qcms` candidate; see `01-architecture/color-management.md`), with a
  documented intent.
- `pictura_adjust::traits` — `Adjustment`, `AdjustmentKind::SelectiveColor`,
  `ParameterSpec` generating the nine-by-four grid.
- `pictura_render::adjust` — optional GPU variant; CPU is the reference and the
  color-conversion path should stay on the CPU/reference side for determinism.

Crossing types: `SelectiveColorParams { families: [CmykDelta; 9], method: CorrectionMethod }`,
`CmykDelta`, `CorrectionMethod { Relative, Absolute }`, `Family`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SelectiveColorPropertiesWidget` | `QWidget` | Colors combo, Method radio group, four scrubby sliders (−100…+100), Preset combo |
| `CmykSliderGroup` | `QWidget` | Reusable C/M/Y/K slider cluster (also used by Channel Mixer) |
| `AdjustmentPropertiesPanel` | `QWidget` stack | Selects the Selective Color page |
| `PresetComboModel` | `QAbstractItemModel` | CS6 Preset menu contents; load/save |
| `AdjustmentListModel` | `QAbstractItemModel` | Adjustment-layer list/thumbnails |

Widgets over QML for the dense grid; the C/M/Y/K cluster is shared with other
CMYK-driven adjustments.

## Data-model impact

- **Layer node.** `AdjustmentKind::SelectiveColor` with
  `SelectiveColorParams { families, method }` (nine C/M/Y/K deltas). Serialize only
  non-zero families to keep files small.
- **Serialization.** PSD stores the Selective Color adjustment-layer parameters in
  its per-adjustment record; confirm the exact keys in
  `01-architecture/file-formats.md`. XMP needs the layer entry only.
- **Presets.** Selective Color is in the CS6 preset-save whitelist, so a
  `SelectiveColorPreset` (named `families` + `method`) is a first-class persisted
  object alongside Levels/Curves/Exposure/Hue-Saturation/B&W/Channel Mixer
  presets (`10-workflow-io/presets-manager.md`).
- **Undo.** Layer create/delete separate from parameter edits. For UX parity, a
  single slider commit is one history record; a drag coalesces on release.
  Changing the Colors entry is a UI-only change unless the slider is moved.
- **Working space dependency.** The RGB result depends on the document CMYK working
  space; changing the working space after the fact must re-render, so the
  adjustment must be evaluated lazily against the current profile, not baked.

## Edge cases

- **Pure white under Relative** — `0/0/0/0` cannot move; the Help documents this.
  Do not special-case it into Absolute.
- **Pure magenta / pure ink families** — a pixel exactly on a family vertex must
  receive that family's full correction with weight 1.
- **Family overlap** — pixels between two families blend corrections; the exact
  weights are unresolved and must not silently collapse to nearest-neighbour.
- **Neutral gray** — should be governed by Whites/Neutrals/Blacks, not the six
  hues; verify weighting is 0 for chromatic families.
- **RGB round-trip** — out-of-gamut results must be clipped in CMYK ink, not in
  RGB, or hue will shift unexpectedly; the profile path must be deterministic.
- **CMYK total ink** — corrections can push total ink above the profile limit; the
  Help does not promise limiting. Do not silently reduce other channels.
- **Composite-only requirement** — with a single channel targeted in the Channels
  panel, the command must be disabled, matching the Help.
- **Grayscale / Lab / Indexed / Bitmap** — verify availability and behavior per
  mode; Indexed/Bitmap expected unavailable.
- **32-bpc** — not in the 32-bpc adjustment-layer whitelist; expected unavailable
  as an adjustment layer. Confirm.
- **Empty / 1-px / huge documents** — tile-local evaluation; profile conversion at
  tile boundaries must use consistent context to avoid seams.
- **GPU unavailable** — identical CPU result; prefer CPU for the color conversion.
- **Undo/redo** — parameter round-trip exact; profile changes are a document-state
  change, not an adjustment undo.

## Parity acceptance criteria

- Given a CMYK document, 50 % magenta, Colors = Magentas, relative +10 → 55 %
  magenta; the other three inks are unchanged.
- Given the same setup with Absolute +10 → 60 % magenta.
- Given Colors = Reds, moving only the Cyan slider changes cyan in red-family
  pixels while leaving blue-family cyan approximately unchanged.
- Given a 50 % magenta pixel, because it is midway between white and pure
  magenta, the result lies between the corrections defined for Whites and
  Magentas in proportion (per the Help's interpolation statement).
- Given Relative and a pure specular white pixel, the pixel is unchanged.
- Given an RGB document, the command operates (no error) and produces a
  deterministic result for a fixed CMYK working space.
- Given a non-composite channel selected in the Channels panel, the command is
  disabled.
- Given an adjustment layer, changing any slider updates the canvas live and undo
  restores the previous value as one state per commit.
- Given a saved Selective Color preset, loading it on another document reproduces
  the same `families` and `method` values.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  selective-color definition and the green-vs-blue cyan example; the Colors menu
  and the CS6 Properties-panel location; the Relative and Absolute definitions with
  the 50 %+10 %→55 % / 50 %+10 %→60 % examples; the pure-specular-white Relative
  limitation; the interpolation statement (a correction is weighted by how close
  the pixel is to a Colors-menu option, so 50% magenta receives a proportionate
  mix of the two neighbouring corrections); the note that Selective Color can be
  used on RGB images even though it uses CMYK inks; the composite-channel
  requirement; the three application paths; the preset-save whitelist including
  Selective Color.
- `https://shotkit.com/selective-color-photoshop` — CC-era secondary source:
  confirms the nine Colors entries as "Reds, Yellows, Greens, Cyans, Blues,
  Magentas, Whites, Neutrals and Blacks" and the four sliders Cyan/Magenta/Yellow/
  Black (range not stated there).
- `https://expertphotography.com/selective-colour-photoshop` — secondary source:
  confirms Whites = highlights, Neutrals = midtones, Blacks = shadows, and that
  the sliders work on complementary pairs (e.g. the Cyan slider adds red when moved
  the other way). CC-era.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Slider range.** Is the CS6 range −100…+100? Confirm from the CS6 UI (the
  secondary source states −100…+100 but is not CS6-specific). Resolves with: a CS6
  panel capture.
- **Default Colors entry and Method.** Reds / Relative is inferred. Resolves with:
  a first-run CS6 panel capture.
- **Family membership functions.** Hue/tone weighting curves are undocumented.
  Resolves with: a CS6 calibration sweep across hue and luminance.
- **RGB→CMYK path.** Which profile, rendering intent, and black-point behavior.
  Resolves with: an RGB probe under a known CMYK working space.
- **Lab support.** Whether Selective Color is enabled in Lab mode at all. Resolves
  with: a CS6 Lab-mode test.
- **Absolute clamping.** Whether out-of-range ink is clipped to 0/100 or wraps in
  the pipeline. Resolves with: an extreme-value CS6 probe.
- **Grayscale mapping.** How six hue families behave with no chroma. Resolves with:
  a CS6 Grayscale test.
- **PSD keys.** Exact serialization keys for the nine families and method.
  Resolves with: the Adobe file-format specification.
