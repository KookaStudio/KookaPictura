# File Info and Metadata

- **Spec ID:** `WF-010`
- **Status:** `Draft`
- **Parity tier:** `Core` for the File Info dialog, XMP/EXIF/IPTC carriage, metadata templates, and sidecar reading. `Extended-only` for the DICOM tab (Photoshop Extended).
- **New in CS6:** `No` (the File Info dialog and XMP storage model predate CS6; CS6 changes are cosmetic — e.g. the resizable dialog and the Description field's edit affordances noted in the CS6 "What's new" list are Print-dialog items, not File Info). *(inferred from the CS6 Help corpus; see `## Open questions`.)*
- **Depends on:** `01-architecture/file-formats.md` (format carriage), `ARCH-008` document-model, `ARCH-009` undo-history, `10-workflow-io/save-and-save-as.md`, `10-workflow-io/document-lifecycle.md`, `06-filters/camera-raw-filter.md` (`FILT-100`, `crs:` XMP), `02-ui-ux/panels/note-panel.md` (annotations), `OVR-004` licensing.

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* or *(secondary)* are
> not taken from the primary CS6 Help PDF and are candidates for `## Open
> questions`.

> **Source caveat.** The CS6 Help reference PDF contains **no dedicated File
> Info/metadata chapter**. It documents only (a) the DICOM metadata categories
> (`DICOM files (Photoshop Extended)`), (b) `File Info` as the source for the
> Print `Description` mark, and (c) XMP/IIM in passing. Tab-by-tab File Info
> behavior below therefore rests on **secondary sources** (Adobe Bridge
> documentation, photometadata.org, standards) and is flagged accordingly.

## CS6 behavior

### Entry points

- **File > File Info** (`Alt+Shift+Ctrl+I` / `Option+Shift+Cmd+I`) opens the
  dialog for the current document. `File Info` is listed in the CS6 menu tree
  in the same position as CS5.
- Adobe Bridge exposes a **File > File Info** for selected assets and a
  **Metadata panel**; Bridge is the batch-file surface in the CS6 workflow.
- Metadata is also reachable from the **File Info dialog box** in other CS
  applications (Illustrator, InDesign, Bridge), and Adobe's **Creative Suite
  Color Settings**-style shared settings do not apply here.

### The File Info dialog in CS6

The dialog presents a **left-hand list of categories (tabs/panels)** over one
form pane. In CS6 the categories are *(secondary; names vary by CS generation)*:

| Tab | Content | Editable |
|---|---|---|
| **Description** (called **Basic** in later CC) | Document Title, Author, Author Title, Description, Description Writer, Keywords, Copyright Status, Copyright Notice, Copyright Info URL | Yes |
| **Camera Data** (EXIF) | Camera make/model/serial, lens, exposure (shutter, aperture, ISO, EV, exposure program), focal length, date/time, orientation, image dimensions | No (camera-generated) |
| **IPTC** | Creator, Creator's Job Title, Creator's Contact Info (address, city, state, postal code, country, phone, email, website), Credit Line, Source, Description Writer, Copyright Notice, URL | Yes |
| **IPTC Extension** | Person/Organisation/Product/Artwork shown, Event, Location Created/Shown (structured), model/property release, licensor, rights usage terms, image rating | Yes |
| **GPS Data** | Latitude, longitude, altitude, timestamp, datum | No (camera/GPS-generated); per standards GPS is also writable |
| **Origin** | City, State/Province, Country, Headline, Credit, Source, Instructions (legacy IPTC-IIM fields) | Yes |
| **Video Data** | Video frame metadata (codec, frame rate, alpha), for video layers/documents | Partial |
| **Audio Data** | Audio annotation/audio file metadata | Partial |
| **Photoshop** | Photoshop-specific keys: document ID, version, history/edit log, EXIF-derived and app-generated values | Partial |
| **DICOM** *(Extended)* | Patient, Study, Series, Equipment, Image data categories (see below) | Partial (Image data read-only) |
| **Raw Data** | The **raw XMP packet** for the file (`<x:xmpmeta>…`), editable as text | Yes |

Notes:
- Later Photoshop versions rename **Description → Basic**, add **AEM
  Properties**, and merge some legacy tabs; the CS6 names above are
  *(secondary)* and must be confirmed against a CS6 capture.
- Fields show a **most-recently-used** dropdown on a right-side triangle
  (secondary), speeding repetitive entry.
- Metadata must be **saved with the document** (`File > Save` / `Save As`) to
  be written; closing File Info alone does not persist values *(secondary)*.

### DICOM metadata (Extended, primary)

CS6 documents five DICOM metadata categories editable in **File Info**:

- **Patient data** — patient name, ID, sex, date of birth.
- **Study data** — study ID, referring physician, study date/time, study
  description.
- **Series data** — series number, modality, series date/time, series
  description.
- **Equipment data** — equipment institution, manufacturer.
- **Image data** — transfer syntax, photometric interpretation, image width
  and height, bits per pixel, frames. **These fields are not editable.**
- A measurement scale present in a DICOM file is **imported automatically**; if
  none exists, a default `1 pixel = 1 mm` custom scale is added
  (see `WF-014`).

### XMP storage and embedding

- XMP (Extensible Metadata Platform) is the canonical metadata layer. It is an
  **ISO standard, ISO 16684-1** (Part 1: data model, serialization, core
  properties), originally from Adobe.
- Photoshop **synchronizes** IPTC Core values between XMP and the older
  **IPTC-IIM** byte record still understood by many applications: a subset of
  fields is written to both, and the software keeps them consistent on read and
  write *(secondary, IPTC User Guide)*. IPTC Extension fields exist **only in
  XMP**.
- EXIF camera data is surfaced in the Camera Data tab and preserved; GPS is
  surfaced in the GPS Data tab.
- The **Raw Data** tab shows the serialized XMP packet; edits there write back
  to the same namespace/value model.
- In PSD/PSB the metadata lives in image resource blocks:
  **1028** IPTC-NAA, **1058/1059** EXIF 1/3, **1060** XMP; the ICC profile is
  resource **1039/1041** (see `ARCH-007`). On classic Mac OS the whole File
  Info payload was also mirrored to `'ANPA'` resource 10000 as an IPTC-NAA
  record 2 (legacy, irrelevant to Linux parity but explains round-trip keys).

### Metadata templates

- **Export**: set fields, then **Export** the template from the File Info
  dialog (CS6 uses a Template/Import-Export control; older CS used a flyout
  Save). Templates are saved as **XMP** files, by default in the Photoshop
  **Metadata Templates** folder.
- **Apply**: import a template from the dialog and choose how it combines with
  existing data. CS6 offers **Append** (add only empty fields), **Replace**
  (overwrite), and a **"Keep original metadata but replace matching
  properties"** mode *(secondary; exact wording varies by version)*.
- Templates are plain XMP, so they are debuggable and portable.

### Write targets

- **PSD/PSB** — image resources 1028/1058/1059/1060.
- **TIFF** — standard TIFF/EXIF/IPTC/XMP tags (Photoshop TIFF tags per the
  Adobe File Formats spec).
- **JPEG** — XMP in APP1, IPTC-IIM in APP13; `Save for Web` exposes a metadata
  choice (`None`, `Copyright Only`, `All Except Camera Info`, `All`).
- **PDF** — XMP packet plus document Info; Save Adobe PDF has an Output section
  including PDF/X and metadata handling.
- **Other formats** carry metadata per their own specs; formats with no
  metadata channel must warn/drop.

### Reading sidecars

- Camera raw formats that cannot carry embedded metadata use **XMP sidecars**
  named `<basename>.xmp` beside the raw file (see `FILT-100` and `WF-012`).
- Photoshop/Bridge read sidecars for raw files and for files on read-only
  volumes where in-file writes are impossible.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| File > File Info | Dialog | `Alt+Shift+Ctrl+I` / `Option+Shift+Cmd+I` | Main editor |
| Bridge File > File Info | Dialog | — | Batch/asset surface |
| Bridge Metadata panel | Panel | — | Read/edit selected assets |
| File Info — left category list | Tab strip | — | Description/Basic, Camera Data, IPTC, IPTC Extension, GPS, Origin, Video, Audio, Photoshop, DICOM, Raw Data |
| File Info — MRU triangles | Drop-down | — | Recently used values |
| File Info — Template control | Button/menu | — | Export/Import/Append/Replace |
| File > Save / Save As | Menu | `Ctrl+S` / `Ctrl+Shift+S` | Persists metadata |
| File > Print — Output > Description | Checkbox | — | Prints File Info Description (~300 chars) |
| File > Save for Web — Metadata menu | Menu | — | None / Copyright Only / All Except Camera Info / All |
| File > Save As Adobe PDF — Output | Pane | — | PDF/X, metadata preservation |
| Camera Raw — IPTC in sidecar XMP | via ACR | — | See `WF-012` |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Description / Basic fields | Text | Empty | Free text | Title, Author, Description, Keywords, … |
| Copyright Status | Enum | Unknown | Unknown / Copyrighted / Public Domain *(secondary)* | |
| Copyright Notice | Text | Empty | Free text | `©` conventional |
| Keywords | List | Empty | Free text, comma/tag entry | Core schema unbounded in XMP |
| Creator / Credit / Source | Text | Empty | Free text | Core + legacy IIM |
| IPTC Extension structures | Nested | Empty | Person/Org/Product/Artwork/Event/Location | XMP-only |
| GPS latitude/longitude | Numeric | From EXIF | ±90 / ±180, DMS or decimal | Tab read-only in practice |
| Camera Data | Read-only | From EXIF | — | Make, model, exposure, lens |
| DICOM Patient/Study/Series/Equipment | Text | From file | Free text | Extended only |
| DICOM Image data | Read-only | From file | Transfer syntax, dims, bits, frames | Not editable |
| Raw XMP | Multiline text | Current packet | Well-formed XMP XML | Namespace-aware edit |
| Metadata template | File | None | `.xmp` | Default Metadata Templates folder |
| Template apply mode | Enum | Replace *(secondary)* | Append / Replace / Keep-original-replace-matching | Wording version-dependent |
| Save for Web metadata | Enum | All Except Camera Info *(secondary)* | None / Copyright Only / All Except Camera Info / All | JPEG full; GIF/PNG partial |

## Algorithms & pipeline

*(Behavioral parity, no closed Adobe algorithm is being reproduced; the
pipeline below is the standard/XMP model.)*

1. **Model.** Metadata is an XMP **data model** (RDF-like property triples)
   serialized as XML. Core namespaces include `dc:` (Dublin Core), `xmp:`,
   `photoshop:`, `exif:`, `tiff:`, `crs:` (Camera Raw), `Iptc4xmpCore:`,
   `Iptc4xmpExt:`, `plus:`, and `GPano`/geo where applicable. Unknown
   namespaces must be preserved verbatim on round-trip.
2. **Read.** Parse the container (PSD resources, TIFF tags, JPEG APP1/APP13,
   PDF XMP/Info, sidecar), collect all metadata packets, and merge them into
   one in-memory store. When IIM and XMP disagree, apply the synchronization
   rules (prefer XMP for XMP-only fields; reconcile shared fields).
3. **Edit.** The dialog edits the store; structure edits (IPTC Extension,
   location structures) edit nested property graphs.
4. **Write.** Serialize the store back to every supported channel for the
   chosen format; preserve unknown keys and unknown PSD resources/TIFF tags.
   Apply the File Info legacy mapping (Copyright → resource 1034 flag, URL →
   1035, IPTC-NAA → 1028) so non-XMP readers still see core values.
5. **Templates.** A template is an XMP file; applying it is a merge policy over
   the store (append/replace/matching-only).
6. **Sidecars.** For raw/read-only sources keep `crs:` and IPTC in the sidecar;
   never silently copy sidecar edits into a JPEG (that is an explicit export).

## Rust module mapping

Proposals:

- `pictura_io::xmp` — `XmpPacket` (ordered property tree + raw bytes),
  `Namespace`, `PropertyPath`; lossless parse/serialize; unknown-key retention.
  Candidate backing: `xmp-toolkit` bindings or a pure-Rust RDF/XML reader;
  **decision deferred**.
- `pictura_io::exif` — `ExifData` (typed tags), read/write for TIFF/JPEG/PSD
  resource 1058/1059; GPS block.
- `pictura_io::iptc` — `IptcCore`, `IptcExtension` typed structs plus
  `IimRecord` byte codec for IPTC-IIM (JPEG APP13 / PSD 1028) and the
  XMP↔IIM synchronization table.
- `pictura_io::psd::resources` — typed access to resources 1028/1034/1035/
  1058/1059/1060, with opaque fall-through for unknown IDs.
- `pictura_io::metadata` — `MetadataStore { xmp, exif, iptc, dng?, dicom?,
  unknown }`, `merge`, `apply_template`, `to_format(Format)`, `from_format`.
- `pictura_io::metadata::template` — `MetadataTemplate` load/save, merge
  policy enum.
- `pictura_io::dicom` *(Extended)* — dataset reader for the File Info
  categories and the embedded measurement scale.
- `pictura_io::sidecar` — sidecar path resolution and read/write, shared with
  `pictura_raw` (`FILT-100`).

Crossing types: `MetadataStore`, `XmpPacket`, `MetadataTemplate`, `MergePolicy`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `FileInfoDialog` | `QDialog` | Category list + stacked form; Save/OK/Cancel; template control |
| `MetadataCategoryList` | `QListView` | Vertical category list (Description…Raw Data) |
| `MetadataFormView` | `QWidget`/`QScrollArea` | Typed field editors per schema |
| `XmpRawEditor` | `QPlainTextEdit` | Raw XMP edit with syntax highlight + validation |
| `MetadataTemplateMenu` | `QToolButton`+`QMenu` | Export/Import/Append/Replace/Keep-original |
| `DicomMetadataForm` | `QWidget` | Patient/Study/Series/Equipment/Image sub-forms (read-only Image) |
| `MetadataFieldDelegate` | `QStyledItemDelegate` | MRU dropdown, validation, date/number editors |
| `BridgeMetadataPanel` | `QDockWidget` | Read/edit for selected assets (Bridge surface) |

Widgets over QML: dense, keyboard-navigable, schema-driven forms match the
shell decision in `ARCH-003`. A generic `MetadataFormView` is preferred to one
hand-written widget per tab.

## Data-model impact

- `Document` gains `metadata: MetadataStore` (XMP/EXIF/IPTC/DICOM + opaque
  blobs). The store is document-level, not per layer.
- Serialization must be lossless for unknown resources/tags/namespaces
  (`ARCH-008`, `file-formats.md`). PSD writes 1028/1058/1059/1060; TIFF/JPEG/
  PDF write their native channels.
- **Undo:** File Info edits are one history command per dialog accept (not per
  keystroke). Template import is one command storing the pre-merge store.
  Raw-XMP text edits may be a single command. Record shape: before/after store
  diff (or the XMP packet before/after).
- **Templates** are external files, not document model state.
- **Sidecar association** is a document→sidecar path binding for raw sources.
- Metadata is excluded from the raster composite and from History snapshot
  pixels.

## Edge cases

- **Format without a metadata channel** — warn/drop; do not silently lose data.
- **Read-only source (CD/DVD/network)** — fall back to sidecar/database for
  raw; for non-raw, warn that edits cannot be saved in place.
- **IIM/XMP divergence** — define precedence and never write conflicting
  duplicates; IPTC Extension is XMP-only.
- **Unknown namespaces / unknown PSD resources** — round-trip byte-exact.
- **Character encoding** — XMP is UTF-8; IIM is byte-oriented and can corrupt
  non-ASCII; surface an encoding warning when writing IIM.
- **Huge XMP packets** — the History Log and Camera Raw settings can bloat
  files; keep the Raw Data tab lazy (parse on open, not on document open).
- **GPS absent / no EXIF** — Camera Data and GPS Data tabs show empty, not
  errors.
- **Video/Audio tabs** — only meaningful for video/audio layer documents; hide
  otherwise *(secondary)*.
- **DICOM Image data** — read-only; attempts to edit are rejected.
- **16/32-bit, CMYK/Lab** — metadata is color-model independent.
- **Undo/redo** — reverting an edit restores the full prior store.
- **PSB/PSD round-trip** — resource padding must be even; Unicode vs Pascal
  string rules differ per resource.

## Parity acceptance criteria

- Given a document, `File > File Info` opens with the category list and the
  DICOM category present only in an Extended build.
- Given edits in Description/IPTC/IPTC Extension, saving a PSD then reopening
  restores every value, and the values are also recoverable from the raw XMP
  packet and the IPTC-NAA resource where applicable.
- Given a Camera Data tab, camera make/model/exposure values match the source
  EXIF within the format's precision.
- Given a template exported as `.xmp` and applied to a second image with
  "keep original metadata but replace matching properties", image-specific
  camera data is preserved and template-owned fields are replaced.
- Given a raw file with a sidecar XMP, opening the raw shows the sidecar's
  IPTC/Camera Raw values.
- Given `Save for Web` metadata `None`, the saved JPEG contains no XMP/IPTC
  (except the EXIF copyright notice); `All` round-trips the full store.
- Given a DICOM file with a measurement scale, the scale is imported
  (see `WF-014`); Image data fields are read-only.
- Given a format without metadata support, export warns and produces a valid
  file.
- Given edits then one Undo, the metadata store returns to its prior state.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  the CS6 Help corpus (downloaded and converted with `pdftotext -layout`).
  Established: the DICOM categories editable in File Info (Patient, Study,
  Series, Equipment, Image; Image read-only); automatic DICOM measurement-scale
  import; the Print `Description` mark reading File Info text (~300 chars);
  Save-for-Web metadata choices; Save-Adobe-PDF Output/PDF-X; Image Processor
  "Copyright Info"/"Include ICC Profile"; the `Blend Text Colors Using Gamma`
  File Info-adjacent Color Settings item. **Does not contain a File Info
  chapter.**
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — Adobe Photoshop File Formats Specification: image resources 1028
  (IPTC-NAA), 1034 (copyright flag), 1035 (URL), 1058/1059 (EXIF 1/3), 1060
  (XMP), 1039/1041 (ICC); the Mac `'ANPA'` resource 10000 File Info mirror;
  TIFF tag handling. Establishes the PSD/format carriage contract.
- `https://developer.adobe.com/xmp/docs/` — XMP as an open labeling
  technology, XMP SDKs, ISO 16684-1 status. Establishes the XMP contract.
- `https://iptc.org/std/photometadata/documentation/userguide/` — IPTC Photo
  Metadata User Guide: Core schema 1.5 / Extension schema 1.9; IPTC Core
  embeds in IIM and/or XMP with synchronization, Extension is XMP-only;
  JPEG APP1 (XMP) / APP13 (IIM) headers; full field reference including
  Location Created/Shown, model/property release, rights terms. Establishes
  the IPTC contract.
- `https://photometadata.org/meta-tutorials-adobe-photoshop` — File Info
  panels/tabs (Description, Camera Data 1/2 → single Camera Data tab in CS4,
  Origin, IPTC), CS4 tab-vs-panel UI, XMP containing IPTC Core plus IIM
  synchronization, Save required to persist, raw/DNG routing through ACR.
  Secondary/community.
- `https://photometadata.org/META-Tutorials-Adobe-Photoshop-Creating-Metadata-Template`
  — metadata template creation, CS4 Export control, default templates folder.
  Secondary/community.
- `https://www.bwillcreative.com/how-to-add-copyright-and-edit-metadata-in-photoshop/`
  — modern File Info tab inventory (Basic/Description, IPTC, Camera Data,
  Origin, IPTC Extension, GPS Data, Photoshop, Raw Data, Audio Data, Video
  Data, DICOM, AEM Properties); template export/import and the
  "keep original metadata but replace matching properties" mode; Export As
  "Copyright and Contact Info". Secondary; confirms the CS6 tab family but not
  version-exact.
- `http://www.updig.org/guidelines/ir_icc_profiles.html` — cross-check of
  metadata/color interop (used mainly by `WF-011`).

Not fetched / not used: `helpx.adobe.com` (HTTP 403, per `README.md`). The
CIPA Exif standard PDF was retrieved but is an encrypted binary and was **not
parsed**; no Exif field claims here rest on it.

## Open questions

- **Exact CS6 File Info tab names and order.** Sources disagree (Description
  vs Basic; whether Video/Audio/DICOM appear only conditionally). *Resolves
  with:* a CS6 UI capture or the CS6 Help "Add metadata" page via an archive.
- **Template merge mode wording in CS6.** Append/Replace/Keep-original names
  and defaults are unverified for v13. *Resolves with:* a CS6 capture.
- **IIM↔XMP precedence rules** and the exact synchronized field set. *Resolves
  with:* the IPTC specification document plus controlled CS6 tests.
- **Exif version and field subset** Photoshop CS6 reads/writes (Exif 2.2 vs
  2.3), and GPS write behavior. *Resolves with:* a parsed Exif 2.3 standard
  and CS6 round-trip tests.
- **DICOM tag mapping.** Which DICOM tags populate each File Info category,
  and whether the scale is stored in PSD resource 1074 on save. *Resolves
  with:* the DICOM standard plus a CS6-saved sample.
- **Undo granularity** for a single dialog accept vs per-field. *Resolves
  with:* a CS6 observation.
- **PSD round-trip fidelity** of unknown XMP namespaces and the 1028 IIM
  record (reserialized vs byte-preserved). *Resolves with:* inspecting
  CS6-written PSDs.
- **XMP library choice in Rust.** `xmp-toolkit` bindings vs a pure-Rust
  implementation; affects licensing and byte fidelity. *Resolves with:* a
  licence/quality evaluation.
