# Extract and Pattern Maker (Legacy / Removed Filters)

- **Spec ID:** `FILT-104`
- **Status:** `Draft`
- **Parity tier:** `Non-goal (removed; optional plug-ins)`. Documented for completeness and for a decision record.
- **New in CS6:** `No` / `Changed` — **corrections to the task premise:** (1) **Extract was not removed in CS6**; it (and Pattern Maker) were removed from the **default install in CS4 (2008)** and remained available as **optional plug-ins**. (2) **Pattern Maker is not "CS4-only"** — it existed through CS3 and was removed from the default install in CS4, like Extract. In CS6 the Help documents both as optional downloads: Extract is **Windows-only and not installed**, Pattern Maker is available for **Windows or Mac OS** (requiring **32-bit mode on 64-bit Mac**).
- **Depends on:** `08-selection/refine-edge.md` (the supported replacement for Extract), `07-color-painting/pattern-presets.md` (pattern-preset model), `01-architecture/file-formats.md`, `02-ui-ux/preferences.md`, `00-overview/feasibility-and-non-goals.md`, `06-filters/filters-overview.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. This spec is largely a **removal/decision record**; the two
> plug-ins are optional and their algorithms are closed (**behavioral parity only,
> algorithm TBD**).

## CS6 behavior

**Removal history (sourced):**
- Photoshop **CS3 and earlier** bundled Extract and Pattern Maker as standard filters.
- In **CS4**, Adobe removed **Extract** and **Pattern Maker** (along with Web Photo Gallery, Contact Sheet, Picture Package, and PDF Presentation) from the **default installation**. The latter four were replaced by the **Bridge CS4 Output module**; Extract and Pattern Maker were offered as **optional downloads** for the plug-ins folder.
- In **CS6**, neither is installed or present in the Filter menu by default. The CS6 Help documents them as optional plug-ins:
  - **Extract** — not installed with Photoshop; the optional Windows plug-in is a separate download. It is **not available for Mac OS** because it is incompatible with recent macOS versions, and the PDF states **Refine Edge produces better extractions**.
  - **Pattern Maker** — an optional plug-in available as a download for Windows or Mac OS. On 64-bit Mac it requires running Photoshop in **32-bit mode**.

**Extract (when installed).** Erases an object's background to transparency; edge pixels lose background-derived color so they blend without halos. Workflow: duplicate/snapshot the layer; choose `Filter > Extract`; set tool options; highlight the object edge; define the foreground; preview; touch up; `OK`. Optionally `Edit > Fade Extract` afterward.
- Tools: `Edge Highlighter`, `Fill`, `Eraser`, `Cleanup`, `Edge Touchup`, `Eyedropper` (with `Force Foreground`), `Zoom`, `Hand`.
- Options: `Brush Size`, `Highlight` (color), `Fill` (color), `Smart Highlighting` (keeps the highlight just wide enough for a well-defined edge).
- Extraction: `Textured Image` (foreground/background has lots of texture), `Smooth` (outline smoothness; low values avoid blurring detail), `Channel` (base the highlight on a saved alpha channel), `Force Foreground` (sample a foreground color for intricate objects lacking a clear interior).
- `Preview`: `Show` (original vs extracted), `Display` (colored matte / grayscale / `None` = transparent).
- Touch-up: `Cleanup` subtracts/restores opacity cumulatively; `Edge Touchup` sharpens edges and adds/subtracts opacity near the edge.

**Pattern Maker (when installed).** Slices and reassembles an image to make a pattern; either fills a layer/selection or creates tiles saved as pattern presets. Workflow: install and (Mac) run 32-bit; select a layer/area or copy a sample to the clipboard; choose `Filter > Pattern Maker`; choose the source (`Use Clipboard As Sample`, or draw with the `Marquee` tool); set the tile size (`Width`, `Height`, or `Use Image Size`); `Generate`; review; `Generate Again`/adjust; navigate `Tile History`; `OK` to fill or `Cancel` for preset-only.
- Options: `Smoothness` (reduces sharp edges), `Sample Detail` (size of pattern slices in the tile; higher keeps more detail but takes longer).
- Preview: `Show` (generated vs source), `Tile Boundaries`, `Offset` direction + `Amount` (% of tile dimension; does not affect saved preset tiles), `Update Pattern Preview`.
- `Tile History`: `First`/`Previous`/`Next`/`Last` tile, index entry, `Delete`, `Save Preset Pattern` (saves a single tile, not the full generated pattern).

**Other filters and the CS6 Filter menu (accuracy note).**
- The CS6 Help **Filter effects reference** lists these groups/entries: **Artistic, Blur, Brush Stroke, Distort, Noise, Pixelate, Render, Sharpen, Sketch, Stylize, Texture, Video, Other, Digimarc, Vanishing Point**, plus `Filter Gallery`, `Liquify`, `Lens Correction`, and the CS6 additions (`Oil Paint`, `Adaptive Wide Angle`, the **Blur Gallery** of `Field Blur`/`Iris Blur`/`Tilt-Shift`, and the revamped `Lighting Effects`).
- **No filter program was removed from the CS6 Filter menu relative to CS5.** The commonly reported "missing filters" (Artistic, Brush Strokes, Distort, Sketch, Texture) are **hidden by default in CS6/CC/CC2014** and accessible via the Filter Gallery; they can be restored with `Preferences > Plug-Ins > "Show all Filter Gallery groups and names"` (secondary source). This is a UI gating change, not a removal.
- **Oil Paint was removed in CC 2014**, not in CS6 (it shipped in CS6 and is used as a Smart Filter).
- Optional/legacy plug-ins from CS3–CS4 (e.g. Extract, Pattern Maker, Texture Fill, TWAIN, JPEG2000, Bigger Tiles, etc.) may appear at the bottom of the Filter menu when installed.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Extract` | Menu | `Alt+Ctrl+X`* | Only when the optional Windows plug-in is installed |
| Extract — Edge Highlighter / Fill / Eraser | Tools | — | Mark and fill |
| Extract — Cleanup / Edge Touchup / Eyedropper | Tools | — | Touch-up / force-foreground sampling |
| Extract — Brush Size / Highlight / Fill / Smart Highlighting | Options | — | Tool options |
| Extract — Textured Image / Smooth / Channel / Force Foreground | Options | — | Extraction settings |
| Extract — Preview (Show / Display) | Controls | — | Compare / matte |
| `Filter > Pattern Maker` | Menu | `Alt+Ctrl+X`* | Only when the optional plug-in is installed |
| Pattern Maker — source / Marquee / tile size | Controls | — | `Use Clipboard As Sample`, `Width`/`Height`, `Use Image Size` |
| Pattern Maker — Generate / Generate Again | Buttons | — | Iterate |
| Pattern Maker — Tile History | Panel | — | First/Prev/Next/Last, Delete, Save Preset Pattern |
| Pattern Maker — Smoothness / Sample Detail / Offset | Options | — | Generation controls |
| `Preferences > Plug-Ins > Show all Filter Gallery groups and names` | Preference | — | Restores hidden Filter-Gallery groups to the Filter menu |
| `Filter > Oil Paint` | Menu | — | Present in CS6; removed in CC 2014 |

*Shortcuts reflect the legacy CS3-era bindings; not re-verified against CS6.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Extract Brush Size | numeric | — | ≥ 0 | Edge Highlighter/Eraser/Cleanup/Edge Touchup width |
| Extract Highlight / Fill color | color | preset/Other | presets or custom | |
| Extract Smart Highlighting | bool | off *(unverified)* | on/off | Auto-width highlight |
| Extract Textured Image | bool | off | on/off | |
| Extract Smooth | numeric | 0 *(recommended small)* | ≥ 0 *(unverified)* | |
| Extract Channel | enum | — | alpha channels | Becomes `Custom` if modified |
| Extract Force Foreground | bool | off | on/off | Uses Eyedropper color |
| Extract Show | enum | — | Original / Extracted | Preview |
| Extract Display | enum | — | matte color / grayscale / None | Preview |
| Pattern Maker Use Clipboard As Sample | bool | off | on/off | |
| Pattern Maker Width / Height | numeric (px) | — | ≥ 1 | Tile size |
| Pattern Maker Use Image Size | bool | off | on/off | One tile filling the layer |
| Pattern Maker Offset direction / Amount | enum + % | — | direction, % of tile | Not applied to saved preset tiles |
| Pattern Maker Smoothness | numeric | — | *(unverified)* | |
| Pattern Maker Sample Detail | numeric | — | *(unverified)*; higher = more detail/slower | |
| Pattern Maker Update Pattern Preview | bool | on *(unverified)* | on/off | |

Numeric ranges/defaults are not tabulated in the CS6 Help PDF; values marked *(unverified)* require a build check.

## Algorithms & pipeline

*(behavioral parity only, algorithm TBD.)* Both plug-ins are closed legacy code; the following is the observable model only.

- **Extract** — (1) the user paints an `Edge Highlighter` band straddling the object boundary; (2) the interior is flood-filled (or `Force Foreground` samples a single foreground color); (3) an alpha-matte is estimated from the highlight band and interior, with `Smooth` controlling outline regularization and `Textured Image` switching the matte estimator for high-texture cases; (4) `Cleanup` and `Edge Touchup` refine opacity/edges. The band→matte estimator is closed. **Superseded by** the non-destructive `Select > Refine Edge` (`08-selection/refine-edge.md`), which the CS6 PDF explicitly recommends.
- **Pattern Maker** — slice the source into a tile (or tiles) and reassemble matching boundary content into a seamless repeating pattern, iterating a randomized synthesis until the user accepts a tile; `Sample Detail` and `Smoothness` steer the slice size and edge regularization. The synthesis algorithm is closed. **CS6 alternatives:** `Edit > Fill > Pattern > Scripted Patterns` (new in CS6) and the `Pattern Preset` workflow (`07-color-painting/pattern-presets.md`).

Neither plug-in should be ported unless a specific user need is identified; the recommended CS6-parity path is Refine Edge + Scripted Patterns.

## Rust module mapping

Proposed only if the project decides to support the optional plug-ins (see `## Open questions`):

- `pictura_filter::legacy::extract` — *(proposal, likely unused)* edge-band → alpha matte; `ExtractParams`.
- `pictura_filter::legacy::pattern_maker` — *(proposal, likely unused)* tile synthesis; `PatternMakerParams`; `TileHistory`.
- **Default decision:** `None` — do not implement; map users to `pictura_selection::refine_edge` and `pictura_filter::scripted_patterns`.

## Qt6 component mapping

- **Default:** `None` — no UI. The Filter menu omits `Extract` and `Pattern Maker` in CS6 parity.
- If optional support is added: `ExtractDialog` (canvas + tool options + Preview controls) and `PatternMakerDialog` (source/tile controls + `TileHistory` view + Save Preset Pattern), both as `QDialog` with a QQuickWidget canvas.

## Data-model impact

- **None for the default (non-goal) decision.** As optional plug-ins they produce raster results/pattern presets and store no document nodes.
- If implemented: Extract produces an alpha channel/mask on the active layer (mirroring Refine Edge's mask output); Pattern Maker produces pixels and `Save Preset Pattern` adds a `.pat`-style pattern preset. Serialization belongs to `pattern-presets.md` and the preset manager, not a new PSD key.
- The CS6 Filter menu **gating** for hidden Filter-Gallery groups is a preference, not a document property (`02-ui-ux/preferences.md`).

## Edge cases

- **Platform** — Extract is Windows-only in CS6 (macOS incompatible); Pattern Maker needs 32-bit mode on 64-bit Mac. On Linux neither exists, reinforcing the non-goal.
- **Optional absence** — if the plug-ins are absent, `Filter > Extract` / `Filter > Pattern Maker` must not appear; the UI must not crash on their absence.
- **Replacement behavior** — Extract users should be pointed to Refine Edge; Pattern Maker users to Scripted Patterns + pattern presets.
- **Force Foreground vs Textured Image/complete highlight** — the PDF notes the whole object cannot be highlighted if `Textured Image` or `Force Foreground` is selected; the UI must enforce this.
- **Channel option** — unavailable unless the document has an alpha channel; label changes to `Custom` if the highlight is modified.
- **Pattern Maker preset save** — saves only a single tile, not the generated pattern; `Cancel` is the "preset-only" exit.
- **Mode/depth** — legacy plug-in support is unverified; if ported, gate like other filters.
- **Undo** — one history step per application, as with any legacy filter.
- **Filter Gallery hiding** — the hidden groups preference must not be confused with removal; document it so testers do not file false "missing filter" bugs.

## Parity acceptance criteria

- Given a CS6-parity install, `Filter > Extract` and `Filter > Pattern Maker` are **absent** from the Filter menu by default.
- Given the CS6 Filter menu, the documented groups (Artistic, Blur, Brush Stroke, Distort, Noise, Pixelate, Render, Sharpen, Sketch, Stylize, Texture, Video, Other, Digimarc, Vanishing Point) plus `Filter Gallery`, `Liquify`, `Lens Correction`, `Oil Paint`, `Adaptive Wide Angle`, and the Blur Gallery entries are present/available subject to mode/depth.
- Given `Preferences > Plug-Ins > Show all Filter Gallery groups and names` enabled, the hidden Filter-Gallery groups (Artistic, Brush Strokes, Distort, Sketch, Texture) reappear in the Filter menu; disabling hides them again.
- Given a user needing background removal, the documented CS6 path is `Select > Refine Edge`, not Extract.
- Given a user needing a generated pattern, the documented CS6 path is `Edit > Fill > Pattern > Scripted Patterns` / pattern presets, not Pattern Maker.
- Given an optional plug-in is installed, it appears at the bottom of the Filter menu and behaves per the legacy dialogs (only if the project opts in).
- Given a legacy `Save Preset Pattern`, the saved single tile appears in the pattern preset list.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — CS6 Help corpus (downloaded, `pdftotext -layout`). Established: Extract as an optional, **Windows-only**, not-installed plug-in with Refine Edge recommended as superior; the full Extract tool/option set; Pattern Maker as an optional Windows/Mac plug-in requiring **32-bit mode on 64-bit Mac**, with source/tile/Generate/Tile History/`Save Preset Pattern` behavior; the CS6 **Filter effects reference** group list (Artistic, Blur, Brush Stroke, Distort, Noise, Pixelate, Render, Sharpen, Sketch, Stylize, Texture, Video, Other, Digimarc, Vanishing Point); the Smart Filters exclusion list (Extract, Liquify, Pattern Maker, Vanishing Point); the note that Lighting Effects (and others) cannot apply to empty layers; the "Keys for Extract and Pattern Maker (optional plug-ins)" keyboard section.
- `https://www.thegraphicmac.com/photoshop-cs4s-shortcut-changes-and-missing-features` — established that **Extract, Pattern Maker, Web Photo Gallery, Contact Sheet, Picture Package, and PDF Presentation were removed from the CS4 default installation**, with the former four replaced by the Bridge CS4 Output module and optional downloads for the rest.
- `https://planetphotoshop.com/wheres-my-patternmaker.html` — secondary confirmation that Pattern Maker (and Extract) were **removed from Photoshop CS4** and provided as optional downloads.
- `https://digital-photography-school.com/the-mysterious-disappearing-filters-in-photoshop` — established that in CS6/CC/CC2014 the Artistic, Brush Strokes, Distort, Sketch, and Texture groups are **hidden from the Filter menu by default** and restorable via `Preferences > Plug-Ins > "Show all Filter Gallery groups and names"`; that **Oil Paint was removed in CC 2014** (not CS6); and that Filter Gallery vs menu application changes the Smart Filter layer label.
- `https://digital-photography-school.com/adobe-camera-raw-acr-photoshop-filter` — used only to cross-check the CC-only camera-raw-filter claim (see `FILT-100`).

Marked secondary/inferred: the CS4 removal details from blog sources; the exact CS3→CS4 transition for any single filter; the legacy shortcuts; and plug-in internals.

## Open questions

- **Implement at all?** Default is non-goal. Resolve with a `00-overview/feasibility-and-non-goals.md` decision; if yes, prioritise a community-compatible Extract/Pattern Maker as explicit non-parity extensions.
- **Exact removal version.** Sources agree on **CS4**, but some forum claims place Extract's removal at CS3. Resolve with a CS3-vs-CS4 install comparison.
- **Extract macOS.** Confirm macOS support ended when (the CS6 PDF says incompatible with recent macOS). Resolve with CS6 release notes.
- **Pattern Maker 32-bit requirement.** Confirm the exact behavior and whether it worked in 64-bit Windows.
- **Algorithm fidelity.** Both are closed; if ported, choose a modern matting (Refine Edge-class) and texture-synthesis algorithm and define tolerances, or accept behavioral parity only.
- **Filter-menu gating default.** Confirm the CS6 default of hidden Filter-Gallery groups and the exact preference wording.
- **Digimarc and other optional plug-ins.** Decide whether any optional/legacy plug-ins (Digimarc, Texture Fill, TWAIN, JPEG2000) are in scope; likely non-goal.
- **Pattern preset format.** If Pattern Maker/pattern presets are supported, confirm the `.pat` (or equivalent) serialization with `pattern-presets.md`.
- **Keyboard shortcuts.** Legacy `Alt+Ctrl+X` bindings are unverified against CS6; confirm before exposing them.
