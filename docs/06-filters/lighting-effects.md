# Lighting Effects

- **Spec ID:** `FILT-103`
- **Status:** `Draft`
- **Parity tier:** `Core` (the CS6 revamped workspace), subject to the GPU requirement. On Linux the Mercury GPU path is replaced by wgpu/QRhi; a CPU fallback must exist.
- **New in CS6:** `Yes` — **verified**. CS6 replaced the older Lighting Effects dialog with a new **64-bit "Lighting Effects gallery"** — a dedicated workspace with on-canvas controls and previews. The CS6 What's-New section credits the revamp with better performance and results and highlights the 64-bit gallery's dedicated workspace with its on-canvas controls and live preview. The CS6 Help corpus contains both a **`Add Lighting Effects (CS6)`** topic and a legacy **`Add Lighting Effects (CS5)`** topic; the difference is the workspace/UI, and the revamp is CS6 — **not** CC.
- **Depends on:** `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/document-model.md`, `01-architecture/undo-history.md`, `05-layers/smart-filters.md`, `07-color-painting/color-models.md`, `04-image-ops/image-modes.md`, `02-ui-ux/panels/channels-panel.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. Adobe's lighting/reflection model is closed; those parts
> are **behavioral parity only, algorithm TBD**.

## CS6 behavior

`Filter > Render > Lighting Effects` produces lighting effects on **RGB images only**, optionally using grayscale texture channels (**bump maps**) to create 3D-like surface relief, and can save custom styles for reuse. **A supported video card is required** (PDF). The filter presents a dedicated workspace instead of a modal dialog.

**Workflow:**
1. Choose `Filter > Render > Lighting Effects`.
2. From the **Presets** menu (upper left), choose a style.
3. In the preview window, select a light; in the upper half of the **Properties** panel choose the light type (`Spot`, `Infinite`, or `Point`) and adjust **color**, **intensity**, and **hotspot size**. `Alt`/`Option`-drag a light to duplicate it.
4. In the lower half of the **Properties** panel adjust the whole rig: `Colorize`, `Exposure`, `Gloss`, `Metallic`, `Ambience`, and `Texture`.

**Lights:** at least one light is required; **only one light can be edited at a time**, but all added lights contribute. Up to **16 lights**. Add lights from the upper-left `Lights` icons; delete by dragging a light to the Trash icon in the **Lights** panel (lower right by default).

**Light types:**
- `Point` — shines in all directions from directly above, like a light bulb.
- `Infinite` — shines across an entire plane, like the sun.
- `Spot` — casts an elliptical beam; the line defines direction/angle and the handles define the ellipse.

**On-canvas adjustment:** Point — drag to move, drag the white arc of the Intensity ring to change spread. Infinite — drag the end handle to change direction, the Intensity ring to change brightness. Spot — drag inside the outer ellipse to move, beyond it to rotate, the inner ellipse edge to change the hotspot angle, the four outer handles to extend/shrink, and the Intensity ring to fill more/less of the ellipse. Intensity: 100 is brightest, ~50 is normal, negative removes light, −100 produces no light.

**Presets / styles:** the Presets menu offers **17 light styles** (the PDF enumerates 16 menu entries because `Five Lights Down/Five Lights Up` is one entry covering two directions). Documented styles: `2 o'clock Spotlight`, `Blue Omni`, `Circle Of Light`, `Crossing`, `Crossing Down`, `Default`, `Five Lights Down/Five Lights Up`, `Flashlight`, `Flood Light`, `Parallel Directional`, `RGB Lights`, `Soft Direct Lights`, `Soft Omni`, `Soft Spotlight`, `Three Down`, `Triple Spotlight`. The PDF lists representative intensity/focus values per style (e.g. `Default` = white spotlight, medium intensity 35, wide focus 69). Custom presets: choose `Custom`, add lights, click `Save`, name and confirm; saved presets include all per-light settings and appear in the Style menu for that image; `Delete` removes a preset.

**Texture channels (bump maps):** add a grayscale image as an alpha channel (or use the image's `Red`, `Green`, or `Blue` channel), then choose it from the `Texture` menu in the Properties panel and drag the **Height** slider from `Flat (0)` to `Mountainous (100)`. A white-on-black channel gives an embossed-text effect.

**Restriction:** Lighting Effects cannot be applied to a layer with no pixels (the PDF notes this for filters such as Lighting Effects).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Render > Lighting Effects` | Menu | — | RGB only; GPU required |
| Workspace — Presets menu | Menu/combo | — | 17 light styles + Custom |
| Workspace — preview | Canvas | — | Select/drag lights; on-canvas controls |
| Workspace — Lights icons (upper left) | Toolbar | — | Add Point/Spot/Infinite; max 16 |
| Workspace — Properties panel (upper) | Panel | — | Light type, color, intensity, hotspot |
| Workspace — Properties panel (lower) | Panel | — | Colorize, Exposure, Gloss, Metallic, Ambience, Texture |
| Workspace — Texture menu / Height | Control | — | Bump-map channel + Flat(0)…Mountainous(100) |
| Workspace — Lights panel (lower right) | Panel | drag to Trash | Manage/delete lights |
| Workspace — Save / Delete | Buttons | — | Save/delete a custom preset |
| Workspace — OK / Cancel | Buttons | — | Commit/discard |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Preset / Style | enum | Default | 17 styles + Custom | Per-image style storage |
| Light type | enum | Spot | Spot / Infinite / Point | Edited one at a time |
| Light count | int | 1 | 1…16 | At least one required |
| Light color | color | style-defined | any color | Per light |
| Intensity | numeric | style-defined (Default 35) | −100…100 | 100 brightest, ~50 normal, −100 none |
| Hotspot / focus | numeric | style-defined (Default 69) | 0…100 *(community)* | Spot/Point spread |
| Light position/direction | on-canvas handles | — | — | Point/Spot/Infinite differ |
| Colorize | bool (+ color) | off *(unverified)* | on/off | Tints overall lighting |
| Exposure | slider | 0 | approx −100…+100 *(community)* | Highlight/shadow detail |
| Gloss | slider | 0 | approx −100…+100 *(community)* | Surface reflectivity |
| Metallic | slider | 0 | approx −100…+100 *(community)* | Light vs object reflectivity |
| Ambience | slider | 0 | −100…100 | 100 = light only, −100 = remove source |
| Texture | enum | None | alpha / Red / Green / Blue channels | Bump map |
| Height | slider | 0 | 0 (Flat) … 100 (Mountainous) | Bump-map relief |

Intensity/Ambience semantics are sourced; other numeric bounds not tabulated in the PDF are marked *(community/unverified)* and listed under `## Open questions`.

## Algorithms & pipeline

*(inferred / behavioral parity only, algorithm TBD unless noted.)* Adobe's lighting model is closed; the following is the standard bump-map shading model consistent with the documented controls.

1. **Surface normals from a bump map** — the selected `Texture` channel is read as a height field; per-pixel normals are derived from its gradient, scaled by `Height`. With no texture, the surface is flat. *(inferred)*
2. **Per-light contribution** — for each light, compute the incident direction (Point: from a position; Infinite: a fixed direction; Spot: a position + axis + cone/ellipse and inner hotspot) and the diffuse term via `N·L`, attenuated by intensity (and for Spot, by the hotspot falloff). *(inferred)*
3. **Specular / material** — `Gloss` and `Metallic` parameterize a reflection lobe (glossier = tighter highlight; metallic shifts the balance between light and object reflectivity). The exact BRDF is closed. *(inferred)*
4. **Rig composition** — `Colorize` tints the overall lighting; `Exposure` maps the composed lighting; `Ambience` blends the light contribution with a uniform ambient term (100 = light only, −100 = ambient only). *(inferred)*
5. **Composite** — the shaded lighting is combined with the layer's image (typically a multiply/overlay-style combination on the image's color). The "3D-like preview" is a per-pixel shading result, **not** 3D geometry.

The **preview is GPU-composited** in CS6 (the 64-bit gallery). For this project the shading pass should run in wgpu/QRhi over a tiled image with a CPU fallback. Rendering must be deterministic so preview and commit agree.

## Kooka implementation

Built: the workspace (`LightingEffectsDialog`) with its options bar, Presets
(the 17 styles plus Custom), add Spot / Point / Infinite, Reset, and Preview;
on-canvas controls for all three light types, including the Intensity ring and
Alt-drag duplicate; the Properties panel; and the Lights panel with an eye per
light and a trash button. Up to 16 lights. The engine
(`pictura-filters` `render/lighting.rs`) shades the rig with photorust's
ambient + `N·L` diffuse + Blinn model. It treats Intensity ≈ 50 as normal, uses
an elliptical Spot whose hotspot ellipse lies toward the aimed end, and keeps
every size a fraction of the half-diagonal so the workspace proxy matches the
commit. The preset placements and the light geometry are tuned by eye against
CS6. Only each style's colours, intensities, and focus are from the Help.

Not built: saving and deleting custom presets, alpha channels as Texture, the
RGB-only and GPU gates, and a GPU shading path.

## Rust module mapping

- `pictura_lighting::Light` — enum `Point { pos, intensity, color }`, `Spot { pos, axis, ellipse, hotspot, intensity, color }`, `Infinite { direction, intensity, color }`; on-canvas transform operations.
- `pictura_lighting::LightRig` — `Vec<Light>` (≤16), `Colorize`, `Exposure`, `Gloss`, `Metallic`, `Ambience`; `shade(&mut TileStore)`.
- `pictura_lighting::BumpMap` — height field from an alpha or color channel; `Height` scaling; normal derivation.
- `pictura_lighting::Preset` — named style = full light list + rig settings; load/save/delete; the style list.
- `pictura_lighting::gpu` — wgpu shading pass + CPU fallback; cache keyed by `(opaque, rig, texture_channel, backend)`.
- `pictura_filter::lighting_effects` — filter entry point; `supported(mode, depth)`; commits one history step; participates in the Smart Filter stack (`LAY-021`).

Crossing types: `LightId`, `Light`, `LightRig`, `ChannelRef`, `BumpMap`, `TileStore`, `Backend`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LightingEffectsDialog` | `QDialog` (dedicated workspace) | Presets menu, Lights icons, Properties panel, Lights panel, OK/Cancel |
| `LightingCanvas` | `QQuickWidget`/`QRhiWidget` | Preview and on-canvas light handles (move/rotate/hotspot/intensity) |
| `LightHandles` | `QQuickItem` | Point/Infinite/Spot gizmos, `Alt`-drag duplicate |
| `LightingPropertiesPanel` | `QWidget`/`QStackedWidget` | Upper (type/color/intensity/hotspot) + lower (Colorize/Exposure/Gloss/Metallic/Ambience/Texture) |
| `LightsPanel` | `QListView` + `QAbstractListModel` | Light list, add/delete (drag to Trash) |
| `LightingPresetModel` | `QAbstractListModel` | 17 built-ins + user presets; Save/Delete |
| `TextureChannelSelector` | `QComboBox` + `HeightSlider` | Alpha/R/G/B channel + Flat…Mountainous |

Same widgets-vs-QML split as `FILT-102`: dense panels as Widgets, interactive canvas as QML/QtQuick for GPU-smooth handles.

## Data-model impact

- Non-destructive path: as a **Smart Filter** entry on a Smart Object; otherwise destructive on the raster layer. The `LightRig` (lights + rig settings + texture-channel reference + `Height`) is the serialized parameter set.
- **Presets/styles** are user data: built-in styles ship with the app; custom presets are saved (per the PDF they appear in the Style menu "whenever you open the image", suggesting image-level storage) and may also be stored in a preset library. Determine whether they live in the document or in preferences — see `## Open questions`.
- Texture references point at an alpha channel or the image's R/G/B channel; deleting/renaming the channel invalidates the reference and must be handled.
- Undo: the whole lighting session is one history step on commit; dialog edits re-render without history entries.
- No PSD key change beyond the generic filter/smart-filter serialization (`LAY-021`, `document-model.md`).

## Edge cases

- **RGB only** — CMYK, Lab, Grayscale, Bitmap, Indexed, Multichannel must be gated (PDF: "works only on RGB images").
- **Empty layer** — the filter cannot run on a layer with no pixels; disable/warn.
- **GPU required** — the CS6 filter requires a supported video card; on Linux define the wgpu path and a CPU fallback, with the warning if neither is available.
- **Bit depth** — Lighting Effects is not listed among the 16-bit or 32-bit capable filters in the CS6 Help; likely 8-bit-only. Verify and gate.
- **Texture channel missing** — if the selected channel is deleted or the document has no alpha channel, degrade to no bump map with a clear message.
- **16-light cap** — prevent adding a 17th light; the UI must reflect the limit.
- **Spot hotspot/extents** — degenerate hotspot (0/100) must not produce NaN normals or artifacts.
- **Negative intensity / Ambience extremes** — `−100` intensity and `±100` Ambience must be well-defined (no light / ambient only).
- **Colorize + per-light color interaction** — order of operations must match CS6 appearance.
- **Smart Filter** — verify Lighting Effects is allowed as a Smart Filter (it is not in the CS6 exclusion list of Extract/Liquify/Pattern Maker/Vanishing Point) and show the warning icon for unsupported mode/depth.
- **Large documents / PSB** — shade in tiles; avoid full-resolution float buffers.
- **Undo/redo** — Cancel restores exactly; OK is one reversible step.
- **Determinism** — on-canvas preview must match the committed render.

## Parity acceptance criteria

- Given an RGB image, `Filter > Render > Lighting Effects` opens the dedicated workspace (CS6 gallery), not the old modal dialog.
- Given `Default` preset, the visible light rig matches the documented style (white spotlight, medium intensity 35, wide focus 69) within tolerance.
- Given a `Point`/`Infinite`/`Spot` light, the on-canvas controls move/spread/rotate the light as described, and only one light is editable at a time while all lights contribute.
- Given 16 lights, a 17th cannot be added; deleting a light via the Lights panel removes its contribution.
- Given an alpha channel used as `Texture`, raising `Height` from 0 to 100 increases apparent relief monotonically.
- Given `Ambience = 100`, only the light source contributes; `Ambience = −100` removes it.
- Given `Colorize` on, the overall lighting is tinted.
- Given a CMYK/grayscale document, the filter is unavailable/gated; given a Smart Object with an unsupported mode/depth, the Smart Filter warning icon appears.
- Given an empty layer, the filter is disabled.
- Given `Save` of a custom preset, it appears in the Style menu for the image and reproduces the same rig.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — CS6 Help corpus (downloaded, `pdftotext -layout`). Established: the CS6 What's-New entry for the **Lighting Effects gallery** (better performance and results, a new 64-bit gallery, and a dedicated workspace with on-canvas controls and previews); the coexistence of `Add Lighting Effects (CS6)` and `Add Lighting Effects (CS5)` topics; `Filter > Render > Lighting Effects`; RGB-only and supported-video-card requirement; the apply workflow; light types Point/Infinite/Spot and their on-canvas adjustments; intensity semantics (100 brightest, ~50 normal, negative removes, −100 none); the upper Properties controls (type, color, intensity, hotspot; `Alt`-drag duplicate) and lower rig controls (Colorize, Exposure, Gloss, Metallic, Ambience, Texture); 17 light styles and the enumerated names/representative values; max 16 lights, one edited at a time; add/delete via Lights panel; custom preset save/delete; bump-map texture channels (alpha or R/G/B) and `Height` Flat(0)–Mountainous(100); the note that Lighting Effects (among others) cannot be applied to layers with no pixels; the 16-/32-bit capable-filter lists (Lighting Effects absent).

Marked inferred/community (not primary): the bump-map normal/BRDF composition model, numeric bounds not in the PDF, the exact preview→commit pipeline, and the persistence location of custom presets.

## Open questions

- **Confirmed version.** The revamp is **CS6** per the CS6 What's-New and the `(CS6)`-labelled Help topic; cross-check any conflicting CC-era documentation during review.
- **GPU requirement vs fallback.** Does shipped CS6 refuse to run without a supported GPU, or fall back to CPU? Resolve with a build test; the project must define its own fallback.
- **Bit depth.** Confirm Lighting Effects is 8-bit-only (it is absent from the CS6 16-/32-bit capable lists). Resolve by testing 16-bit RGB in CS6.
- **Exact lighting model.** The diffuse/specular/ambient composition and `Gloss`/`Metallic` mapping are closed. Resolve with reference renders and tolerances, or accept behavioral parity only.
- **Preset count.** The PDF says 17 styles but enumerates 16 menu entries (one entry covers two directions). Confirm the canonical count and exact per-style parameters.
- **Preset storage.** Are custom presets stored in the image, a preset file, or preferences? The PDF says they "appear in the Style menu whenever you open the image". Resolve with a CS6 test.
- **Colorize semantics.** Whether `Colorize` multiplies, tints, or blends the rig, and its default color.
- **Smart Filter compatibility.** The exclusion list omits Lighting Effects; confirm it is smart-compatible and serializes as a generic filter effect.
- **Performance budget.** Interactive on-canvas lighting over a multi-megapixel image needs a latency target; define it in `01-architecture/performance-targets.md`.
- **Post-CS6 drift.** Exclude any CC-era Lighting Effects changes (if any) from parity.
