# Smudge, Blur, and Sharpen Tools

- **Spec ID:** `TOOL-040`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Blur, Sharpen, and Smudge predate CS6 and are carried forward. The Sharpen tool's `Protect Detail` option is documented in the CS6 Help but its introduction version is not established here (see `## Open questions`).
- **Depends on:** `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-004` rust-qt-interop, `07-color-painting/brush-engine.md`, `07-color-painting/brush-dynamics.md`, `03-tools/brush-and-pencil.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

The **Blur**, **Sharpen**, and **Smudge** tools are three separate tools that
share one toolbox slot. The slot shows the last tool used; the other two are
reached by clicking and holding the slot (the Help says of Sharpen, "If the tool
isn't visible, hold down the Blur tool"). All three are *destructive* edits to
the active layer's pixels unless the user works on a duplicate layer or a
separate layer.

- **Blur** softens hard edges and reduces detail. "The more you paint over an
  area with the tool, the blurrier it becomes."
- **Sharpen** increases contrast along edges to increase apparent sharpness. It
  offers a **Protect Detail** option: "enhance details and minimize pixelated
  artifacts"; with it deselected the sharpening is "more exaggerated."
- **Smudge** simulates dragging a finger through wet paint: "The tool picks up
  color where the stroke begins and pushes it in the direction you drag." It has
  a **Finger Painting** option; when selected, the stroke starts from the
  **foreground color** instead of the color under the pointer. Holding
  `Alt`/`Option` while dragging temporarily enables Finger Painting.

All three share two options-bar controls:

- **Sample All Layers** — use color/data from all *visible* layers; deselected
  (default), the tool uses only the active layer. For Smudge this is what makes
  painting onto an empty layer blend with the layers below *(inferred)*.
- **Strength** — the per-stroke amount of the effect *(CS6 Help names it for
  Blur/Sharpen; the Smudge options bar exposes the same control — inferred from
  the tool-options family and community sources)*.
- **Mode** — a blending-mode menu ("options for the blending mode"). Available
  modes change with the tool; the CS6 Help does not enumerate the exact subset
  for these three tools.

The tools accept brush presets and tablet pressure. Pressure can drive size
and/or strength/opacity through the Brush panel dynamics. The Help lists Smudge,
Blur, Sharpen (with Dodge, Burn, Sponge) among the tools controlled by the
**Painting Cursors** preference.

Blur, Sharpen, and Smudge remain available on **32-bpc HDR** images: the Help's
"Paint on HDR images" list includes Blur, Sharpen, and Smudge. Dodge, Burn, and
Sponge are explicitly *not* available at 32 bpc (see `TOOL-041`).

None of the three has a default single-letter toolbox shortcut in the CS6
shortcut table; the retouch group is not listed among the lettered tool groups.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox retouch slot | Tool (fly-out group) | none assigned in CS6 | Blur, Sharpen, Smudge share one slot; visible tool = last used |
| Options bar | Tool options | n/a | Brush preset picker, Mode, Strength, Sample All Layers; Sharpen adds Protect Detail; Smudge adds Finger Painting |
| Brush panel | Dock | `F5` (Brush panel) | Tip shape, dynamics, scattering, pressure mappings |
| Brush Presets panel | Dock | n/a | Preset tips used by all three |
| Tool Presets | Options-bar picker | n/a | Stores options-bar settings with the tip |
| Preferences > Cursors | Pane | `Ctrl/Cmd+K` | "Painting Cursors" controls the pointer for these tools |
| Smudge + `Alt`/`Option` drag | Modifier | `Alt`/`Option` | Temporarily turns on Finger Painting while dragging |
| Edit > Step Backward / Undo | Menu | `Ctrl+Alt+Z` / `Ctrl+Z` | One history state per completed stroke |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush tip | preset | 1-px soft round (tool default) | any brush preset | Brush Preset picker in options bar |
| Size | int px | from preset | 1–5000 (brush-dependent) | `[` / `]` |
| Hardness | percent | from preset (soft) | 0–100 | `Shift+[` / `Shift+]` |
| Mode | enum | Normal | paint-mode list (subset unverified) | Blends the effect with underlying pixels |
| Strength | percent | 50 *(inferred; community source)* | 1–100 | Per-dab amount; number keys set it |
| Sample All Layers | bool | Off | on / off | Uses all visible layers when on |
| Finger Painting (Smudge) | bool | Off *(inferred)* | on / off | Foreground color at stroke start; `Alt`/`Option` while dragging |
| Protect Detail (Sharpen) | bool | Off *(inferred)* | on / off | Detail-preserving sharpening; off = more exaggerated |
| Pressure size | bool | On for pressure devices | on / off | Per Brush panel mapping |
| Pressure strength | bool | Off *(inferred)* | on / off | Per Brush panel mapping |
| Airbrush | bool | Off | on / off | Common painting-tool option; buildup behaviour |

## Algorithms & pipeline

Behavioral parity only; Adobe's exact kernels are closed and not published. The
description below is a publicly documented model *(inferred)* unless a CS6 Help
statement is quoted.

Shared stroke model (as in `07-color-painting/brush-engine.md`):

```text
for each dab at position p with coverage mask m in [0,1]:
    effect = strength * m * pressure_factor
    tile[p] = apply_effect(tile[p], neighborhood, effect)
```

- **Blur** — a local low-pass convolution of the source neighborhood, applied
  under the brush mask and accumulated per dab. A separable Gaussian (or a small
  box approximation) is the standard choice. Because the tool builds with
  repeated strokes, strength is a per-dab blend factor, not a one-shot radius:
  `dst = lerp(dst, gaussian(src, r), effect)`. Kernel radius for a given brush
  size is not documented; treat as a tunable.
- **Sharpen** — an unsharp-mask / high-pass operation:
  `dst = src + amount * (src - gaussian(src, r)) * effect`. **Protect Detail**
  is modelled as an edge/detail-aware limiter that suppresses the halo where the
  local gradient is high or where clipping would occur; the exact rule is
  *(inferred)*. Sharpen is destructive in Photoshop; performing it on a separate
  layer set to Luminosity is the documented recommendation for avoiding colour
  shifts.
- **Smudge** — a directional smear with a carried "brush load":
  ```text
  // at stroke start: load = sample(p)  (or foreground color if Finger Painting)
  // each dab, in drag direction d:
  sample = average(tile, p)                 // color under the brush
  tile[p] = lerp(tile[p], load, effect)     // deposit the carried color
  load    = lerp(load, sample, effect)      // pick up new color
  ```
  This matches the documented "picks up color where the stroke begins and pushes
  it" behaviour and the Finger Painting start color. The exact pickup/push ratio
  is *(inferred)*.

**Sample All Layers**: the tool reads from a flattened visible composite of the
document (or an equivalent per-pixel merge of visible layers) for sampling, but
writes only to the active layer. For Smudge this is required to smear onto an
empty layer; Blur/Sharpen apply the effect to the active layer while sampling the
composite. *(inferred.)*

**Bit depth**: pixels are processed in linear or working space at 8/16/32 bpc; all
three tools are on the Help's 32-bpc list. Smudge on 32-bpc float should clamp or
handle negative/HDR values safely.

## Rust module mapping

Proposals. These tools are brush-based edits like the paint tools, so they reuse
the shared brush/tool substrate rather than defining their own.

- `pictura_tools::tool` — `Tool` trait (`begin_stroke`, `dab`, `end_stroke`),
  `ToolId`, `ToolContext { doc: &mut Document, history: &mut History, options }`,
  pointer/stylus event translation.
- `pictura_tools::retouch` — `RetouchTool { kind: Blur | Sharpen | Smudge,
  options: RetouchOptions }`; `RetouchOptions { mode, strength, sample_all_layers,
  finger_painting, protect_detail }`.
- `pictura_tools::retouch::sample` — builds the per-dab source: active layer or a
  cached visible-layer composite when `sample_all_layers`.
- `pictura_filters::retouch` — CPU kernels `blur_dab`, `sharpen_dab`,
  `smudge_dab` operating on a `TilePatch`; rayon-parallel across touched tiles.
- `pictura_render::retouch` — optional GPU compute variants (wgpu) for
  interactive latency; CPU path is the correctness reference.
- `pictura_core::tile` — `TilePatch`, `TileId`, dirty-tile set returned to the
  history layer.

Crossing types: `StrokeId`, `TilePatch { tile: TileId, rect: Rect, before,
after }`, `BrushDab { center, radius, pressure, angle }`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `RetouchOptionsWidget` | `QWidget` (options bar) | Brush picker, Mode, Strength, Sample All Layers; conditional Finger Painting / Protect Detail |
| `ToolOptionsBar` | `QWidget` stack | Swaps the active tool's options widget; defined in `ARCH-003` |
| `CanvasView` / `CanvasWidget` | `QGraphicsView` / `QRhiWidget` | Pointer capture, dab dispatch, pressure via `QTabletEvent`, cursor preview |
| `BrushPresetModel` | `QAbstractItemModel` | Shared brush-preset list (also used by Brush, Eraser, etc.) |
| `ToolPresetModel` | `QAbstractItemModel` | Saves options-bar settings per tool |

The canvas forwards pointer/tablet events to the Rust tool through the bridge
(`ARCH-004`) on the GUI thread; dab work is submitted to a worker and the result
is published as repainted tiles. `QGraphicsView` is used for the vector overlay
and hit testing, `QRhiWidget` for the colour buffer (`ARCH-003`).

## Data-model impact

- **Destructive pixel edit**: strokes mutate the active layer's tiles. Each
  completed stroke is one **history state** holding tile deltas (`ARCH-009`).
  Undo restores the previous tile versions; redo re-applies.
- **No new document nodes.** The tools do not add layers, channels, or metadata.
- **Sampling cache**: when `sample_all_layers` is on, a transient visible-layer
  composite must be available at the dab's region; it is a cache, not serialized
  state, and must be invalidated when layer visibility/stack changes.
- **Tool options persistence**: Mode/Strength/Sample All Layers/Finger
  Painting/Protect Detail live in the tool's options and are saved through tool
  presets and the session's tool state (`11-cross-cutting/preference-storage.md`),
  not in the PSD.
- **Bit-depth**: tile deltas store the document's channel type (`u8`/`u16`/`f32`);
  a 32-bpc Smudge delta is 4× the 8-bpc size (`ARCH-009`).

## Edge cases

- **No visible source** — `Sample All Layers` on but every layer hidden, or an
  empty active layer with the option off: the dab is a no-op; do not write
  transparent/black.
- **Locked/background layer** — Dodge/Burn clear-pixel rules do not apply here,
  but a locked layer or a locked-alpha layer must reject partial writes
  consistently with the paint tools.
- **Layer mask / vector mask active** — painting must target the mask when the
  mask is selected, as with the Brush tool.
- **1-px and empty documents** — a 1×1 document still allows a dab; empty
  documents are rejected at open.
- **8/16/32 bpc** — 32-bpc allows HDR values; Smudge's carried-load lerp must not
  assume `[0,1]`. 8-bpc rounds to integers (dithering policy TBD).
- **CMYK/Lab** — sampling and blending happen in the document's working space; a
  Smudge load carried across an out-of-gamut colour must not clip incorrectly.
- **Indexed/Bitmap mode** — large classes of tools are unavailable; document
  whether Blur/Sharpen/Smudge are disabled (likely) and grey out the tool slot.
- **GPU unavailable** — dabs run on the CPU path with the same results; only
  latency changes.
- **Huge (PSB) documents** — dabs touch only the brush footprint's tiles; no
  full-canvas allocation.
- **Undo mid-stroke** — a stroke is atomic; `Ctrl+Z` mid-drag is ignored or
  cancels the stroke, never commits half a dab sequence.
- **Strength 0 / 100** — 0 is a no-op (should not create a history state);
  100 saturates in one dab.

## Parity acceptance criteria

- Given a flat test image and a single Blur stroke, edge gradient magnitude in
  the painted path decreases and pixels outside the brush footprint are
  bit-identical.
- Given the same image and a Sharpen stroke, edge contrast increases; with
  **Protect Detail** on, halo/clipping is measurably reduced versus off.
- Given a red-to-white gradient, a Smudge stroke started on red pushes red in the
  drag direction and does not modify pixels outside the stroke.
- Given **Finger Painting** enabled (or `Alt` held), the same stroke starts from
  the foreground colour instead of the pixel under the cursor.
- Given a two-layer document with **Sample All Layers** on, smudging on an empty
  top layer pulls colour from the layer below; with the option off, the empty
  layer yields no visible change.
- Given a completed stroke, the History panel shows exactly one new state and
  `Ctrl+Z` restores the pre-stroke pixels bit-exactly at 8, 16, and 32 bpc.
- Given all layers hidden with **Sample All Layers** on, the stroke is a no-op
  and creates no history state.
- Given the GPU is unavailable, the CPU path produces the same acceptance
  results within latency tolerance.
- Given a 32-bpc HDR image, Blur/Sharpen/Smudge operate and do not corrupt
  values outside `[0,1]`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  Blur tool "softens hard edges or reduces detail"; its options bar has Mode,
  Strength, Sample All Layers; Sharpen "increases contrast along edges"; Sharpen
  adds **Protect Detail**; Smudge "picks up color where the stroke begins and
  pushes it in the direction you drag"; Smudge options are Mode, Sample All
  Layers, **Finger Painting**, and `Alt`/`Option` while dragging; the three tools
  share a toolbox slot; Blur/Sharpen/Smudge are on the 32-bpc tool list; the
  painting-cursors preference covers them; the CS6 tool-shortcut table gives this
  group no letter.
- `https://glensmith.co.uk/photoshop/smudge-tool` — community tutorial (CC-era,
  but explicitly covers the same options): Mode, Strength (recommends 50%),
  Sample All Layers, Finger Painting, pressure-sensitive size. Secondary source.
- `https://www.adobe.com/products/photoshop/blend-colors.html` — Adobe page on
  blending colors with the Smudge tool and Sample All Layers (surfaced via search
  result; page content not fully parsed).
- SearXNG meta-search (queries: "Photoshop Smudge tool strength default 50% finger
  painting sample all layers"; "Photoshop Rotate View tool introduced CS4 or
  CS5") — used to locate the above secondary sources; no facts asserted from
  snippets alone.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Exact CS6 blend-mode subset for Blur/Sharpen/Smudge.** The CS6 Help says only
  "options for the blending mode." Resolves with: a CS6 options-bar capture or
  the archived Help's per-tool reference.
- **CS6 defaults for Strength, Protect Detail, Finger Painting, and Sample All
  Layers.** The 50% strength default and option defaults are inferred from a
  CC-era community source, not CS6. Resolves with: a first-run CS6 options-bar
  screenshot or the Help PDF's tool reference.
- **Sharpen `Protect Detail` algorithm and introduction version.** The option is
  documented in CS6, but the detail-preservation rule is closed and the version it
  first appeared in is unverified. Resolves with: a CS5/CS6 comparison and Adobe
  technical notes.
- **Blur and Sharpen kernel radius relative to brush size.** Not documented.
  Resolves with: a calibration experiment against CS6 output.
- **Smudge pickup/deposit weighting.** The carried-load ratio is inferred.
  Resolves with: a gradient test against CS6 and fitting the per-dab transfer.
- **Behaviour in Indexed/Bitmap mode.** Whether the tools are disabled or merely
  ineffective is not confirmed. Resolves with: a CS6 mode test.
- **Performance budget for large brushes.** Smudge is reported to be
  resource-heavy; target latencies belong to
  `01-architecture/performance-targets.md`.
