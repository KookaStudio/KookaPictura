# Dodge, Burn, and Sponge Tools

- **Spec ID:** `TOOL-041`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the three tools are long-standing. The `Protect Tones` (Dodge/Burn) and `Vibrance` (Sponge) options are documented in the CS6 Help; the version they were introduced in is not established here (see `## Open questions`).
- **Depends on:** `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-004` rust-qt-interop, `07-color-painting/brush-engine.md`, `07-color-painting/color-models.md`, `03-tools/smudge-blur-sharpen.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Dodge**, **Burn**, and **Sponge** are three tools that share one toolbox slot
(single-letter shortcut `O` for the group in the CS6 shortcut table). They are
tonal retouch tools in the darkroom tradition. The CS6 Help describes Dodge and
Burn as follows: Dodge "lightens" and Burn "darkens" areas, based on "a
traditional darkroom technique for regulating exposure"; "The more you paint over
an area with the Dodge or Burn tool, the lighter or darker it becomes." Sponge
"subtly changes the color saturation of an area."

- **Dodge / Burn options bar** — brush tip, a **Range** menu, **Exposure**, an
  **Airbrush** toggle, and **Protect Tones**:
  - Range: **Midtones** ("Changes the middle range of grays"), **Shadows**
    ("Changes the dark areas"), **Highlights** ("Changes the light areas").
  - Exposure sets the amount of the dodge/burn.
  - Protect Tones: "minimize clipping in the shadows and highlights. This option
    also tries to keep colors from shifting hue."
- **Sponge options bar** — brush tip, a **Mode** menu, **Flow**, and
  **Vibrance**:
  - Mode: **Saturate** ("Intensifies the color's saturation") and
    **Desaturate** ("Dilutes the color's saturation").
  - Flow sets the rate of the effect.
  - Vibrance: "minimize clipping for fully saturated or desaturated colors."
- **Grayscale behaviour (Sponge)** — in Grayscale mode the tool "increases or
  decreases contrast by moving gray levels away from or toward the middle gray."
- **Destructive by default** — the Help warns that applying Dodge or Burn to the
  background layer "permanently alters the image information"; it recommends a
  duplicate layer for non-destructive work. Sponge has the same property
  *(inferred; community source).*
- **Not available at 32 bpc** — the Help's 32-bpc feature list explicitly
  excludes Dodge, Burn, and Sponge from the allowed tools.
- **Painting-mode shortcuts** — while the tool is active, the CS6 keyboard table
  lists `Shift+Alt+S` / `Shift+Alt+M` / `Shift+Alt+H` to select Dodge/Burn
  shadows / midtones / highlights, and Sponge `Shift+Alt+S` / `Shift+Alt+D` for
  Saturate / Desaturate.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox retouch slot | Tool (fly-out group) | `O` | Dodge, Burn, Sponge share one slot; visible tool = last used |
| Options bar (Dodge/Burn) | Tool options | `O` | Brush picker, Range, Exposure, Airbrush, Protect Tones |
| Options bar (Sponge) | Tool options | `O` | Brush picker, Mode, Flow, Vibrance |
| Range quick keys | Modifier | `Shift+Alt+S/M/H` | Shadows / Midtones / Highlights for Dodge/Burn |
| Sponge mode quick keys | Modifier | `Shift+Alt+S/D` | Saturate / Desaturate |
| Brush panel | Dock | `F5` | Tip shape, dynamics, pressure mappings |
| Preferences > Cursors | Pane | `Ctrl/Cmd+K` | "Painting Cursors" controls the pointer for these tools |
| Edit > Undo / Step Backward | Menu | `Ctrl+Z` / `Ctrl+Alt+Z` | One history state per completed stroke |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush tip | preset | soft round, hardness 0 | any brush preset | Options bar picker |
| Range (Dodge/Burn) | enum | Midtones *(inferred)* | Shadows / Midtones / Highlights | Tonal band the tool affects |
| Exposure (Dodge/Burn) | percent | 50 *(inferred; community source)* | 1–100 | Per-dab amount; number keys set it |
| Airbrush (Dodge/Burn) | bool | Off | on / off | Build-up while holding the button |
| Protect Tones (Dodge/Burn) | bool | On *(inferred)* | on / off | Clipping + hue-shift protection |
| Mode (Sponge) | enum | Saturate *(inferred)* | Saturate / Desaturate | Direction of saturation change |
| Flow (Sponge) | percent | 50 *(inferred; community source)* | 1–100 | Per-dab rate; `Shift`+number keys |
| Vibrance (Sponge) | bool | Off *(inferred)* | on / off | Limits clipping at the saturation extremes |
| Pressure size | bool | On for pressure devices | on / off | Brush panel mapping |
| Pressure flow/exposure | bool | Off *(inferred)* | on / off | Brush panel mapping |

## Algorithms & pipeline

Behavioral parity only; Adobe's exact implementation is closed and the following
is a publicly documented model *(inferred)* unless a CS6 Help statement is quoted.

### Tonal range weighting

All three tools are local, tone-selective operators under a brush mask. For a
pixel with luminance `Y ∈ [0,1]`, the selected **Range** contributes a weight
`w(Y)`:

- **Shadows**: weight peaks at `Y ≈ 0` and falls off toward midtones.
- **Midtones**: weight peaks around `Y ≈ 0.5`.
- **Highlights**: weight peaks at `Y ≈ 1`.

The exact curves are not documented. A common implementation uses smooth
(non-overlapping) band weights such as a raised-cosine or Gaussian in `Y`
centred on the three zones. Treat the curve as a tunable; the acceptance
criteria only require that a shadows stroke changes shadows far more than
highlights.

### Dodge and Burn

Both apply an exposure gain modulated by `effect = Exposure * w(Y) * m`, where
`m` is the brush coverage mask:

- **Dodge** lightens. A standard photo-pipeline model is a multiplicative gain
  toward white, e.g. `dst = src * (1 + k)` (or the screen/`1/(1-a)` form); the
  exact gain curve is *(inferred)*.
- **Burn** darkens. The inverse model, e.g. `dst = src * (1 - k)` or
  `dst = src / (1 + k)`.

Accumulation across dabs is what makes repeated passes progressively lighter or
darker, matching "the more you paint over an area … the lighter or darker it
becomes."

**Protect Tones** is modelled as a soft limiter that pulls the result back from
0/1 and adjusts the per-channel gains toward a luminance-only change so hue and
saturation are preserved; the exact rule is *(inferred)*. With it off, channels
clip independently, which is the classic cause of hue shift.

### Sponge

Sponge changes saturation within the selected tonal band. In a hue-based model
(HSL/HSV/Lab chroma), the saturation is scaled:

- **Saturate**: `S' = S + Flow * w(Y) * m * (1 - S)` (asymptotic to 1).
- **Desaturate**: `S' = S * (1 - Flow * w(Y) * m)`.

**Vibrance** reduces the applied change for pixels already near the saturation
extremes, so the tool does not clip fully saturated colours and does not push
near-grey colours past the desaturation floor. In Grayscale documents the same
"away from / toward middle gray" description maps to contrast around 0.5;
whether the grayscale path uses the Range weighting at all is *(inferred)*.

### Placement in the pipeline

The tools sample the **active layer** (Dodge/Burn/Sponge have no Sample All
Layers option) and write to it. They read the composited colour under the brush
for tone/hue decisions only if Photoshop does; the safest parity model is to
read and write the active layer directly *(inferred)*, matching the Help's
warning that the edit is applied to the layer.

## Rust module mapping

Proposals, sharing the brush/tool substrate with `TOOL-040`.

- `pictura_tools::tool` — `Tool` trait, `ToolId`, `ToolContext`, stroke
  lifecycle (shared).
- `pictura_tools::tonal` — `TonalTool { kind: Dodge | Burn | Sponge, options }`.
- `pictura_tools::tonal::range` — `ToneRange` enum and `range_weight(Y)` band
  weighting; a single function used by all three tools.
- `pictura_filters::tonal` — CPU kernels `dodge_dab`, `burn_dab`, `sponge_dab`
  over a `TilePatch`, with `protect_tones` and `vibrance` flags; rayon across
  touched tiles. Operates in working space, converting to/from the document
  colour model at the tile boundary (`pictura_color`).
- `pictura_render::tonal` — optional wgpu compute variants for interactive
  latency; CPU is the reference.

Crossing types: `TonalOptions { range: ToneRange, amount: f32, protect_tones:
bool }`, `SpongeOptions { mode: SpongeMode, flow: f32, vibrance: bool }`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `DodgeBurnOptionsWidget` | `QWidget` (options bar) | Brush picker, Range combo, Exposure spin/scrubber, Airbrush, Protect Tones |
| `SpongeOptionsWidget` | `QWidget` (options bar) | Brush picker, Mode combo, Flow spin/scrubber, Vibrance |
| `ToolOptionsBar` | `QWidget` stack | Swaps the options widget per active tool (`ARCH-003`) |
| `CanvasView` | `QGraphicsView` | Pointer capture and dab dispatch; `Shift+Alt+letter` quick keys |
| `BrushPresetModel` | `QAbstractItemModel` | Shared brush presets |
| `ToolPresetModel` | `QAbstractItemModel` | Persists per-tool options |

Range/Exposure/Flow use scrubby numeric fields to match CS6's options-bar
interaction. The options bar is rebuilt when the toolbox slot changes tool.

## Data-model impact

- **Destructive pixel edit** to the active layer; each completed stroke is one
  **history state** of tile deltas (`ARCH-009`). No new nodes, channels, or
  metadata.
- **Colour-model dependence**: the operator runs in the document's working space,
  but saturation is defined only in a hue-bearing model. CMYK and Lab documents
  need a defined saturation/lightness mapping (see
  `01-architecture/color-management.md` and
  `07-color-painting/color-models.md`); the mapping must be documented, not
  implicit.
- **32-bpc**: these tools are *unavailable* at 32 bpc. The document model must
  expose a per-tool availability query so the toolbox/options bar can disable the
  tools without hard-coding the bit depth in the UI.
- **Tool options persistence**: Range, Exposure, Protect Tones, Sponge Mode/Flow/
  Vibrance live in tool presets and session tool state
  (`11-cross-cutting/preference-storage.md`), not in the PSD.

## Edge cases

- **Background vs layer** — the Help notes the background-layer edit is permanent;
  on a standard layer the same is true but pixels may be transparent. Both write
  through the normal destructive path. Locked-alpha must be honoured.
- **Clipping** — Dodge on near-white and Burn on near-black must saturate
  gracefully; Protect Tones on should asymptote rather than hard-clip.
- **Hue preservation** — with Protect Tones off, per-channel gain can shift hue;
  that is CS6-consistent, so it must not be "fixed" silently.
- **Grayscale** — Sponge maps to contrast around middle gray, not saturation;
  Range weighting behaviour at 1 channel is *(inferred)*.
- **Indexed/Bitmap** — tonal tools are expected to be unavailable; grey out and
  document.
- **CMYK/Lab** — saturation/concentration semantics differ per model; the
  conversion must be defined and tested, and must not accumulate rounding drift
  across dabs.
- **1-px / empty / huge documents** — dabs touch only the brush footprint tiles;
  no full-canvas allocation. A 1×1 document still accepts a dab; empty documents
  are rejected at open.
- **GPU unavailable** — identical CPU results, higher latency.
- **Undo mid-stroke** — atomic per stroke; a mid-drag `Ctrl+Z` is ignored or
  cancels the stroke and creates no state.
- **Exposure/Flow 0** — no-op; must not create a history state.

## Parity acceptance criteria

- Given a grey ramp image, a **Shadows** Dodge stroke lightens the dark end much
  more than the light end, and vice versa for **Highlights**; a **Midtones**
  stroke peaks in the middle.
- Given the same exposure and repeated identical passes, the result gets
  progressively lighter (Dodge) or darker (Burn), saturating rather than
  oscillating.
- Given **Protect Tones** on, a saturated midtone patch retains its hue within a
  small tolerance and does not clip to pure white/black; with it off, the
  measured hue shift is larger (CS6-consistent).
- Given a fully saturated colour patch, a Sponge **Saturate** stroke changes it
  less with **Vibrance** on than with it off.
- Given a near-grey patch, Sponge **Desaturate** does not push channels below 0
  or produce colour casts.
- Given a Grayscale document, Sponge increases/decreases contrast around middle
  gray.
- Given a 32-bpc document, the Dodge/Burn/Sponge tools are disabled, and no
  shortcut can invoke them.
- Given a completed stroke, History shows exactly one new state and undo restores
  pixels bit-exactly at the document bit depth.
- Given a C, M, Y, K or L, a, b document, the operator completes without
  out-of-range values and its colour-model conversion is deterministic.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: Dodge/
  Burn darkroom description and "the more you paint … the lighter or darker it
  becomes"; Range = Midtones / Shadows / Highlights (with the quoted
  definitions); Exposure; Airbrush; Protect Tones "minimize clipping … also tries
  to keep colors from shifting hue"; the background-layer permanence warning;
  Sponge Modes Saturate / Desaturate, Flow, and Vibrance; the Grayscale-middle-
  gray behaviour; the 32-bpc exclusion list naming Dodge, Burn, Sponge; the `O`
  group shortcut and the Dodge/Burn/Sponge painting-mode quick keys.
- `https://glensmith.co.uk/photoshop/dodge-tool` — community tutorial (CC-era):
  Range and Exposure options, Protect Tones, exposure default 50%, hardness 0,
  destructive-on-duplicate advice. Secondary source.
- `https://glensmith.co.uk/photoshop/sponge-tool` — community tutorial (CC-era):
  Mode (Saturate/Desaturate), Flow default 50%, Vibrance option, destructive
  advice. Secondary source.
- The two `glensmith.co.uk` tool pages above were reached directly from that
  site's tool index; no search engine contributed a fact.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **CS6 defaults.** Range = Midtones, Exposure = 50%, Protect Tones on, Sponge
  Mode = Saturate, Flow = 50%, Vibrance off are all inferred from a CC-era
  community source or general knowledge, not CS6. Resolves with: a first-run CS6
  options-bar capture.
- **Introduction versions of `Protect Tones` and `Vibrance`.** Both are in the
  CS6 Help, but whether either was new in CS6 versus CS5 is unverified. Resolves
  with: the CS5 Help or side-by-side options bars.
- **Exact Range weighting curves.** Not documented. Resolves with: fitting the
  operator to CS6 output on a grey ramp (calibration).
- **Protect Tones algorithm.** The limiter/hue-preservation rule is closed.
  Resolves with: a CS6 calibration and a documented approximation, marked
  behavioural-parity-only.
- **Vibrance formula.** Whether CS6 uses a fixed saturation-falloff or a
  curve is unknown. Resolves with: a saturation-sweep comparison.
- **Grayscale path for Sponge.** Whether Range still selects tonal bands in
  Grayscale is not documented. Resolves with: a Grayscale-mode CS6 test.
- **Does Dodge/Burn/Sponge read the active layer only, or the composite?** No
  Sample All Layers option is documented; the read source is unverified. Resolves
  with: a two-layer CS6 experiment.
- **Availability in CMYK/Lab and Indexed/Bitmap.** Confirm per colour mode.
  Resolves with: a CS6 mode test; feeds `04-image-ops/image-modes.md`.
