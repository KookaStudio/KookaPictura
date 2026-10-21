# Measurement and Count

- **Spec ID:** `WF-014`
- **Status:** `Draft`
- **Parity tier:** `Extended-only` — the entire measurement feature set is marked **(Photoshop Extended)** in the CS6 Help PDF; on a Standard build these commands are absent. The Count tool portion is also Extended-only (`TOOL-019`).
- **New in CS6:** `No` — the Measurement scale, Measurement Log, Data Points, and Count tool are carried from CS5/CS4. CS6 changes no documented measurement behavior.
- **Depends on:** `PAN-023` measurement-log-panel (panel UI), `TOOL-018`/`TOOL-019` note-and-count (Count tool semantics, ruler), `03-tools/eyedropper-color-sampler-ruler.md` (Ruler tool), `ARCH-008` document-model, `ARCH-009` undo-history, `07-color-painting/histogram-and-info.md` (histogram data), `WF-010` (DICOM scale import), `09-automation/rust-scripting-replacement.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Behavior is taken from the CS6 Help PDF unless marked
> *(inferred)*. Column-label and record formatting beyond the documented data
> points are behavioral parity only.

## CS6 behavior

### Overview

The measurement feature (`Measurement (Photoshop Extended)`) measures any area
defined by the **Ruler tool** or a **selection tool** (Marquee, Lasso, Quick
Select, Magic Wand), including irregular areas. It computes height, width,
area, perimeter, and pixel values, and can track measurements across one image
or many. Results are written to the **Measurement Log** panel. The Log's
columns can be reordered, sorted, and deleted, and the data exported.

Three concepts:

- **Measurement scale** — maps a pixel length to logical units (inches, mm,
  microns, miles, …).
- **Scale markers** — on-image rulers that display the scale.
- **Data points** — the selectable columns recorded for each measurement.

### Measurement scale

- `Analysis > Set Measurement Scale` holds the current scale plus presets. The
  current scale is checked in the submenu and shown in the Info panel.
- **Default** is `1 pixel = 1 pixel` (`Set Measurement Scale > Default`).
- **Custom…** opens the Measurement Scale dialog and auto-selects the Ruler
  tool: drag a pixel distance (or type **Pixel Length**), then enter
  **Logical Length** and **Logical Units**. Example: Pixel Length 50, Logical
  Length 1, Units `microns` ⇒ 50 px = 1 micron.
- **Presets**: create with **Save Preset**, delete with **Delete Preset**; a
  preset is added to the `Set Measurement Scale` submenu. Only **one scale per
  document** at a time.
- **Persistence**: `File > Save` stores the current scale with the document
  (PSD resource **1074**, "Measurement Scale"). CS6 Help states:
  "Choose File > Save to save the current measurement scale setting with the
  document."
- **Display**: Info-panel Panel Options can show **Measurement Scale** in the
  Status Information area; the document window menu can show the scale at the
  bottom.
- **DICOM**: any measurement scale in a DICOM file is imported automatically;
  if absent, a default `1 pixel = 1 mm` custom scale is added (`WF-010`).

### Scale markers

- `Analysis > Place Scale Marker` opens the **Measurement Scale Marker**
  dialog: **Length** (in logical units), **Font**, **Font Size**,
  **Display Text**, **Text position** (above/below), **Color** (black/white).
- The marker is placed in the **lower-left** corner and adds a **layer group**
  containing a text layer (if Display Text) and a graphic layer; the Move tool
  moves it and the Type tool edits the caption.
- **Add/replace**: placing another marker prompts **Remove** or **Keep**;
  additional markers stack at the same corner and can obscure each other.
- **Delete**: select the Measurement Scale Marker layer group and delete the
  group and contents.

### Performing a measurement

1. Open a document (optionally set a measurement scale first).
2. `Analysis > Select Data Points` → **Custom**, or a data-point preset. The
   dialog groups data points by measurement tool; **Common** points apply to
   all tools (file name, scale, date/time). **All data points are selected by
   default.** Save subsets as presets.
3. Choose the tool matching the data points:
   - **Selection tools** — one or more selections (area, perimeter, height,
     width, gray values).
   - **Ruler tool** (`Analysis > Ruler Tool`) — length and angle.
   - **Count tool** (`Analysis > Count Tool`) — item counts (`TOOL-019`).
4. `Window > Measurement Log` opens the Log panel.
5. `Analysis > Record Measurements` (or the Log's **Record Measurements**
   button) appends the measurement. If the selected data points don't match the
   current tool, you are asked to adjust them.
6. Repeat. Measurements are computed and recorded in the scale units in effect
   **at recording time**; with no scale, the default `1 px = 1 px` applies.

**Multiple selections**: one row is created with summary/cumulative data for
all selected areas, followed by one row per selection; each selection is a
distinct **Feature** with a unique number. The **Document** column records the
source document, so a Log can span multiple open documents.

### Measurement data points (CS6 Help list)

| Data point | Meaning |
|---|---|
| Angle | Ruler orientation, ±0–180 |
| Area | Selection area in square pixels or calibrated square units |
| Circularity | 4π·area/perimeter²; 1.0 = perfect circle; invalid for very small selections |
| Count | Selection tool: number of discontiguous areas; Count tool: items counted; Ruler: number of lines (1 or 2) |
| Date and Time | Timestamp of the measurement |
| Document | Source file |
| Gray Value | Brightness 0–255 (8-bit), 0–32,768 (16-bit), 0.0–10 (32-bit); image internally converted to grayscale; mean/median/min/max |
| Height | max y − min y, in scale units |
| Histogram | Per-channel histogram, 0–255 (16/32-bit converted to 8-bit); exported to **CSV** |
| Integrated Density | Sum of selected pixel values; = Area × Mean Gray Value |
| Label | Auto "Measurement 1, 2, …"; each simultaneous selection gets an extra Feature label |
| Length | Ruler distance in scale units |
| Perimeter | Selection perimeter; multiple selections yield total + per-selection values |
| Scale | Source document scale, e.g. `100 px = 3 miles` |
| Scale Units | Logical units of the scale |
| Scale Factor | Pixels assigned to the scale unit |
| Source | Ruler tool, Count Tool, or Selection |
| Width | max x − min x, in scale units |

Only data points associated with the measuring tool appear in the Log, plus any
selected **Common** points.

### Using the Measurement Log

- Each Log **row** is a measurement set; **columns** are the data points.
- The Log supports reordering columns, sorting, deleting rows/columns, and
  **export to a tab-delimited Unicode text file**. **Histogram** data exports to
  a **CSV** file in its own folder beside the text export, numbered from 0.
- **Recording does not mutate on-image counts**, and clearing on-image counts
  does not change Log rows (`TOOL-019`).
- The Log is append-only in practice; repeated recording adds rows.

### Count tool relationship

The Count tool (`TOOL-019`, Extended-only) counts objects into named **count
groups** with color, marker size (1–10) and label size (8–72). **Automatic
counting** uses Magic Wand or `Select > Color Range` to define areas, then
records the number of discontiguous areas to the Measurement Log as the **Count**
data point. Count numbers and groups persist in the saved PSD (resource
**1080**, "Count Information"). See `TOOL-018`/`TOOL-019` and `PAN-023` for the
tool and panel semantics; this spec owns the **data scale/log/export** side.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Analysis > Set Measurement Scale > Default | Menu | — | 1 px = 1 px |
| Analysis > Set Measurement Scale > Custom… | Dialog | — | Ruler auto-selected |
| Analysis > Set Measurement Scale > <preset> | Menu | — | Saved scales |
| Analysis > Place Scale Marker | Dialog | — | Length/font/position/color |
| Analysis > Select Data Points > Custom… | Dialog | — | Common + per-tool groups |
| Analysis > Select Data Points > <preset> | Menu | — | Saved data-point sets |
| Analysis > Record Measurements | Menu | — | Append to Log |
| Analysis > Ruler Tool | Menu | — | Selects Ruler |
| Analysis > Count Tool | Menu | — | Selects Count |
| Window > Measurement Log | Panel | — | Table of records |
| Measurement Log — Record Measurements | Button | — | Same as Analysis menu |
| Measurement Log — export | Button/menu | — | Tab-delimited text; CSV histograms |
| Info panel — Measurement Scale | Field | — | Shown via Panel Options |
| Document window menu — Show > Measurement Scale | Menu | — | Bottom-of-window readout |
| Layers panel — Measurement Scale Marker group | Layer group | — | Movable/editable marker; delete group |
| Ruler tool + Info | Tool | — | Length/angle readout (`TOOL-018` group) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Measurement Scale | Struct | 1 px = 1 px | Pixel Length, Logical Length, Logical Units | One per document |
| Logical Units | Enum/free | pixels | inches, mm, microns, miles, … *(list not enumerated in PDF)* | |
| Scale presets | Named list | none | user-defined | Stored in Analysis submenu |
| Scale Marker Length | Number | 1 *(unverified)* | in logical units | |
| Scale Marker Font | Font | Helvetica *(unverified)* | any installed | |
| Scale Marker Font Size | Number | 12 *(unverified)* | >0 | |
| Scale Marker Display Text | Bool | Off *(unverified)* | on/off | |
| Scale Marker Text Position | Enum | Below *(unverified)* | above / below | |
| Scale Marker Color | Enum | Black *(unverified)* | black / white | |
| Data points | Bool set | All | Common + tool groups | Saved as presets |
| Count data point | Bool | All | selection count / count-tool items / ruler lines | |
| Gray Value depth | Enum | from mode | 8-bit 0–255; 16-bit 0–32,768; 32-bit 0.0–10 | internal grayscale |
| Histogram bins | Integer | 256 | 0–255 (8-bit) | 16/32-bit down-converted |
| Scale Factor | Read-out | 1 px/unit | computed | |
| Log export | File | — | tab-delimited Unicode text | |
| Histogram export | File | — | CSV | own folder, numbered from 0 |

## Algorithms & pipeline

1. **Scale.** A measurement scale is `(pixel_length, logical_length,
   units)`; `scale_factor = pixel_length / logical_length`. Conversion:
   `logical = pixels / factor` (and `logical² = pixels² / factor²` for area).
2. **Measure.**
   - *Selection*: rasterize the selection mask; `Area` = count of set pixels;
     `Perimeter` = boundary length (pixel-edge tracing) with the standard
     convention; `Height`/`Width` = bounding-box extents; `Gray Value` from a
     grayscale conversion of the active channel (mean/median/min/max).
   - *Ruler*: `Length` = Euclidean distance; `Angle` = orientation ±0–180.
   - *Count*: selection tool = connected components (discontiguous areas);
     Count tool = placed marks; Ruler = visible lines.
   - `Circularity = 4π·Area / Perimeter²`; `IntegratedDensity = Area · MeanGray`.
   - `Histogram` = per-channel 256-bin counts (down-convert 16/32-bit).
3. **Record.** Copy the enabled data points for the current measurement into a
   new Log row (plus one row per Feature when several selections exist). Values
   are frozen; later edits to the image do not change Log rows.
4. **Export.** Serialize rows as UTF-8 tab-delimited text; write each Histogram
   to a numbered CSV in a sibling folder.
5. **Persist.** The measurement scale is saved with the document (PSD resource
   1074); count groups/marks save to resource 1080 (`TOOL-019`).
6. **Scale markers** are ordinary document content (a layer group), not
   analysis state.

Pixel-edge/perimeter conventions and connected-component connectivity (4- vs
8-neighbour) are **behavioral parity only, algorithm TBD**; the CS6 Help does
not define them (`TOOL-019` open question).

## Rust module mapping

Proposals:

- `pictura_analysis::scale` — `MeasurementScale { pixel_len: f64,
  logical_len: f64, units: Unit }`, `factor()`, `to_logical`,
  `to_logical_area`; `ScalePreset` store (global).
- `pictura_analysis::marker` — `ScaleMarker` creation: emits a document layer
  group (text + graphic layer) via the layer model.
- `pictura_analysis::measure` — `Measurement { source: Source, values:
  BTreeMap<DataPoint, Value> }`; `measure_selection(mask, scale)`,
  `measure_ruler(Line, scale)`, `measure_count(count_state, scale)`.
- `pictura_analysis::datapoints` — `DataPoint` enum (Angle…Width), grouping by
  tool, `DataPointPreset`.
- `pictura_analysis::log` — `MeasurementLog` (append-only rows),
  `record(measurement, enabled: &DataPointSet)`, `export_tsv`, `export_histograms`
  (CSV).
- `pictura_analysis::gray` — grayscale conversion used by Gray Value, reusing
  the color engine (`ARCH-007`).
- `pictura_core::count` — Count state (shared with `TOOL-019`).
- `pictura_io::psd::resources` — typed codec for resource 1074 (Measurement
  Scale descriptor) and 1080 (Count Information).
- `pictura_io::dicom` — measurement-scale extraction (`WF-010`).

Crossing types: `MeasurementScale`, `Measurement`, `DataPoint`, `MeasurementLog`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MeasurementScaleDialog` | `QDialog` | Pixel Length, Logical Length, Logical Units, Save/Delete Preset |
| `ScaleMarkerDialog` | `QDialog` | Length, font, font size, display text, position, color |
| `SelectDataPointsDialog` | `QDialog` | Grouped checkboxes (Common + per tool), preset load/save/delete |
| `MeasurementLogPanel` | `QDockWidget` | Table view, Record Measurements button, column reorder/sort/delete, export |
| `MeasurementLogModel` | `QAbstractTableModel` | Rows of data points; append-only |
| `MeasurementLogView` | `QTableView` | Sorting, column drag, context export |
| `DataPointSetModel` | `QAbstractItemModel` | Enabled data points; presets |
| `ScaleMarkerOverlay` | scene item | Renders/edits the on-image marker (with the layer group) |

`MeasurementLogPanel` is shared with `PAN-023` (which owns the panel contract);
this spec supplies its data points and export. Widgets over QML: dense,
table-like, consistent with `ARCH-003`.

## Data-model impact

- **Document** gains `measurement_scale: Option<MeasurementScale>` (PSD resource
  1074) and, via `TOOL-019`, `count_groups`/`active_count_group` (resource
  1080). Scale and count state are Extended-only document data.
- **Scale presets** and **data-point presets** are application/preset state
  (`10-workflow-io/presets-manager.md`), not per document.
- **Measurement Log** rows are session/document records. Like CS6, recording
  must not mutate on-image counts and clearing counts must not rewrite Log rows.
  The Log is not currently documented as saved in PSD; treat it as
  session-scoped unless a persistence decision says otherwise (open question).
- **Scale markers** are ordinary layer groups and are fully undoable/editable
  through the normal layer model.
- **Undo:** setting the scale, placing/removing a marker, and count
  add/remove/move/clear are undoable commands; **recording a Log row is
  append-only and not undone by clearing counts** (`TOOL-019`).
- **Round-trip:** resource 1074 (descriptor) and 1080 (descriptor) must be
  written/read losslessly for Adobe interop.

## Edge cases

- **Standard build** — all Analysis measurement commands and the Count tool are
  absent/disabled; `OVR-002`.
- **No scale set** — measurements use `1 px = 1 px`; Area is in square pixels.
- **Scale changed after recording** — existing Log rows keep the units in
  effect when recorded; new rows use the new scale.
- **Multiple selections** — summary row + per-Feature rows; Feature numbering
  is deterministic.
- **Tiny/1-px selections** — Circularity may be undefined/invalid; mark as
  invalid rather than emitting `∞`/`NaN`.
- **Empty selection** — record zero/undefined fields rather than failing.
- **16/32-bit** — Gray Value uses the documented ranges; Histogram down-converts
  to 8-bit; document this as behavioral parity.
- **CMYK/Lab/indexed** — Gray Value converts internally to grayscale; the
  conversion uses the default grayscale profile, which may differ from the
  document profile (CS6 behavior).
- **Huge documents/PSB** — area/perimeter/histogram must stream over tiles, not
  allocate a full-document buffer.
- **Count group deletion with marks** — semantics from `TOOL-019` (blocked vs
  clear); Log rows unaffected.
- **Export path collisions** — histogram CSV numbering restarts at 0 per
  export and lives in its own folder; avoid clobbering existing files.
- **DICOM auto-scale** — imported scale overrides the default; do not silently
  overwrite a user scale without a prompt.
- **GPU unavailable** — measurement is CPU; overlays fall back to CPU.

## Parity acceptance criteria

- Given `Analysis > Set Measurement Scale > Custom` with Pixel Length 50,
  Logical Length 1, Units microns, a 100-px Ruler measures 2 microns in the
  Log.
- Given `Set Measurement Scale > Default`, Area is reported in square pixels.
- Given a rectangular selection with Area/Perimeter/Height/Width enabled, a Log
  row is appended with correct values and `Source = Selection`.
- Given several disjoint selections recorded together, one summary row plus one
  row per Feature appears, with unique Feature numbers.
- Given a Ruler measurement with Angle enabled, values are within ±0–180.
- Given the Count tool with the Count data point enabled, `Record Measurements`
  appends a row whose Count equals the number of placed marks (selection case:
  number of discontiguous areas).
- Given `File > Save` after setting a scale, reopening the PSD restores the
  scale (resource 1074).
- Given a DICOM file with a scale, the imported scale matches the file and is
  used by the Log.
- Given `Analysis > Place Scale Marker`, a layer group (text + graphic) is
  created in the lower-left; moving and deleting the group behaves as a normal
  layer group.
- Given Log export, a tab-delimited UTF-8 text file is produced; when Histogram
  is enabled, numbered CSV files appear in a sibling folder.
- Given recording then clearing the on-image count, the Log rows are unchanged.
- Given `File > Print` (or any raster output), measurement overlays are never
  baked into the printed composite.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  CS6 Help corpus (`pdftotext -layout`). Established: `Measurement (Photoshop
  Extended)` overview (Ruler/selection tools; irregular areas via Lasso/Quick
  Select/Magic Wand; height/width/area/perimeter; one or many images;
  tab-delimited Unicode export); measurement scale (default 1 px = 1 px,
  Custom dialog with Pixel Length/Logical Length/Logical Units, presets,
  one scale per document, Save with document, Info/window display); scale
  markers (dialog fields, lower-left placement, layer group, add/replace,
  delete); performing a measurement (Select Data Points Custom/presets, Common
  grouping, Record Measurements, multiple-selection summary + per-Feature
  rows, Document column); the full **Measurement Data Points** list (Angle,
  Area, Circularity, Count, Date and Time, Document, Gray Value ranges,
  Height, Histogram + CSV export, Integrated Density, Label, Length,
  Perimeter, Scale, Scale Units, Scale Factor, Source, Width); Measurement Log
  (rows/columns, reorder/sort/delete, export); DICOM automatic scale import and
  the 1 px = 1 mm fallback; Count tool cross-reference (automatic counting via
  Magic Wand / Color Range; Count data point requiring selection).
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD image resource **1074** "Measurement Scale" and **1080** "Count
  Information" (both descriptor-based, CS3/CS4), establishing in-file
  persistence keys.
- `03-tools/note-and-count.md` (`TOOL-018`/`TOOL-019`) — in-repo:
  Count tool groups, marker/label sizes, clear/move/delete, record-to-log rule
  (clearing counts does not rewrite Log rows), persistence.
- `02-ui-ux/panels/measurement-log-panel.md` (`PAN-023`) — in-repo: panel
  contract, Record Measurements button, Count data point dependency, export.

Not fetched / not used: `helpx.adobe.com` (403).

## Open questions

- **Measurement Log persistence.** Whether CS6 saves Log rows in the PSD or
  only in the session; which resource (if any) carries them. *Resolves with:* a
  CS6-saved PSD with recorded measurements.
- **Area/perimeter algorithm.** Pixel-counting convention, boundary inclusion,
  and anti-aliasing treatment. *Resolves with:* controlled CS6 measurements.
- **Connected-component connectivity** for automatic counting (4- vs
  8-neighbour) and minimum size. Shared with `TOOL-019`. *Resolves with:*
  controlled CS6 selections.
- **Logical-units list.** The full set and any locale differences.
  *Resolves with:* a CS6 capture.
- **Scale-marker defaults** (length, font, size, position) and the exact layer
  structure/generated names. *Resolves with:* a CS6 capture.
- **Histogram CSV format.** Column headers, bin range, and per-channel
  layout. *Resolves with:* a CS6 export.
- **Record-to-log schema** for the Count data point (columns/labels) shared
  with `TOOL-019`. *Resolves with:* a CS6 export.
- **Actions/scripting.** Whether measurement and Count commands are recordable
  action steps and their parameters. *Resolves with:* CS6 action/scripting
  docs.
