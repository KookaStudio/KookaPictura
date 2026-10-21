# Printing

- **Spec ID:** `WF-013`
- **Status:** `Draft`
- **Parity tier:** `Core` for the **Print dialog** (position/scale/preview, color management and proofing, output marks) and for `Print One Copy`. `Non-goal (Linux)` for OS-specific print-driver integration (Windows DEVMODE / macOS print system) and for the CS6 `Send 16-bit Data` macOS path.
- **New in CS6:** `Changed` — CS6 reworked the Print dialog: it adds an **Edit** button for the **Description** field, lets the **Print dialog and preview window be resized**, allows a **customizable preview background color**, and **reintroduces Print Selected Area** (adjustable in the dialog). The Properties-panel-driven CS5 print flow is replaced by the single CS6 dialog (CS5 behavior is still documented in the same chapter).
- **Depends on:** `ARCH-007` color-management (proofing/transforms), `ARCH-008` document-model, `ARCH-009` undo-history, `WF-011` color-settings, `05-layers/vector-masks-and-clipping-masks.md` (vector data), `04-image-ops/bit-depth-and-conversion.md`, `OVR-003` feasibility.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Printing is where Linux parity is hardest; this spec draws
> the line explicitly rather than promising driver-level fidelity.

## CS6 behavior

### Commands

- **File > Print** (`Ctrl+P` / `Cmd+P`) — opens the Print dialog with live
  preview; customized settings are saved as new defaults on **Done**/**Print**.
  All print settings live in one dialog (so `Print` can be recorded in actions).
- **File > Print One Copy** — prints one copy without a dialog.
- **Print Selected Area** — with a rectangular marquee selection, `File >
  Print` exposes Print Selected Area; handles in the preview let you adjust the
  region.

### Print dialog sections (CS6)

1. **Printer / job** — printer, copies, orientation; **Print Settings** opens
   the driver's page setup (paper size, source, orientation).
2. **Preview** — the shaded border is the paper margin; the white area is
   printable. Position via `Center Image` / `Top`+`Left` / drag; scale via
   `Scale To Fit Media` / `Scale`+`Height`+`Width` / drag handles. A **Print
   Resolution** readout shows ppi at the current scaling. The preview
   background color is customizable (CS6).
3. **Position and Size** — the controls above, plus **Print Selected Area**.
4. **Color Management** — see below.
5. **Output** — page marks and prepress output options (see below).
6. **PostScript Options** — `Include Vector Data` (sends each type/vector layer
   as a separate, vector-clipped image at printer resolution).

**Scaling caution (PDF):** the Print command's scaling affects only the image,
whereas a driver scaling option affects everything on the page including marks.
Set scaling in **one** place only.

### Color management

- **Color Handling: Photoshop Manages Colors** — pick a **Printer Profile**;
  set **Rendering Intent** and **Black Point Compensation**. Recommended when
  you have a custom printer/ink/paper profile.
- **Color Handling: Printer Manages Colors** — the driver converts; the
  **Document Profile** is shown. Many non-PostScript drivers ignore the
  rendering intent and use Perceptual.
- Preview aids: **Match Print Colors**, **Gamut Warning** (enabled with Match
  Print Colors), **Show Paper White**.
- The user must turn off color management in the **printer driver** when
  Photoshop manages, or the two override each other.

### Hard proofing

- `View > Proof Setup` selects the press condition to simulate (preset or
  custom, saved so it appears in the Print dialog).
- In the Print dialog: `Photoshop Manages Colors`, choose the output-device
  **Printer Profile**, then select **Hard Proofing** from the menu above the
  Proof Setup/Rendering Intent menus. **Proof Setup** picks a local custom
  proof; **Simulate Paper Color** and **Simulate Black Ink** increase accuracy
  (not available for all profiles).

### Output marks and prepress options

From the PDF's `Page marks` list:

| Option | Behavior |
|---|---|
| Calibration Bars | 11-step grayscale (0–100%) / CMYK gradient + progressive bars |
| Registration Marks | Bull's-eyes and star targets for aligning separations |
| Corner Crop Marks | Trim marks at corners (also star targets on PostScript) |
| Center Crop Marks | Trim marks at the center of each edge |
| Description | File Info Description text, ~300 chars, 9-pt Helvetica plain |
| Labels | File name (and separation name) above the image |
| Emulsion Down | Makes type readable with emulsion down (film) |
| Negative | Inverts the printed output (not the on-screen image) |
| Background | Page background color outside the image |
| Border | Black border of specified width |
| Bleed | Prints crop marks inside the image; width settable |
| Interpolation | Resamples low-res images up at print (PostScript) |
| Include Vector Data | Vector layers printed as vector-clipped separations |

Marks print only if the paper is larger than the image. **Print Separations**
prints each color channel as a separate page (CMYK/spot).

### 16-bit / 32-bit printing

- In **Mac OS**, the CS6 Help instructs expanding Color Management and
  selecting **Send 16-bit Data** for subtle gradients; the CS5 text says the
  same. This is a macOS-only path in CS6.
- 32-bit float documents must be converted before ordinary printing; the CS6
  Help does not document a general 32-bit print pipeline. Treat high-bit-depth
  printing as format/driver dependent (open question).

### Linux reality (CUPS + Qt6)

CS6's print stack is OS-driver-centric (Windows DEVMODE, macOS print system).
On Linux the natural backend is **CUPS** with **PDF** as the job format:

- **Qt 6 `QPrinter`** — on X11 it uses **CUPS** and sends **PDF** output; with
  no valid printer it can emit a **searchable PDF** (`PdfFormat`). `QPrinter`
  exposes page size/orientation (`QPageLayout`), resolution, copies, duplex,
  color mode, and page ranges.
- **`QPrintDialog` / `QPageSetupDialog`** — printer selection, page size and
  orientation, color/grayscale, ranges, copies, and (on CUPS) a **Properties**
  button for driver options.
- **`QPrinterInfo`** — enumerate available CUPS queues.

**Realistic parity:**

- *Achievable:* printer/queue selection, page setup, copies, orientation,
  duplex, print ranges, position/scale, page marks, `Print One Copy`,
  Photoshop-manages color by rendering an already-color-converted raster into
  the print device, soft/hard proof preview, and `Include Vector Data` where a
  PostScript/CUPS path exists.
- *Partial:* device profiles come from CUPS/colord, not Adobe; "printer manages
  colors" is delegated to the CUPS filter chain with no guarantee of intent
  honoring; 16-bit print is driver-dependent (CUPS raster/PDF pipelines
  typically 8-bit).
- *Non-goal:* bit-exact CS6 PostScript output, Windows DEVMODE / macOS
  `NSPrintInfo` round-tripping, Adobe-specific separation/PDF/X print features,
  and the macOS-only `Send 16-bit Data` switch.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| File > Print | Dialog | `Ctrl+P` / `Cmd+P` | CS6 single-dialog workflow |
| File > Print One Copy | Menu | — | No dialog |
| Print — printer/copies/orientation | Controls | — | Top of dialog |
| Print — Print Settings | Button | — | Opens driver/CUPS page setup |
| Print — preview | Canvas | — | Position/scale, adjustable background (CS6) |
| Print — Position And Size | Section | — | Center Image, Top/Left, Scale To Fit Media, Scale/Height/Width, Print Resolution |
| Print — Print Selected Area | Checkbox | — | Reintroduced in CS6 |
| Print — Color Management | Section | — | Color Handling, Printer Profile, Intent, BPC |
| Print — Match Print Colors / Gamut Warning / Show Paper White | Checkboxes | — | Preview aids |
| Print — Hard Proofing | Menu | — | Uses View > Proof Setup |
| Print — Proof Setup | Menu | — | Local custom proofs |
| Print — Simulate Paper Color / Simulate Black Ink | Checkboxes | — | Profile dependent |
| Print — Output | Section | — | Calibration bars, registration, crop marks, description, labels, emulsion, negative, background, border, bleed, interpolation |
| Print — PostScript Options > Include Vector Data | Checkbox | — | Vector layers at printer resolution |
| View > Proof Setup / Proof Colors | Menu/toggle | `Ctrl+Y` | Shared with `ARCH-007` |
| File Info — Description | Dialog | — | Prints in Output > Description (`WF-010`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Printer | Enum | system default | CUPS queues | |
| Copies | Integer | 1 | ≥1 | |
| Orientation | Enum | from driver | portrait / landscape | |
| Center Image | Bool | On | on/off | |
| Top / Left | Number | 0 | document units | when Center off |
| Scale To Fit Media | Bool | On *(unverified)* | on/off | |
| Scale | Percent | 100 | e.g. 1–1000% *(unverified)* | image only |
| Height / Width | Number | document size | units | |
| Print Resolution | Read-out | computed | ppi | not editable |
| Print Selected Area | Bool | Off | on/off | requires marquee |
| Color Handling | Enum | Printer Manages Colors *(unverified)* | Photoshop Manages Colors / Printer Manages Colors | |
| Printer Profile | Profile | current printer default | installed output profiles | Photoshop-manages only |
| Rendering Intent | Enum | Relative Colorimetric *(unverified)* | Perceptual / Relative / Saturation / Absolute | |
| Black Point Compensation | Bool | On *(secondary)* | on/off | |
| Match Print Colors | Bool | Off | on/off | Photoshop-manages only |
| Gamut Warning | Bool | Off | on/off | needs Match Print Colors |
| Show Paper White | Bool | Off | on/off | |
| Hard Proofing | Enum selection | off | Hard Proofing / (driver) | |
| Proof Setup | Enum | current View proof | local custom proofs | |
| Simulate Paper Color / Black Ink | Bool pair | Off | on/off | profile dependent |
| Marks (calibration/registration/crop/labels/description) | Bool group | Off | on/off | paper must be larger |
| Emulsion Down / Negative | Bool pair | Off | on/off | film |
| Background / Border / Bleed | Color/number | none/0 | width + units | |
| Interpolation | Bool | Off | on/off | PostScript |
| Include Vector Data | Bool | Off | on/off | PostScript Options |
| Send 16-bit Data (macOS) | Bool | Off | on/off | **Not parity on Linux** |

## Algorithms & pipeline

1. **Compose a print raster.** Rasterize the visible composite (or the selected
   area) at the printer resolution and the requested scale. Vector layers may
   be emitted separately when `Include Vector Data` is on.
2. **Color-convert.** If Photoshop manages, build a document→printer-profile
   transform with the chosen intent/BPC/dither (`ARCH-007`) and apply it to the
   print raster. If the printer manages, pass the document profile through and
   rely on the driver/CUPS filters.
3. **Hard proof.** When hard proofing, first transform through the **proof
   profile** (with paper/black-ink simulation as configured), then through the
   **printer profile**, so the print simulates the press.
4. **Add marks.** Draw calibration bars, registration marks, crop marks,
   labels, description text, background, border, and bleed in device space
   according to the paper/image rectangles (`QPrinter::paperRect` /
   `pageRect`).
5. **Emit.** On Linux, hand a PDF (or raster) to CUPS; CUPS applies the queue's
   filters and driver. Page ranges/copies/duplex are set on `QPrinter`.
6. **Record.** On `Done`/`Print`, persist the dialog's settings as the new
   defaults. `Print` has no document side effects; the printed pixels are not
   written back to the document.

Marks sizing rules (paper larger than image; crop/registration geometry) are
standard prepress behavior; exact Adobe geometry is not documented in the CS6
Help PDF and is behavioral-parity only.

## Rust module mapping

Proposals:

- `pictura_print::job` — `PrintJob { source: PrintSource, page: PageSetup,
  position: Position, scale: Scale, marks: Marks, color: ColorHandling }`.
- `pictura_print::layout` — `PaperSpec`, `ImagePlacement`, `compute_placement`
  (paper vs printable rect), `ScaleToFit`, `print_ppa() -> f32`.
- `pictura_print::marks` — `render_marks(&PageSetup, &Marks)` (calibration,
  registration, crop, labels, description, background, border, bleed).
- `pictura_print::proof` — `HardProof { setup, simulate_paper, simulate_black }`
  composed with the color transforms from `pictura_color` (`ARCH-007`).
- `pictura_print::backend` — trait `PrintBackend { enumerate, submit(PdfJob),
  page_setup() }`; `CupsBackend` implementation; `PdfBackend` for output-to-file.
- `pictura_print::vector` — vector-layer extraction and clipping for `Include
  Vector Data` (shares path machinery with `08-selection/paths…`/layer specs).
- `pictura_print::settings` — persisted dialog defaults.

Crossing types: `PrintJob`, `PaperSpec`, `Marks`, `ColorHandling`,
`PdfJob`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PrintDialog` | `QDialog` | CS6-style single dialog: preview, sections, Print/Done/Cancel |
| `PrintPreviewWidget` | custom `QWidget` | Paper/image rendering, drag position/scale, adjustable background |
| `PageSetupSection` | `QGroupBox` | Printer, copies, orientation, Print Settings button |
| `PositionSizeSection` | `QGroupBox` | Center/Scale To Fit/Top/Left/Scale/Height/Width, Print Resolution |
| `ColorManagementSection` | `QGroupBox` | Color Handling, Printer Profile, intent, BPC |
| `ProofingSection` | `QGroupBox` | Match Print Colors, Gamut Warning, Show Paper White, Hard Proofing, Proof Setup, simulations |
| `OutputMarksSection` | `QGroupBox` | Marks, emulsion/negative/background/border/bleed/interpolation |
| `PostScriptSection` | `QGroupBox` | Include Vector Data |
| `PrinterCombo` / `ProfileCombo` | `QComboBox` | CUPS queues / output profiles (shared `ColorProfileModel`) |

`QPrintDialog`/`QPageSetupDialog` are used only for the native page-setup step;
the CS6-dialog parity lives in the custom `PrintDialog`. Widgets over QML: the
dialog is dense, keyboard-navigable, and consistent with `ARCH-003`.
`PrintPreviewWidget` may use `QPainter` on a `QPrinter`/`QPdfWriter` or an
offscreen render target.

## Data-model impact

- **No new document nodes.** Print settings are **application/document print
  state**, persisted as new dialog defaults (per CS6) rather than baked into the
  raster.
- PSD/PSB already carry print-related resources that a lossless implementation
  must round-trip: **1011** print flags, **1062** print scale, **1071** print
  info, **1082** print information (color management options), **1083** print
  style (marks), plus OS-specific **1084 NSPrintInfo** / **1085 Windows
  DEVMODE** (which Kooka Pictura may preserve but not interpret). On Linux these
  OS blobs have no native equivalent; preserve them opaquely.
- **Undo:** the Print command does not modify the document, so it is not a
  history state. Persisting changed dialog defaults updates preferences only.
- Selected-area printing uses the existing selection (`ARCH-008`) without
  altering it.

## Edge cases

- **No printer / no CUPS queue** — offer **Print to PDF** via `QPrinter`
  `PdfFormat` and `QPdfWriter`, and say so rather than failing.
- **Image larger than printable area** — warn and offer Scale To Fit Media, as
  CS6 does.
- **Scaling in both dialog and driver** — detect/avoid double scaling; the CS6
  Help warns explicitly.
- **Marks with paper ≤ image** — marks are suppressed (CS6 rule).
- **Overprint/knockout, spot colors, separations** — spot/CMYK separations are
  PostScript-specific and likely a **non-goal** on CUPS; document the gap.
- **PANTONE/spot plates** — cannot be meaningfully proofed on an average CUPS
  printer; hard proof only as a visual approximation.
- **16-bit/32-bit** — CUPS/PDF is generally 8-bit; down-convert with dither and
  warn; the macOS `Send 16-bit Data` switch has no Linux equivalent.
- **Vector data** — only meaningful with a PostScript/PDF backend; otherwise
  rasterize and warn.
- **Proof profile missing** — fall back to unproofed preview with a message.
- **Color appearance on screen ≠ paper** — Show Paper White / Match Print
  Colors are approximations; state them as such.
- **CMYK documents** — print via the printer profile; do not double-convert
  when the printer manages.
- **Undo/redo / memory** — printing is read-only; stream tiles rather than
  allocating a full-page float buffer for PSB.
- **CUPS driver options** — vary per queue; expose what CUPS reports, hide the
  rest.

## Parity acceptance criteria

- Given `File > Print`, the CS6-style dialog opens with a live preview and the
  Position And Size, Color Management, Output, and PostScript Options sections.
- Given a marquee selection and Print Selected Area on, only the selected
  region prints, and the preview handles allow adjustment.
- Given Scale To Fit Media, the image fits the printable area; given a manual
  Scale, the Print Resolution readout equals the scaled ppi.
- Given Photoshop Manages Colors with a printer profile, the output matches a
  reference conversion within the `ARCH-007` tolerance; given Printer Manages
  Colors, no Photoshop conversion is applied.
- Given `View > Proof Setup` + Hard Proofing, the preview and print simulate
  the proof profile, and Simulate Paper Color/Black Ink change the preview when
  supported.
- Given Output marks enabled with paper larger than the image, the marks render
  in the expected positions; with paper ≤ image they are suppressed.
- Given no CUPS printer, `File > Print` still offers Print to PDF and produces a
  valid PDF.
- Given `Print One Copy`, one copy is sent with no dialog.
- Given a 32-bit document, printing down-converts with dither and warns rather
  than producing wrong output.
- Given the CS6 scope, Windows/macOS-only controls (`Send 16-bit Data`, DEVMODE/
  NSPrintInfo round-trip) are either hidden on Linux or documented as non-goal.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  CS6 Help corpus (`pdftotext -layout`). Established: CS6 print changes (Edit
  button for Description, resizable dialog/preview, customizable preview
  background, reintroduced Print Selected Area); `Print`/`Print One Copy`;
  Print dialog sections and the Position/Scale behavior and Print Resolution
  readout; scaling-vs-driver caution; **Printing with color management in
  Photoshop CS6** (Photoshop Manages / Printer Manages, Printer Profile,
  Rendering Intent, Black Point Compensation, Match Print Colors, Gamut
  Warning, Show Paper White, driver-management turn-off); **Print a hard
  proof** (View > Proof Setup, Hard Proofing, Proof Setup, Simulate Paper
  Color, Simulate Black Ink); `Page marks` (calibration bars, registration
  marks, corner/center crop marks, description ~300 chars, labels, emulsion
  down, negative, background, border, bleed, interpolation); `Include Vector
  Data`; print separations; Mac `Send 16-bit Data`; `Description` drawing from
  File Info.
- `https://doc.qt.io/qt-6/qprinter.html` — `QPrinter` uses CUPS and sends PDF
  on X11; `PdfFormat` for PDF output; `QPageLayout`/`QPageSize`, resolution,
  copies, duplex, color mode, page ranges; `paperRect`/`pageRect`;
  `supportsMultipleCopies` false on non-CUPS X11.
- `https://doc.qt.io/qt-6/qprintdialog.html` — `QPrintDialog`: paper
  size/orientation, color/grayscale, ranges, copies, printer list; CUPS
  **Properties** button; `QPageSetupDialog` for page setup.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD print-related image resources 1011/1062/1071/1082/1083 and OS-specific
  1084/1085, establishing what a lossless save must preserve.
- `ARCH-007` (`01-architecture/color-management.md`) — rendering intents, BPC,
  proofing transform composition reused here.

Not fetched / not used: `helpx.adobe.com` (403). CUPS/OpenPrinting
documentation was not fetched; Qt's own docs are the Linux-printing source of
record here.

## Open questions

- **Exact CS6 Print dialog section layout and defaults.** The PDF describes the
  controls but not every default or the precise grouping; the CS5 subsection
  in the same chapter documents the older Properties-panel flow. *Resolves
  with:* a CS6 dialog capture.
- **High-bit-depth printing.** What CS6 actually does for 16-bit on Windows and
  for 32-bit everywhere; whether "Send 16-bit Data" is macOS-only in CS6 or was
  later extended. *Resolves with:* CS6 observation and current Adobe docs.
- **Page-mark geometry.** Exact sizes/offsets of crop marks, star targets,
  calibration bars, and bleed. *Resolves with:* measurement of CS6 output.
- **Separation/spot-color printing on Linux.** Whether any CUPS path can
  reproduce separations, or whether this is a documented non-goal. *Resolves
  with:* a CUPS/PDF separation spike and an `OVR-003` decision.
- **Printer profile source on Linux.** `colord` vs CUPS PPDs vs a bundled set,
  coordinated with `ARCH-007`. *Resolves with:* a platform spike.
- **Preservation of resources 1084/1085** on save without understanding them;
  whether keeping them causes cross-platform confusion. *Resolves with:* a
  file-format decision.
- **Print-in-action recording.** Which dialog options a recorded action stores
  in CS6. *Resolves with:* an action/scripting source.
