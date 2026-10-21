# Info Panel

- **Spec ID:** `PAN-013`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the panel's readout grid, options dialog, and sampler block are unchanged. CS6 changes are contextual: before/after readouts are now driven by the **Properties panel** (replacing the CS5 Adjustments panel), and the Eyedropper's **Sample** options gained "ignore adjustment layers" and "current layer and below" (`TOOL-015`).
- **Depends on:** `CLR-001` color-models, `CLR-004` histogram-and-info, `ARCH-003` qt6-ui-design, `ARCH-007` color-management, `ARCH-008` document-model, `03-tools/eyedropper-color-sampler-ruler.md`, `03-tools/note-and-count.md`, `02-ui-ux/preferences.md`.

> This document owns the **Info panel UI surface**. The readout pipeline,
> samplers' data model, and the Histogram panel are specified together in
> `CLR-004`; the sampling tools are `TOOL-015` (Eyedropper), `TOOL-016` (Color
> Sampler), `TOOL-017` (Ruler). All crate, module, widget, and type names are
> **design proposals**. The repository contains a working implementation; this
> file stays the behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not. Facts
> not confirmed by a fetched CS6 source are marked *(inferred)*.

## CS6 behavior

`Window > Info` (shortcut `F8`, which toggles show/hide) opens the **Info panel**. It shows the color values beneath the pointer and, depending on the active tool, other useful information. It also displays a hint for the selected tool, gives document status information, and can show values at 8-bit, 16-bit, or 32-bit precision.

**Default placement.** The Info panel is not part of the default Essentials workspace; it is supplied by the **Photography** workspace (with Histogram and Actions) in the right-hand column (`02-ui-ux/workspace-and-docks.md`, `UI-003`).

### Readout grid

The panel is a compact two-column grid. Its contents are **contextual to the active tool/gesture** (full list in `CLR-004`):

| Context | Readout |
|---|---|
| Default pointer | Numeric color values beneath the pointer at 8/16/32-bit precision |
| CMYK display | An **exclamation point** beside the CMYK values when the sampled color is out of the printable CMYK gamut |
| Marquee tools | x/y pointer position and width (W) / height (H) while dragging |
| Crop / Zoom | W/H of the marquee and the crop marquee's angle of rotation |
| Line / Pen / Gradient / moving a selection | Start x,y, change in X (DX), change in Y (DY), angle (A), length (D) |
| 2-D transform | Percentage change in width (W) and height (H), angle (A), horizontal (H) or vertical (V) skew |
| Any color adjustment (e.g. Curves) | Before-and-after values beneath the pointer and beneath color samplers (fed by the Properties panel in CS6) |
| Show Tool Hints on | A hint for the selected toolbox tool |
| Status information on | Selected document/status fields (see below) |

The panel's upper rows show the **First** and **Second** color readouts; the middle rows show the geometry readouts for the active gesture; the **lower half** shows the up-to-four **color samplers**; the **bottom** shows status information and the tool hint.

### Icons and menus

- Click the **eyedropper icon** to change color readout modes (and to select 8/16/32-bit; the 32-bit option lives on this pop-up).
- Click the **crosshair icon** (cursor coordinates icon in the Help text) to change the **unit of measurement**; changing the units on the Info panel also changes the units on the rulers.
- The **panel menu** (upper-right triangle) provides **Panel Options** and the **Color Samplers** toggle.

### Panel Options dialog

`Panel Options` opens the **Info Panel Options** dialog:

- **First Color Readout** — **Actual Color** (current image mode), **Proof Color** (output color space), **a color mode**, **Total Ink** (sum of all CMYK ink % at the pointer, "based on the values set in the CMYK Setup dialog box"), or **Opacity** (current layer; "does not apply to the background").
- **Second Color Readout** — the same set of options.
- **Ruler Units** — a unit of measurement.
- **Status information** checkboxes:
  - **Document Sizes** — left number = flattened/printing size, right = size including layers and channels.
  - **Document Profile** — the image's color profile name.
  - **Document Dimensions** — image dimensions.
  - **Scratch Sizes** — left = memory used to display all open images, right = total RAM available.
  - **Efficiency** — % of time spent processing vs scratch read/write; below 100% means scratch is in use.
  - **Timing** — time to complete the last operation.
  - **Current Tool** — active tool name.
  - **Measurement Scale** — the document's measurement scale.
- **Show Tool Hints** — a hint for the selected tool at the bottom of the panel.

### Color samplers

Up to **four** samplers can be placed in an image (`TOOL-016`); they are saved in the image and survive close/reopen. They are placed with the Color Sampler tool or `Shift`-click with the Eyedropper, with a **sample size** chosen in the options bar (Point Sample or an N×N average). Readings appear in the Info panel; samplers can be moved, deleted (drag out, `Alt`/`Option`-click with scissors, or **Clear** in the options bar), shown/hidden via `View > Extras` and the panel's **Color Samplers** toggle, and each sampler's color space changed from its icon in the panel.

### Status bar

Separate from the panel, the document window's **status bar** (`CLR-004`, `03-tools/note-and-count.md`) shows the same class of information (magnification, file size, tool instructions) and shares the `Measurement Scale` and `32-bit Exposure` items; the panel must stay consistent with it.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Info` | Panel | `F8` | Toggles the Info panel |
| Panel eyedropper icon | Menu button | click | First/Second readout mode + 8/16/32-bit |
| Panel crosshair icon | Menu button | click | Ruler units (also changes rulers) |
| Panel menu | Menu | — | Panel Options; Color Samplers toggle |
| Info Panel Options | Dialog | — | Readout modes, ruler units, status checkboxes, tool hints |
| Color sampler row | Row + icon menu | click icon | Per-sampler color space; shows value/exclamation |
| Color Sampler tool | Tool | — | Place up to 4 samplers |
| Eyedropper + `Shift`-click | Gesture | `Shift`+click | Adds a sampler (`TOOL-015`) |
| Options bar (Color Sampler) | Controls | — | Sample Size; Clear |
| `View > Extras` | Toggle | `Ctrl/Cmd+H` | Show/hide sampler marks |
| `Image > Adjustments` | Dialog | — | Before/after readouts for the dialog pixels |
| Properties panel | Panel | — | CS6 adjustment context for before/after (`CLR-004`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| First Color Readout | enum | Actual Color | Actual / Proof / a mode / Total Ink / Opacity | Panel Options + eyedropper icon; Actual Color is the CS6 default |
| Second Color Readout | enum | CMYK (RGB documents) | same set | Panel Options + eyedropper icon; CS6 default shows the current mode plus CMYK equivalents |
| Color bit depth | enum | 8-bit *(inferred)* | 8-bit / 16-bit / 32-bit | Eyedropper icon pop-up |
| Ruler Units | enum | pixels *(inferred)* | pixels / inches / cm / mm / points / picas / percent | Crosshair icon |
| Status — Document Sizes | bool | on *(inferred)* | on / off | Panel Options |
| Status — Document Profile | bool | off *(inferred)* | on / off | Panel Options |
| Status — Document Dimensions | bool | off *(inferred)* | on / off | Panel Options |
| Status — Scratch Sizes | bool | off *(inferred)* | on / off | Panel Options |
| Status — Efficiency | bool | off *(inferred)* | on / off | Panel Options |
| Status — Timing | bool | off *(inferred)* | on / off | Panel Options |
| Status — Current Tool | bool | off *(inferred)* | on / off | Panel Options |
| Status — Measurement Scale | bool | off *(inferred)* | on / off | Panel Options |
| Show Tool Hints | bool | off *(inferred)* | on / off | Panel Options |
| Color Samplers visible | bool | on | on / off | Panel menu |
| Color samplers | count | 0 | 0 … 4 | Saved in the image (`TOOL-016`) |
| Sample size | enum | Point Sample | Point / 3×3 / 5×5 / 11×11 / 31×31 / 51×51 / 101×101 | Options bar (`TOOL-015`) |

## Algorithms & pipeline

The readout pipeline is specified in `CLR-004` (`pictura_analysis::readout`). Panel-specific behaviour:

- **Grid slotting** — on each pointer move the panel chooses which readout slots to draw from the contextual `Readout` (color, geometry, status) and clears the slots that do not apply to the active tool.
- **Coalescing** — pointer-move readouts are coalesced to the UI frame rate; the panel must not allocate per event (`CLR-004`).
- **Sampler subscriptions** — each sampler row subscribes to the same readout evaluation at its fixed image coordinate; the panel updates it whenever the document state or proof/profile changes.
- **Gamut flag** — the CMYK exclamation uses the CMYK working-space predicate (`ARCH-007`), identical to the Color panel's alert.
- **Unit conversion** — geometry readouts and sampler coordinates render in the chosen Ruler Units; changing units must not change the underlying values.
- **Status computation** — `Document Sizes`, `Scratch Sizes`, `Efficiency`, and `Timing` are sourced from the engine's status counters, not recomputed by the panel.

## Rust module mapping

- `pictura_analysis::readout` — `Readout { color, geometry, status }`, `ColorReadout { space, precision, cmyk_out_of_gamut }` (`CLR-004`).
- `pictura_analysis::sampler` — `ColorSampler { doc_id, index, point, sample_size, readout_space }`, `DocumentSamplers([Option<ColorSampler>; 4])` (`CLR-004`).
- `pictura_analysis::info_panel` — `InfoPanelModel { first: ReadoutMode, second: ReadoutMode, units: RulerUnits, status_flags: StatusBits, show_tool_hints: bool, samplers_visible: bool }`, `slots_for(tool_state) -> ReadoutSlots`.
- `pictura_core::prefs` — readout defaults, ruler units, status toggles, tool-hints (`02-ui-ux/preferences.md`).
- `pictura_core::command` — sampler place/move/delete routed to `TOOL-016` (not document History; `CLR-004`).

Crossing types: `Readout`, `ReadoutSpace`, `RulerUnits`, `StatusBits`, `ColorSampler`, `DocumentId`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `InfoPanel` | `QDockWidget` | Host; readout grid, icon menus, sampler block, status area |
| `InfoReadoutGrid` | `QGridLayout` | First/Second readout + contextual geometry rows |
| `InfoIconButton` | `QToolButton` with menu | Eyedropper (readout/bit-depth) and crosshair (units) pop-ups |
| `InfoSamplerBlock` | `QWidget` | The up-to-four sampler rows; per-sampler color-space menu |
| `InfoStatusArea` | `QLabel`/`QFormLayout` | Selected status fields + tool hint |
| `InfoOptionsDialog` | `QDialog` | First/Second readout, ruler units, status checkboxes, tool hints |
| `ColorSamplerOverlay` | Canvas overlay | Draws/moves/deletes sampler marks; scissors cursor |
| `ColorSamplerModel` | `QAbstractListModel` | Sampler list + per-sampler space |
| `InfoPanelController` | `QObject` | Subscribes to pointer/tool/state changes; coalesces repaints |

Widgets, not QML: an always-visible, high-refresh instrument panel, consistent with `ARCH-003`. A `QGridLayout` of labels is cheaper and more keyboard/screen-reader friendly than a scene graph; the canvas overlay is shared with `TOOL-016`.

## Data-model impact

- **Color samplers are document data.** Up to four `(point, sample_size, readout_space)` records are saved in the image and must round-trip through PSD/open-save; exact keys *(inferred)* (`CLR-004`, `ARCH-011` file-formats).
- **Panel options are preferences**, not document data: readout modes, ruler units, status checkboxes, tool hints, sampler visibility.
- **Ruler units** propagate to the rulers and other measurement UI (`03-tools/note-and-count.md`).
- **Undo:** placing/moving a sampler or changing a readout is **not** a History state (`CLR-004`; verify against CS6).
- **No new document fields** beyond the sampler records.
- **Status values** are derived engine counters, never serialized.

## Edge cases

- **No open document** — the panel shows a neutral/blank state; color and geometry readouts are suppressed.
- **Pointer outside the canvas** — color readout clears; geometry readouts retain the last gesture value as appropriate.
- **8/16/32-bit** — the 32-bit option is only selectable from the eyedropper menu; 32-bit color values must render at full precision (`CLR-004`).
- **CMYK / Lab / Grayscale / Indexed** — Actual Color follows the document mode; Total Ink applies only to CMYK; the exclamation uses the CMYK working space.
- **Alpha/spot channels** — sampling source per the Eyedropper Sample option (`TOOL-015`).
- **Proof Color without a proof profile** — fall back to Actual Color with a clear state, not a wrong value.
- **N×N sample at the canvas edge** — clamp or reject out-of-bounds pixels consistently with the Eyedropper (`CLR-004`).
- **Four samplers already placed** — a fifth placement is refused; **Clear** removes all four.
- **Sampler on a deleted layer** — reading must not crash; the value source follows the sample option.
- **Huge (PSB) documents** — sampler/readout evaluation must stay per-pixel and never scan the canvas.
- **High-frequency pointer moves** — coalesce repaints; avoid layout thrash from variable-width labels.
- **Undo/redo** — after undo the pointer readout and samplers must reflect restored pixels.
- **High contrast / dark theme** — the out-of-gamut exclamation and sampler marks must remain legible.
- **Keyboard-only** — panel options, bit-depth, units, and sampler controls must be reachable without a pointer.

## Parity acceptance criteria

1. Given the Info panel with the pointer over the canvas, the First Color Readout shows values in the chosen mode at the chosen 8/16/32-bit precision.
2. Given a CMYK readout over an out-of-gamut pixel, an exclamation appears beside the CMYK values.
3. Given a marquee drag, the panel shows x/y and W/H; given a 2-D transform, it shows percentage W/H, angle, and H/V skew.
4. Given the **Panel Options** dialog, choosing **Total Ink** for the First readout shows the summed CMYK ink percentage at the pointer.
5. Given Ruler Units changed from the crosshair icon or Panel Options, geometry readouts and the document rulers both change units.
6. Given **Show Tool Hints** enabled, the panel shows a hint for the selected toolbox tool.
7. Given four color samplers placed, their readings persist across save/close/reopen; a fifth placement is refused and **Clear** removes all four.
8. Given a sampler's color space changed from its icon, that row reads in the new space while the others are unchanged.
9. Given `View > Extras` toggled off or the **Color Samplers** panel toggle off, sampler marks disappear but their readings remain available.
10. Given an adjustment dialog with Preview on, before-and-after values appear for the pointer and samplers, and revert on cancel.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded and text-extracted). Established: `Window > Info` and `F8`; the panel purpose and 8/16/32-bit readouts; the CMYK exclamation; the contextual readout list (marquee, crop/zoom + rotation angle, line/pen/gradient/moving selection with x,y/DX/DY/A/D, 2-D transform W/H/A/H/V skew, adjustment before/after, tool hints, status information); the three ways to set options (Panel Options, eyedropper icon, crosshair/cursor-coordinates icon); the Info Panel Options dialog (First/Second Color Readout with Actual Color, Proof Color, a color mode, Total Ink, Opacity; Ruler Units; the full status-information list; Show Tool Hints); "Keys for the Info panel" (eyedropper and crosshair icons); the up-to-four color samplers, their save-in-image behavior, sample sizes, move/delete/hide operations, per-sampler color space, and the panel's Color Samplers toggle; the rule that changing the Info panel's units also changes the rulers.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — same file, "Productivity enhancements (JDI's) in CS6 → Eyedropper": the added Sample options (ignore adjustment layers; current layer and below) and sample-size context menus that feed the Info readouts.

Not used in this pass:

- `helpx.adobe.com` Info-panel pages (HTTP 403 from this environment); the archived CS6 PDF and `CLR-004` were used instead.

Fetched for this revision:

- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Info_palette.html` — Martin Evening CS6/CS5 support page: by default the color display shows pixel values for the selected color mode plus the CMYK equivalents; the out-of-gamut exclamation beside the CMYK value; status checkboxes appear in the middle of the panel and `Show Tool Hints` below; sample readouts follow the Eyedropper's sample-area size; the 16-bit readout values range 0–32,768 (15 bits used).
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Photography workspace supplies the Info panel (with Histogram and Actions), not the default Essentials workspace.

Consulted as search-result snippets only (not individually fetched): `http://www.colormanagementinfo.com/Articles/Advanced_Photoshop_Color_Settings` ("The info palette's secondary readout is set to CMYK by default") and the Adobe community answers thread confirming the First readout defaults to Actual Color.

## Open questions

- **Exact default Ruler Units and status toggles** (pixels assumed; Document Sizes shown by default). *Resolves with:* a CS6 first-run Info panel capture.
- **Exact 16-bpc/32-bpc readout formats** and whether 32-bit shows more than three channels. *Resolves with:* a CS6 32-bpc capture.
- **Whether sampler/readout changes are undoable** in CS6. *Resolves with:* a CS6 experiment.
- **Sampler PSD serialization keys.** *Resolves with:* PSD inspection and `ARCH-011` file-formats.
- **Whether a sampler can be renamed or reordered.** *Resolves with:* a CS6 capture.
- **Status-counter semantics** (Efficiency/Timing source quantities). *Resolves with:* `ARCH-001` system-architecture and engine metrics.
