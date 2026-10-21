# Camera Raw Workflow (ACR 7)

- **Spec ID:** `WF-012`
- **Status:** `Draft`
- **Parity tier:** `Core` for the raw-file workflow (open raw/JPEG/TIFF, edit in ACR, store settings, open as Smart Object, DNG conversion, batch). Raw **demosaic/color/tone internals** are behavioral parity only; see `FILT-100`.
- **New in CS6:** `Changed` — CS6 ships **Camera Raw 7.0** with **Process Version 2012 (PV2012)**: rewritten Basic tone controls (`Highlights`, `Shadows`, `Whites`, `Blacks`), expanded local corrections, camera profiles, and `Snapshots`. The CS6 Help PDF documents the ACR dialog under **Camera Raw** (pages 303–335).
- **Depends on:** `FILT-100` camera-raw-filter (dialog tabs, parameters, pipeline), `LAY-020` smart-objects, `LAY-021` smart-filters, `ARCH-007` color-management, `ARCH-008` document-model, `ARCH-009` undo-history, `01-architecture/file-formats.md`, `10-workflow-io/file-info-and-metadata.md` (sidecars), `09-automation/batch-processing.md`, `09-automation/actions.md`, `04-image-ops/32-bit-hdr.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. This document owns the **workflow** (where ACR is reached,
> how settings travel, how files are batched/converted). The ACR **workspace
> tabs, controls, and pipeline** are owned by `FILT-100` and are referenced,
> not duplicated.

## CS6 behavior

### What Camera Raw is

Adobe Camera Raw (ACR) is a **separate plug-in** bundled with Photoshop,
Bridge, and After Effects. It interprets a camera raw file (unprocessed sensor
data plus capture metadata) using camera-specific knowledge to construct a
color image. In CS6 the shipped version is **ACR 7.0**, and CS6 accepts later
ACR updates (community reports up to 9.1.1) for new-camera support only.

Capabilities from the CS6 Help PDF:

- ACR opens camera raw files **and** JPEG/TIFF files (already-processed
  pixels). Raw files keep their original unprocessed data; edits are metadata.
- ACR supports images up to **65,000 px** on a side and up to **512 MP**;
  **CMYK is converted to RGB on open**.
- ACR **cannot save a camera raw file**; it saves copies as **DNG, JPEG, TIFF,
  or PSD**, or opens them in Photoshop.
- You can modify **camera-model, camera-serial, or ISO-specific defaults** and
  save settings as presets.

> The project's `FILT-100` records an authoritative correction: **`Filter >
> Camera Raw Filter` is not in CS6** (it arrived in Photoshop CC). CS6 exposes
> ACR only as the raw-decode engine and the open-as-Smart-Object path.

### Reaching ACR in CS6

- **From Bridge** — select raw/JPEG/TIFF files and `File > Open In Camera Raw`
  (`Ctrl+R` / `Cmd+R`). Bridge can apply/copy/clear settings and preview raw
  files without opening the ACR dialog.
- **From Photoshop** — `File > Open` a raw file; ACR opens before the document.
  A JPEG/TIFF opens in ACR only if the Camera Raw preference routes it there.
- **As a Smart Object** — `Shift`+click **Open Image** (or the Workflow Option
  `Open In Photoshop As Smart Objects`) opens the result as a Smart Object
  layer. Double-clicking the layer reopens ACR with the saved settings, so raw
  settings remain editable after "opening" (see `LAY-020`).
- **From After Effects** — ACR can open raw files; `Save Image`/`Done` are
  unavailable there and Workflow Options are not shown.

### The ACR workspace (summary; full detail in `FILT-100`)

The CS6 ACR dialog: **Filmstrip** (multi-image), camera/file label,
full-screen toggle, **image adjustment tabs**, histogram with clipping
toggles, **Camera Raw Settings menu**, zoom controls, **Workflow options**
link, navigation arrows, and the `Save Image` / `Open Image` / `Done` /
`Cancel` actions.

Image-adjustment tabs (CS6 ACR 7): **Basic**, **Tone Curve**, **Detail**,
**HSL / Grayscale**, **Split Toning**, **Lens Corrections**, **Effects**,
**Camera Calibration**, **Presets**, **Snapshots**. Tool shortcuts and every
slider range are tabulated in `FILT-100`; do not duplicate them here.

### ACR preferences

Reachable from Bridge (`Edit > Camera Raw Preferences`), from the ACR dialog
(Open Preferences button), and from Photoshop (`Edit > Preferences > Camera
Raw`, `Ctrl+K`):

- **JPEG and TIFF Handling** — whether JPEGs/TIFFs are automatically opened in
  ACR (and whether unsupported ones are skipped).
- **Cache** — maximum size (default **1 GB**, holding ~200 images/GB), purge,
  and location.
- **Default Image Settings** — per camera model / serial / ISO defaults;
  **Apply Auto Tone Adjustments** to include automatic tone adjustments in
  defaults.
- **Apply Sharpening To** — all images or previews only.
- **DNG File Handling** — `Ignore Sidecar ".XMP" Files` to keep all DNG
  adjustments inside the DNG.
- **Save Image Settings In** — `Camera Raw Database` or `Sidecar ".XMP"`
  (see next).

### Where ACR settings are stored

Choose in preferences:

- **Camera Raw Database** — under the user's Adobe CameraRaw folder; indexed by
  **file content**, so settings survive moving/renaming the raw file. On
  Linux the equivalent is a per-user database keyed by content hash.
- **Sidecar `.XMP` Files** — `<basename>.xmp` beside the raw file; ideal for
  archiving and multi-user exchange; can also carry **IPTC** data. ACR cannot
  write a sidecar on a **read-only volume**, so it falls back to the database.
- **DNG files** — adjustments are stored **inside the DNG**; the sidecar can
  be ignored via the DNG preference.
- **TIFF/JPEG** — settings for these processed files are always stored **in
  the file itself**.
- Image attributes (target profile, bit depth, pixel size, resolution) are
  **not** stored with the settings.

`Export Settings To XMP` copies database settings into sidecars (or embeds
them in DNGs); `Update DNG Previews` refreshes embedded JPEG previews.

### DNG conversion

- **DNG (Digital Negative)** is a non-proprietary, publicly documented raw
  container. ACR can open DNG files without camera-specific knowledge and can
  embed adjustments in the file.
- Convert raw → DNG with the **Adobe DNG Converter** or the **ACR dialog**
  (`Save Image > DNG`). DNG save options include compatibility level, lossless
  vs linear demosaiced, embedded preview, and embedding the original raw
  (`FILT-100`).
- ACR reads only **primary-image** settings from Lightroom-saved XMP, not
  virtual copies.

### Batch raw processing

- **Batch** (`File > Automate > Batch`) runs an action over a folder. For raw
  workflows the PDF recommends: record with **Image Settings** from the Camera
  Raw Settings menu (so per-image database/sidecar settings replay), keep a
  `Save As` step, and use **Override Action "Open" Commands** and **Suppress
  File Open Options Dialogs** so ACR does not prompt per file. **Droplets**
  should set Suppress File Open Options Dialogs too.
- **Image Processor** (`File > Scripts > Image Processor`) processes a folder
  of raw/PSD/JPEG without an action; supports **Open First Image To Apply
  Settings** for a common raw batch, "Resize To Fit", sRGB conversion, and
  "Copyright Info".
- Workflow Options apply to files output from ACR (Space, Depth, Size,
  Resolution, Sharpen For, Open as Smart Objects) and do not affect the raw
  data.

### ACR as the raw-decode engine

For Kooka Pictura, ACR 7's role decomposes into: (a) a **raw decoder** (container
parse + sensor unpack + metadata), (b) the **ACR 7 pipeline** (WB → camera
profile → PV2012 tone → local → detail → lens → effects → output), and (c) the
**settings record**. These belong in `pictura_raw` and are specified in
`FILT-100`. This workflow spec binds them to file open, sidecar storage,
Smart Objects, DNG, and batch.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Bridge File > Open In Camera Raw | Menu | `Ctrl+R` / `Cmd+R` | Primary batch/multi-file entry |
| Photoshop File > Open (raw) | Menu | — | ACR opens before document |
| ACR Open Image + `Shift` | Modifier | `Shift` | Opens as Smart Object |
| ACR Open as Smart Objects (Workflow Options) | Checkbox | — | Preference equivalent |
| Smart Object layer double-click | Gesture | — | Reopens ACR with settings |
| Edit > Preferences > Camera Raw | Dialog | `Ctrl+K` | JPEG/TIFF handling, cache, defaults |
| Bridge Edit > Camera Raw Preferences | Menu | — | Same preferences |
| ACR Open Preferences button | Button | — | Same preferences |
| ACR Workflow options link | Link | — | Space/Depth/Size/Resolution/Sharpen/Smart Objects |
| ACR Save Image | Button | `Alt`-suppress | DNG/JPEG/TIFF/PSD |
| File > Automate > Batch | Menu | — | Batch raw via action |
| File > Scripts > Image Processor | Menu | — | Folder raw batch |
| File > Automate > Create Droplet | Menu | — | Droplet raw batch |
| Bridge Edit > Develop Settings | Menu | — | Copy/paste/apply/clear settings |
| File > File Info | Dialog | — | IPTC in sidecar/DNG (`WF-010`) |

## Parameters & ranges

Workflow-relevant settings (the ACR tab controls themselves are in `FILT-100`):

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Save Image Settings In | Enum | Camera Raw Database | Camera Raw Database / Sidecar ".XMP" | Preference |
| Ignore Sidecar ".XMP" Files | Bool | Off | on/off | DNG File Handling |
| JPEG Handling | Enum | Disable/Enable *(unverified)* | Automatically open / Disable / Enable | Preference |
| TIFF Handling | Enum | Disable/Enable *(unverified)* | Automatically open / Disable / Enable | Preference |
| Cache Maximum Size | Size | 1 GB | user-set *(range unverified)* | ~200 images/GB |
| Apply Sharpening To | Enum | All Images *(unverified)* | All Images / Previews Only | Preference |
| Apply Auto Tone Adjustments | Bool | Off | on/off | Default Image Settings |
| Camera Raw defaults scope | Enum | Camera Model *(unverified)* | Camera Model / Camera / ISO | Preference |
| Workflow Space | Enum | working RGB *(secondary)* | built-in profiles (e.g. ProPhoto RGB) | Per-output, not stored |
| Workflow Depth | Enum | 8 bpc *(secondary)* | 8 / 16 bpc | Per-output |
| Workflow Size | Enum | native | resample sizes; best marked `*` | Per-output |
| Workflow Resolution | Number | document ppi | free | Per-output |
| Workflow Sharpen For | Enum | none *(unverified)* | Screen / Matte Paper / Glossy Paper; Amount Low/Std/High | Per-output |
| Open In Photoshop As Smart Objects | Bool | Off | on/off | `Shift` overrides |
| DNG Compatibility | Enum | current *(unverified)* | version levels | DNG save |
| DNG Linear | Bool | Off | embedded-original / linear | DNG save |

## Algorithms & pipeline

This spec does not restate the ACR pipeline; the observable staging and
behavioral-parity caveat live in `FILT-100` (`## Algorithms & pipeline`). For
the workflow layer:

1. **Acquire** — locate files (Bridge/folder/camera download).
2. **Decode/resolve settings** — for each file, resolve the settings record
   (database by content hash → sidecar XMP → embedded DNG → TIFF/JPEG inline);
   merge with defaults (camera model / serial / ISO).
3. **Render** — run the ACR 7 pipeline exactly as `FILT-100` specifies.
4. **Emit** — open in Photoshop (optionally as a Smart Object wrapping the raw
   source and its `AcrParams`), or save a copy (DNG/JPEG/TIFF/PSD) with output
   sharpening and the chosen Space/Depth/Size.
5. **Persist settings** — write back to the configured store; `Export Settings
   To XMP` migrates database→sidecar; `Update DNG Previews` refreshes previews.
6. **Batch** — repeat 2–5 under Batch / Image Processor / Droplet with dialogs
   suppressed; the Smart Object path preserves editability for later.

Settings records are versioned (Process Version) and must round-trip unknown
`crs:` keys losslessly for Lightroom/Adobe interop.

## Rust module mapping

Proposals (decode/pipeline types are in `FILT-100`; this module owns the
workflow plumbing):

- `pictura_raw::workflow` — `AcrSession` orchestrating resolve → render →
  emit; `InputKind { Raw, Jpeg, Tiff, Dng }`.
- `pictura_raw::settings_store` — `SettingsStore` trait with `DatabaseStore`
  (content-hash keyed), `SidecarStore` (`.xmp` beside source), `DngStore`
  (in-file), `InlineStore` (TIFF/JPEG); `resolve(source) -> AcrParams`.
- `pictura_raw::defaults` — `CameraDefaults { model, serial, iso }` apply/merge.
- `pictura_raw::workflow::output` — `WorkflowOptions { space, depth, size,
  resolution, sharpen_for, amount, smart_object }` and the emit path.
- `pictura_raw::dng` — DNG open/save; embed-or-sidecar policy; preview update.
- `pictura_io::xmp::crs` — `crs:` read/write (shared with `FILT-100`).
- `pictura_core::smart::AcrSource` — Smart Object raw source carrying
  `AcrParams` (`LAY-020`).

Crossing types: `AcrSession`, `AcrParams`, `SettingsStore`, `WorkflowOptions`,
`RawFileHandle`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `CameraRawPreferencesDialog` | `QDialog` | Store choice, JPEG/TIFF handling, cache, defaults, DNG handling, sharpening |
| `OpenRawOptionsDialog` | `QDialog` | Per-open JPEG/TIFF routing and Smart-Object choice |
| `AcrWorkflowOptionsDialog` | `QDialog` | Space/Depth/Size/Resolution/Sharpen For/Smart Objects (shared with `FILT-100`) |
| `SettingsStoreCombo` | `QComboBox` | Database / Sidecar XMP |
| `ExportSettingsToXmpAction` | `QAction` | Migrate database→sidecar / embed in DNG |
| `DngSaveDialog` | `QDialog` | DNG compatibility, linear, preview, embed original |
| `BatchRawPanel` | `QWidget` + `QThread` | Folder batch using an action or Image-Processor-style options, progress, suppression |
| `RawFileListModel` | `QAbstractItemModel` | Bridge-style multi-select, rating, mark-for-deletion |

Widgets over QML for preferences/batch (dense, form-like); the ACR dialog
itself is specified in `FILT-100`. Batch runs on a worker thread with
cancellation (`ARCH-005`).

## Data-model impact

- **No new document node** for raw edits: ACR edits a *source*; the result is a
  layer, or a Smart Object whose `AcrParams` stay editable (`LAY-020`).
- `Document`/source gains a `SettingsBinding` describing where `AcrParams` live
  (database key, sidecar path, or DNG file) so save/round-trip is deterministic.
- **Undo:** while the ACR dialog is open, each `AcrParams` mutation is undoable
  (coordinate with `FILT-100`); committing the session into Photoshop is a
  single history step. Batch is one action invocation.
- **Serialization:** `crs:` XMP and DNG tags must round-trip unknown keys
  byte-exact (`ARCH-008`, `file-formats.md`).
- Workflow Options (Space/Depth/Size/Resolution) are **not** stored with
  settings — they are per-output.
- Sidecar IPTC is shared with `WF-010`.

## Edge cases

- **`Filter > Camera Raw Filter` absent in CS6** — do not expose it as parity;
  gate any equivalent as a post-CS6 extension (`FILT-100`, `OVR-003`).
- **Read-only volume** — cannot write sidecar; fall back to database, or warn
  if neither is writable.
- **No Settings store configured / content hash change** — settings can be
  "lost" after edits to a raw file if using the content-indexed database;
  explain, and prefer sidecar for archiving.
- **Unsupported camera** — decode fails with a clear "camera/file not
  supported" message, no crash.
- **JPEG/TIFF routing** — preference decides; opening a JPEG in ACR edits
  processed pixels and uses the approximate temperature scale (`FILT-100`).
- **CMYK** — converted to RGB on open.
- **Huge images** — 65,000 px / 512 MP ceiling; tile/stream the pipeline.
- **HDR float TIFF/DNG** — ACR 7.1+ under PV2012, ±10 Exposure; output 8/16-bit
  per Workflow Options.
- **Lightroom interop** — primary-image settings only; ignore virtual copies.
- **Batch with prompts** — must suppress file-open dialogs or the batch stalls.
- **DNG round-trip** — preserve unknown DNG tags and embedded original.
- **GPU unavailable** — ACR preview must fall back to CPU (`ARCH-006`).
- **Determinism** — preview and final render identical; no zoom dependence.

## Parity acceptance criteria

- Given a supported raw file opened from a folder/Bridge, ACR 7 opens with the
  full CS6 tab set and shows the Filmstrip for multiple files.
- Given `Save Image` in DNG/JPEG/TIFF/PSD, the copy is written and the original
  raw data is unmodified.
- Given `Open Image` with `Shift`, the result is a Smart Object whose
  double-click reopens ACR with the same `AcrParams`.
- Given sidecar storage selected, edits create `<basename>.xmp` with `crs:`
  keys; on a read-only volume, settings go to the database instead.
- Given a DNG with embedded settings, opening it shows those settings and the
  sidecar is ignored when `Ignore Sidecar ".XMP" Files` is on.
- Given `Export Settings To XMP`, database settings appear in sidecars/DNGs.
- Given a folder batch with dialogs suppressed, all raws are processed with
  their own settings (Image Settings) and saved as configured.
- Given the ACR preferences, the cache can be resized, purged, and relocated.
- Given a committed ACR session, one Undo returns the document to its pre-ACR
  state; batch is a single action invocation.
- Given a Lightroom-saved primary-image XMP sidecar, ACR shows the same
  adjustments (virtual copies excluded).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  CS6 Help corpus (`pdftotext -layout`). Established: "Camera Raw" chapter
  structure (`Introduction to Camera Raw`, `Navigate, open, and save images in
  Camera Raw`, `Manage Camera Raw settings`, etc.); ACR 7 plug-in; raw vs
  JPEG/TIFF; raw data preservation; 65,000 px / 512 MP and CMYK→RGB limits;
  DNG description and adjust-in-file storage; dialog overview and tab list;
  Bridge cache (1 GB default, ~200 images/GB, purge/relocate); `Save Image` /
  `Open Image` (+`Shift` Smart Object) / `Done` / `Cancel`; Snapshots;
  Save/Reset/Load settings; **Specify where Camera Raw settings are stored**
  (database/content-indexed vs sidecar vs DNG vs TIFF/JPEG inline; read-only
  fallback; attributes not stored); `Export Settings To XMP` / `Update DNG
  Previews`; Workflow Options (Space/Depth/Size/Resolution/Sharpen For/Smart
  Objects); Camera Raw preferences (cache, JPEG/TIFF handling, defaults,
  sharpening, DNG file handling); batch/droplet tips (`Override Action "Open"
  Commands`, `Suppress File Open Options Dialogs`, Image Settings, Save As);
  Image Processor options including `Copyright Info`.
- `06-filters/camera-raw-filter.md` (`FILT-100`) — in-repo authoritative
  correction that `Filter > Camera Raw Filter` is post-CS6 (CC), the full tab
  inventory, every slider range, and the raw pipeline. Primary cross-reference.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD/PSB resource context for Smart Object and metadata carriage.

Not fetched / not used: `helpx.adobe.com` (403). The Adobe community thread
cited in `FILT-100` ("no ACR filter in CS6") was read there, not re-fetched for
this document. Adobe's DNG specification was not fetched for this pass.

## Open questions

- **ACR 7.1/7.x minor differences** (HDR float support, chromatic-aberration
  `Color` tab) relative to base 7.0; carried from `FILT-100`. *Resolves with:*
  Adobe ACR release notes.
- **Camera Raw preference enums/defaults.** JPEG/TIFF handling, "Apply
  Sharpening To", default scope, cache clamp. *Resolves with:* a CS6
  preferences capture.
- **Camera Raw database format.** Location, hashing algorithm, schema.
  *Resolves with:* inspection of the ACR database file.
- **`.xmp` sidecar merge precedence** when both a sidecar and a DNG internal
  packet exist. *Resolves with:* controlled CS6 tests.
- **DNG specification version** ACR 7 writes and which tags it preserves.
  *Resolves with:* the Adobe DNG specification.
- **Batch concurrency and determinism** under a multi-threaded Rust pipeline
  with per-file settings caches. *Resolves with:* a performance spike.
- **Smart Object round-trip** — whether the embedded raw bytes or a reference is
  stored, and how it interacts with `LAY-020`. *Resolves with:* inspecting a
  CS6 Smart Object PSD.
