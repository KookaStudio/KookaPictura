# Measurement Log Panel

- **Spec ID:** `PAN-023`
- **Status:** `Draft`
- **Parity tier:** `Extended-only` — the CS6 source feature is Photoshop **Extended** only. Kooka Pictura treats measurement/count/DICOM parity as a `Non-goal (Linux)` (`OVR-003`); a `Core` build must still **represent and preserve** the data.
- **New in CS6:** `No` — the Measurement Log and its workflow are carried from CS5; the CS6 Help documents the Measurement feature under "(Photoshop Extended)" with no functional change called out.
- **Depends on:** `03-tools/note-and-count.md` (`TOOL-019` Count tool; the Measurement Log is the shared record surface), `03-tools/eyedropper-color-sampler-ruler.md` (Ruler tool / measurement scale), `10-workflow-io/measurement-and-count.md` (record schema, export, persistent measurement data — currently *planned* in `INDEX.md`), `00-overview/cs6-editions-and-constraints.md` (`OVR-002`), `00-overview/feasibility-and-non-goals.md` (`OVR-003`), `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`).

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. Per `OVR-003`, exact measurement parity is a non-goal; this
> spec defines the UI surface and the preservation contract.

## CS6 behavior

The **Measurement Log panel** (`Window > Measurement Log`) is the record surface for the Extended-only measurement and counting workflow. Measurements are made with the **Ruler tool**, the **Count tool**, or **selection tools**, and recorded into the log by `Analysis > Record Measurements` (also a **Record Measurements** button in the panel). Each row is a **measurement set**; each column is a **data point**.

With the Photoshop Extended measurement feature, any area defined by the Ruler tool or a selection tool can be measured, including irregular areas from the Lasso, Quick Select, or Magic Wand tools. Height, width, area, and perimeter can be computed, and measurements can be tracked for one image or several.

### Measurement scale

- `Analysis > Set Measurement Scale` sets a document's scale (pixels ↔ logical units such as inches, millimetres, or microns). Multiple **presets** are allowed, but **only one scale is active per document**.
- `Analysis > Set Measurement Scale > Default` returns to **1 pixel = 1 pixel**.
- `Custom` opens the **Measurement Scale** dialog: the Ruler tool is auto-selected; drag (or type **Pixel Length**) and enter **Logical Length** and **Logical Units**.
- Scale is shown in the **Info panel** (panel menu → Panel Options → Measurement Scale) and optionally at the document-window bottom (`Show > Measurement Scale` from the document window menu), and is saved with the document via `File > Save`.
- **Create a preset:** `Set Measurement Scale > Custom` → set scale → **Save Preset** → name → OK. **Delete a preset:** `Custom` → select preset → **Delete Preset** → OK.
- **DICOM note:** measurement scale is imported automatically from DICOM files. *(DICOM is out of scope per `OVR-003`; preserve only.)*

### Scale markers

`Analysis > Place Scale Marker` places a scale marker in the lower-left corner and adds a **layer group** to the document containing a text layer (when **Display Text** is on) plus a graphic layer. The **Measurement Scale Marker** dialog exposes **Length**, **Font**, **Font Size**, **Display Text**, **Text position** (above/below), and **Color** (black/white). Adding another marker offers **Remove**/**Keep** for the existing one; markers are moved/edited with the Move/Type tools and deleted by deleting the marker's layer group.

### Data points and recording

- `Analysis > Select Data Points` chooses which data points are recorded; `Custom` opens the **Select Data Points** dialog (grouped by measuring tool; **Common** points available to all tools; all selected by default). A subset can be saved as a **data point preset**, edited, or deleted.
- If a measurement is made with a tool whose data points are not selected, the user is prompted to select data points for that tool.
- Recording is via `Analysis > Record Measurements` or the panel's **Record Measurements** button. A **Count** measurement requires the Count data point to be enabled (`03-tools/note-and-count.md`, `TOOL-019`).
- **Multiple selections:** one summary/cumulative row for all selected areas, followed by one row per selection area; each area is a **Feature** in the **Label** column with a unique number. The **Document** column identifies the source document, so a single log can span documents.
- Measurements use the scale units in effect *at the moment of recording*.

### Measurement Log operations

- **Rows:** click to select; `Shift`-click for contiguous; `Ctrl`/`Cmd`-click for non-contiguous; **Select All** / **Select None**. Rows cannot be manually reordered.
- **Columns:** click a header to select; `Shift`/`Ctrl`-click for contiguous/non-contiguous; drag to reorder; drag the separator to resize; click a header to sort, or right-click → **Sort Ascending**/**Sort Descending**.
- **Delete:** select rows/columns → `Delete` from the options menu, the panel's Delete icon, or right-click → Delete.
- **Export:** select rows → `Export` from the options menu, the Export icon, or right-click → Export → filename/location → Save. Output is a **comma-delimited UTF-8 text file**. *(The CS6 text says both "tab-delimited, Unicode text file" in the About section and "comma-delimited text file" for export; see Open questions.)*
- **Histogram** data is exported to a separate **CSV** file in its own folder beside the log export, numbered from 0; one file for the total selected area plus one per selection.

### Measurement data points (CS6 list)

**Angle** (Ruler orientation ±0–180), **Area** (square pixels or calibrated units), **Circularity** (`4π·area/perimeter²`; 1.0 = circle), **Count** (discontiguous selection areas; counted items; or Ruler lines 1/2), **Date and Time**, **Document**, **Gray Value** (0–255/8-bit, 0–32,768/16-bit, 0.0–10/32-bit; image internally converted to grayscale via the default grayscale profile, then mean/median/min/max per feature and summary), **Height** (`max y − min y`), **Histogram** (per-channel counts, 8-bit), **Integrated Density** (`Area × Mean Gray Value`), **Label** (automatic Measurement N + Feature labels), **Length** (Ruler distance), **Perimeter**, **Scale** (source scale), **Scale Units**, **Scale Factor** (pixels per scale unit), **Source** (Ruler tool / Count tool / Selection), **Width** (`max x − min x`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Measurement Log` | Menu → dock panel | — | The log |
| `Analysis > Record Measurements` | Menu | — | Also the panel's Record Measurements button |
| Panel top | Record Measurements button | — | Records current measurement |
| Panel top | Delete icon | — | Delete selected rows/columns |
| Panel top | Export icon | — | Export to comma-delimited UTF-8 |
| Panel options menu (fly-out) | Menu | — | Delete / Export |
| Column header | Interaction | — | Select, drag-reorder, resize, click-sort, right-click sort |
| Row | Interaction | `Shift`/`Ctrl`-click | Multi-select; no manual row reorder |
| `Analysis > Set Measurement Scale` | Menu | — | Default / Custom / presets |
| `Analysis > Place Scale Marker` | Menu | — | Scale Marker dialog; adds layer group |
| `Analysis > Select Data Points` | Menu | — | Common/tool data points; presets |
| `Analysis > Ruler Tool` / `Analysis > Count Tool` | Menu | — | Select measuring tool |
| Ruler-tool options bar | Bar | — | Set/clear measurement scale entry points |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Measurement scale | ratio | 1 px = 1 px | Pixel Length + Logical Length + Logical Units | One active per document |
| Scale preset | named entry | Default | multiple | Added to `Set Measurement Scale` submenu |
| Scale marker length | number (logical units) | — | > 0 | Pixel length depends on scale |
| Scale marker font / size | font / pt | — | any installed font | Display text caption |
| Scale marker display text | bool | — | on/off | Logical length + units |
| Scale marker text position | enum | below *(not stated)* | above / below | — |
| Scale marker color | enum | black *(not stated)* | black / white | Marker + caption |
| Data point set | bool per point | all on | 17 listed points | Common vs. per-tool grouping |
| Data point preset | named set | none | Save/Edit/Delete | — |
| Log columns | ordered set | selected data points | reorderable/resizable | One column per selected data point |
| Sort | enum per column | none | Ascending / Descending | Click header or right-click |
| Export format | enum | comma-delimited UTF-8 | (tab-delimited in About text) | See Open questions |
| Histogram export | separate CSV | — | per channel, indexed from 0 | One extra file per selection/area |

## Algorithms & pipeline

The measurement engine is **largely documented at the semantics level only**; the exact numeric implementation is Adobe-closed in places, and `OVR-003` classifies exact parity as a non-goal.

1. **Scale resolution.** `logical_per_pixel = logical_length / pixel_length`. All calibrated data points (Area/Height/Width/Length/Perimeter/Scale Factor) are computed with the document's active scale; uncalibrated (pixel) values remain available.
2. **Selection measurement.** Compute per-selection area, perimeter, bounding box (height/width), mean/median/min/max gray, integrated density, and circularity from the selection mask and the (optionally grayscale-converted) pixels. For multiple selections, emit one cumulative row then one row per connected selection, labelled as Features.
3. **Ruler measurement.** Length = Euclidean distance; angle = orientation ±0–180; Count of Ruler lines = 1 or 2.
4. **Count measurement.** A selection counts **discontiguous selected areas**; the Count tool counts placed count marks (including, in the automatic workflow, connected components of a Magic Wand / Color Range selection — see `TOOL-019`).
5. **Gray Value conversion.** The image is *internally converted to grayscale using the default grayscale profile* before brightness statistics; 16/32-bit values are scaled to the documented ranges. Histogram is per channel, reduced to 8-bit.
6. **Recording.** Appending a record copies the selected data points for the current measurement into a new row; existing rows are immutable to later on-image edits (clearing counts does not rewrite the log; `TOOL-019`).
7. **Export.** Serialize rows/columns to a delimited UTF-8 text file; each Histogram point writes its own CSV in a sibling folder with sequential names from 0.

**Non-goal note.** The precise selection/edge/AA treatment and the exact data-point formulas beyond those quoted are not fully documented; this spec claims **behavioral parity only, algorithm TBD** and, per `OVR-003`, a preserve-and-report contract.

## Rust module mapping

- `pictura_measure::MeasurementScale` — `{ pixel_length, logical_length, logical_units }`; presets.
- `pictura_measure::DataPoint` — enum of the 17 CS6 data points; `DataPointSet` (Common + per-tool), presets.
- `pictura_measure::MeasurementRecord` — `{ label, feature, document_id, source: Ruler|Count|Selection, values: Map<DataPoint, Value>, scale }`.
- `pictura_measure::MeasurementLog` — `Vec<MeasurementRecord>`; append-only; sort/select/delete/export. **Shared with `03-tools/note-and-count.md`** (`pictura_core::measure::MeasurementLog`).
- `pictura_measure::scale_marker` — marker parameters plus the layer group it creates (text + graphic).
- `pictura_measure::export` — delimited text + histogram CSV writer.
- `pictura_core::document` gains an opaque `measurement` field to preserve image resources **1074 (Measurement Scale)** and **1080 (Count Information)** on round-trip (`OVR-002`).

Crossing types: `MeasurementRecord`, `DataPoint`, `Value`, `MeasurementScale`, `DocumentId`. No Qt types cross into `pictura_measure`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MeasurementLogPanel` | `QDockWidget` | Host; Record/Delete/Export buttons; options menu |
| `MeasurementLogModel` | `QAbstractTableModel` | Rows = records, columns = data points; sort/selection |
| `MeasurementLogView` | `QTableView` | Column select/reorder/resize/sort; row multi-select |
| `DataPointsDialog` | `QDialog` | Common/tool data points; preset save/edit/delete |
| `MeasurementScaleDialog` | `QDialog` | Pixel/Logical Length/Units; Save/Delete Preset |
| `ScaleMarkerDialog` | `QDialog` | Length/font/size/display/position/color; add or replace |
| `RecordMeasurementsAction` | `QAction` | Enabled with a supported measuring tool / valid selection |

Widgets over QML (dense table, model/view, keyboard-centric; `ARCH-003`). Sort/filter live in a proxy model; export runs off the GUI thread. The panel is **absent/disabled** in a Standard-parity build (`OVR-002`), while preserved measurement image resources remain readable.

## Data-model impact

- **Document data.** Measurement scale is a document setting saved with the file; the panel edits it. **Count Information** and **Measurement Scale** image resources (`1080`, `1074`) are Extended-relevant and round-trip even in a `Core` build (`OVR-002`, `OVR-003`).
- **Scale markers** materialize as real layers (a layer group with text + graphic), so they are ordinary document nodes and participate in undo and PSD serialization.
- **Measurement Log rows** are **append-only session/document records**; recording does not mutate on-image counts, and clearing on-image counts leaves existing rows (`TOOL-019`).
- **Undo.** Measurement scale changes, scale-marker placement, and data-point preset edits are commands (scale and markers document-level; presets app-state). Appending a log row is a non-undoable record by design (CS6 does not undo log rows) *(inferred)*.
- **Preservation contract.** A `Core` build must open a CS6 Extended PSD, keep the measurement resources and scale-marker layer group intact, and report the Measurement Log surface as unavailable.
- **No XMP impact** beyond whatever DICOM/file metadata a source file carried.

## Edge cases

- **Standard-parity build** — panel absent/disabled; preserved resources still round-trip (`OVR-002`).
- **No scale set** — default 1 px = 1 px is used and the log records Scale/Scale Units/Scale Factor accordingly.
- **Mixed scales across documents** — each record carries the scale in effect at record time; the `Document` column disambiguates.
- **Multiple selections** — cumulative row plus per-selection rows and Features; must not double-count the summary.
- **Empty selection** — Record Measurements is disabled or no-ops; never writes a garbage row.
- **Non-grayscale/CMYK/Lab/32-bit** — Gray Value uses the internal grayscale conversion; the panel must not silently read raw channels.
- **Histogram export** — many selections produce many CSVs; enforce a file-count/name scheme and report the folder.
- **Delimiter mismatch** — the About text says tab-delimited while the export procedure says comma-delimited; pick and document one (Open questions).
- **DICOM** — out of scope (`OVR-003`); open/import must preserve metadata without claiming analysis parity.
- **Huge PSB / many selections** — perimeter/area/distribution computation must tile and stream; do not materialize whole-canvas masks per feature.
- **Log size** — a long session can accumulate thousands of rows; virtualize and keep export streaming.
- **Undo** — deleting a row/column is a panel operation (not document undo); confirm before export-affecting deletion if CS6 does *(inferred)*.
- **GPU unavailable** — measurement is CPU-side; the log still works.

## Parity acceptance criteria

Tier is `Non-goal` for exact parity (`OVR-003`); criteria assert structure and preservation:

1. Given a scale of 50 px = 1 micron, a 50-px Ruler line records Length 1 micron and a matching Scale/Scale Units/Scale Factor.
2. Given multiple selections, one cumulative row plus one row per selection appear, each selection labelled as a distinct Feature with a unique number.
3. Given a Ruler, Count, and selection measurement, only the data points associated with each tool (plus selected Common points) appear for that measurement.
4. Given a Count measurement with the Count data point disabled, the app prompts to select data points; with it enabled, the recorded count matches the on-image total.
5. Given clearing on-image counts after recording, existing Measurement Log rows are unchanged.
6. Given selected rows and Export, a delimited UTF-8 file is written with one row per record and one column per data point; Histogram points additionally produce per-selection CSV files in a sibling folder, numbered from 0.
7. Given a column header click, the log sorts Ascending; a second click sorts Descending; rows cannot be manually reordered.
8. Given `Analysis > Place Scale Marker` with Display Text, a layer group containing a text layer and a graphic layer appears at the lower-left; deleting the group removes the marker.
9. Given a CS6 Extended PSD with measurement scale (1074) and Count Information (1080), a `Core` build opens and re-saves it with both resources preserved and reports the Measurement Log as unavailable.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: the Extended-only Measurement feature and its scope (Ruler/selection tools; height/width/area/perimeter; one or many images); the Measurement Log as the record surface; measurement scale (presets, one per document, Default 1 px = 1 px, Custom with Pixel Length/Logical Length/Logical Units, Info-panel and document-window display, save-with-document, create/delete preset, DICOM auto-import); scale markers (`Place Scale Marker`, Length/Font/Font Size/Display Text/Text position/Color, lower-left placement, layer-group construction, Remove/Keep, delete via Layers panel); performing a measurement (tool/data-point matching, Common points, all-selected default, prompt on mismatch, `Record Measurements`, multiple-selection summary + per-selection rows and Features, Document column, scale-at-record-time); the full Measurement Data Points list with definitions and ranges (Angle, Area, Circularity, Count, Date and Time, Document, Gray Value, Height, Histogram, Integrated Density, Label, Length, Perimeter, Scale, Scale Units, Scale Factor, Source, Width); data-point preset create/edit/delete; Measurement Log operations (row/column selection, reorder/resize/sort, delete, export to a comma-delimited UTF-8 text file, per-selection Histogram CSVs numbered from 0).
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (same corpus) — also established the Count-tool cross-reference ("record the results in the Measurement Log panel") and `Analysis > Select Data Points > Custom` gating, mirrored in `03-tools/note-and-count.md`.
- `https://searxng` query "Photoshop CS6 Measurement Log panel measurement scale data points export" — discovery only.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- Adobe's current-version "Measure" help pages surfaced by the query; later-version UI, not asserted as CS6.

Not used in this pass:

- `helpx.adobe.com` measurement pages (HTTP 403 / current-version only).

## Open questions

- **Export delimiter.** The fetched CS6 text says "tab-delimited, Unicode text file" (About) and "comma-delimited text file" (export procedure). Which CS6 actually writes is unresolved. *Resolves with:* a CS6 export sample; coordinate with `10-workflow-io/measurement-and-count.md`.
- **Record schema and column set.** The exact default columns/labels per data point and whether defaults changed in CS6 are not itemized. *Resolves with:* a CS6 log screenshot and `10-workflow-io/measurement-and-count.md`.
- **Scale-marker defaults** (font, size, text position, color, opacity) are not stated. *Resolves with:* a CS6 dialog capture.
- **Rounding/precision and grayscale-profile behavior** for Gray Value/Histogram are only qualitatively described. *Resolves with:* controlled CS6 measurements.
- **Are log rows persisted in the PSD or session-only?** The fetched text implies the log is a session panel; whether rows survive save/reopen is unverified. *Resolves with:* a CS6 test.
- **Automatic-count connected-component rules** (8- vs 4-connectivity, min size) are unverified — shared with `TOOL-019`. *Resolves with:* controlled selections on CS6.
- **Does the project implement the panel at all** given the `Non-goal` tier, or only preserve resources 1074/1080 without a measurement engine? *Resolves with:* an ADR following `OVR-003`.
