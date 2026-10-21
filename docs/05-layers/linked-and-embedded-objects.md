# Linked & Embedded Smart Objects

- **Spec ID:** `LAY-024`
- **Status:** `Draft`
- **Parity tier:** `Core` for **embedded** Smart Objects (the only kind CS6 has); `Non-goal (CS6 parity)` for **linked** Smart Objects — a CC 2014 feature documented here because the corpus lists it and because the PSD `lnk2`/`lnkD`/`lnk3` blocks matter for round-tripping files from later Photoshop.
- **New in CS6:** `No` — CS6 supports **embedded** Smart Objects only (every `File > Place` embeds the source bytes in the PSD). **Linked** Smart Objects (`Place Linked` / `Convert to Linked`) and **Place Embedded** as a distinct command arrived in **Photoshop CC 2014 (14.2, June 2014)**. The PSD **`lnk2`** key is older (it appears in the spec's PSB 8-byte-length list) and predates user-facing linked objects; **`lnkD`/`lnk3`** are the linked-layer keys. See `## Open questions`.
- **Depends on:** `ARCH-008` document-model, `ARCH-011` file-formats, `LAY-020` smart-objects, `LAY-021` smart-filters, `10-workflow-io/save-and-save-as.md`, `10-workflow-io/open-and-new.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. CS6 embedded behavior is from the fetched CS6 Help
> PDF; linked-object behavior is post-CS6 and marked as such; PSD block facts are
> from the Adobe file-format specification.

## CS6 behavior

### Embedded (the CS6 contract)

In CS6 a Smart Object is **always embedded**: `File > Place`, `Open As Smart Object`, drag-and-drop, paste-from-Illustrator, and `Convert to Smart Object` all copy the source content **into the PSD/PSB**. Consequences the user observes:

- The document is **self-contained**: it opens and renders correctly with no external file present.
- `Layer > Smart Objects > Edit Contents` opens the embedded source in Photoshop or Illustrator; saving updates all linked instances inside the host document.
- `Layer > Smart Objects > Export Contents` writes the embedded source back out in its original format (JPEG/AI/TIF/PDF), or **PSB** if the object was created from layers.
- Duplicating with `Layer > New > Layer Via Copy` creates a second layer that shares the **same embedded source** (edits propagate); `Layer > Smart Objects > New Smart Object Via Copy` makes an **independent** copy with its own source.
- `Layer > Smart Objects > Replace Contents` swaps the embedded source while preserving transform, warp, and effects.

There are **no link-management commands** in CS6: no relink, no link status, no package, no "convert to linked/embedded". A placed file is detached from the original on disk once embedded.

### Linked (post-CS6; reference only)

Linked Smart Objects, introduced in **CC 2014 (14.2)**, store the source as a separate file referenced by path instead of embedding its bytes:

- Creation: `File > Place Linked` (or the modern `File > Place` with an embedded/linked choice), and `Layer > Smart Objects > Convert to Linked`.
- The source can be shared by many documents; saving the source updates every document that links it.
- Link management: **Convert to Linked** / **Embed Linked** (CC 2014), relink/replace a missing source, update when modified, and (later versions) **Package** the document to gather linked assets. Adobe's own help frames the benefit as "update multiple Smart Objects at once".
- A linked object can be **missing/broken**; the host shows a status and renders the last-known or a placeholder until relinked.
- A linked object still supports non-destructive transform and Smart Filters exactly like an embedded one.

**These behaviors are explicitly out of CS6 parity.** They are documented so the data model and PSD reader can preserve them.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Place` | Menu | — | **CS6:** always embeds as a Smart Object |
| `File > Open As Smart Object` | Menu | — | **CS6:** embedded |
| `Layer > Smart Objects > Convert to Smart Object` | Menu | — | **CS6:** embeds the selected layer(s) |
| `Layer > Smart Objects > Edit Contents` | Menu | — | **CS6:** opens the embedded source |
| `Layer > Smart Objects > Replace Contents` | Menu | — | **CS6:** replaces embedded source, keeps transform/effects |
| `Layer > Smart Objects > Export Contents` | Menu | — | **CS6:** writes the embedded source in its original format |
| `Layer > Smart Objects > New Smart Object Via Copy` | Menu | — | **CS6:** independent embedded copy |
| `Layer > New > Layer Via Copy` | Menu | — | **CS6:** linked duplicate sharing the same source |
| `File > Place Linked` | Menu | — | **Post-CS6 (CC 2014)** |
| `Layer > Smart Objects > Convert to Linked` | Menu | — | **Post-CS6 (CC 2014)** |
| `Layer > Smart Objects > Embed Linked` | Menu | — | **Post-CS6 (CC 2014)** |
| Link status / relink | Panel/dialog | — | **Post-CS6**; missing-link indicator |
| `File > Package` | Menu | — | **Post-CS6**; gathers linked assets |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Source kind | enum | Embedded | Embedded / Linked | CS6 = Embedded only |
| Original filename | string | source name | — | Unicode string stored in the linked-layer record |
| File type / creator | 4-char / 4-char | — | — | `lnkD` record fields |
| Link type | enum | — | `liFD` linked file data, `liFE` linked file external, `liFA` linked file alias | From the PSD spec |
| Linked-layer version | int | — | 1–7 | From the PSD spec; newer versions add fields |
| Unique ID | Pascal string | — | — | Per linked file |
| Page number / total pages | int / int | 1 / 1 | ≥ 1 | Multi-page PDF/AI sources |
| Open-descriptor present | bool | — | yes/no | Followed by a descriptor when true |
| mtime (linked) | date | — | version > 3 | Year/Month/Day/Hour/Min/Sec + double seconds |
| File size (linked, `liFE`) | int64 | — | — | Bytes |
| Child document ID | Unicode string | — | version ≥ 5 | For Library/child documents |
| Asset mod time | double | — | version ≥ 6 | |
| Asset locked state | byte | — | version ≥ 7 | For Libraries assets |

## Algorithms & pipeline

### Embedded vs linked resolution

*(inferred)* A `SmartSource` is either:

```text
enum SmartSource {
  Embedded { bytes: Blob, format: FileFormat, page: u32, total_pages: u32 },
  Linked   { uri: Path, unique_id: String, file_type: [u8;4],
             link_type: LiF, mtime: Option<SystemTime>, resolved: bool },
}
```

Rendering resolves a `Linked` source by loading the external file (and re-checking `mtime`), while `Embedded` renders straight from `bytes`. Everything downstream — transform, warp, Smart Filter stack — is identical. A missing link renders a placeholder/error state rather than failing the document.

### PSD serialization

**Embedded sources** use the placed-layer blocks from `LAY-020`: **`plLd`** (Placed Layer, pre-CS3) and **`SoLd`** (Placed Layer Data, CS3+; id `soLD`, version 4, descriptor v16). The embedded bytes live inside the placed-layer structure.

**Linked sources** use the **Linked Layer** block:

- **Key is `lnkD`; also keys `lnk2` and `lnk3`.** *(sourced)*
- The block begins with a per-link record: 8-byte length of the data to follow; 4-byte type (`liFD` linked file data, `liFE` linked file external, `liFA` linked file alias); 4-byte version (1–7); a Pascal-string unique ID; a Unicode string of the original filename; 4-byte file type; 4-byte file creator; an 8-byte length; a 1-byte "file open descriptor" flag followed by a descriptor when true. *(sourced)*
- If the type is `liFE`: a linked-file descriptor, then (version > 3) a Year(4)/Month(1)/Day(1)/Hour(1)/Minute(1)/seconds(8-byte double) timestamp and an 8-byte file size. If the type is `liFA`: 4 zero bytes then an 8-byte zero. If the type is `liFE`: the raw bytes of the file follow. *(sourced)*
- Version ≥ 5 adds a Unicode **Child Document ID**; version ≥ 6 adds an **Asset mod time** (double); version ≥ 7 adds a 1-byte **asset locked state** for Libraries assets. *(sourced)*

**Smart Object content newer than CS6:** **`SoLE`** (Smart Object Layer Data) is labelled **"Photoshop CC 2015"** in the spec (type `soLD`, version 4 or 5, descriptor). A CS6-parity reader must not depend on it, but should preserve it as an unknown block.

**PSB length widths:** `lnk2` is in the spec's list of keys whose lengths are **8 bytes in PSB** (along with `LMsk`, `Lr16`, `Lr32`, `Layr`, `Mt16`, `Mt32`, `Mtrn`, `Alph`, `FMsk`, `FEid`, `FXid`, `PxSD`). See `ARCH-008`.

### File dependencies

- **Embedded:** the PSD has **no external dependencies**; it is portable by construction.
- **Linked:** the PSD depends on each referenced file. Missing/moved assets break the render; a package operation must collect them. *(inferred)* Keep a link table `uri → [NodeId]` so relink/resolve updates all referencing nodes.

## Rust module mapping

CS6-relevant (embedded):

- `pictura_core::smart::SmartSource::Embedded` — bytes + original format + page metadata (`LAY-020`).
- `pictura_io::psd::placed` — `plLd`/`SoLd` read/write; opaque descriptor preservation.

Post-CS6 extension (linked):

- `pictura_core::smart::SmartSource::Linked` — `{ uri, unique_id, file_type, link_type, mtime, resolved }`.
- `pictura_core::link::LinkTable` — `uri → Vec<NodeId>`; `resolve_all`, `relink(old, new)`, `missing() -> Vec<NodeId>`.
- `pictura_io::psd::linked` — `lnkD`/`lnk2`/`lnk3` parse/write; PSB 8-byte lengths; preserve unknown record versions.
- `pictura_io::package` — gather linked assets beside the document.

Crossing types: `SmartSource`, `PathBuf`, `NodeId(u64)`, `LinkStatus { Ok, Missing, Modified }`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SmartObjectPropertiesPage` | `QWidget` | Shows Embedded vs Linked, original filename, format, page; Edit/Replace/Export |
| `LinkStatusDelegate` | `QStyledItemDelegate` | Broken/modified link badge on Smart Object (and linked-layer) rows |
| `RelinkDialog` | `QFileDialog` | Choose a replacement source for a missing link (post-CS6) |
| `PackageDialog` | `QDialog` | Choose destination + collect links (post-CS6) |
| `SourceRevisionWatcher` | `QFileSystemWatcher` | Watch linked files; prompt/auto-update on change (post-CS6) |

## Data-model impact

- Extend `SmartObjectRef` (`ARCH-008`) to a discriminated `SmartSource { Embedded | Linked }`; the CS6 engine only ever constructs `Embedded`.
- A document-level **link table** is needed for linked sources (post-CS6 extension) so relink/resolve and dependency listing are O(links), not a tree scan.
- Undo: `Replace Contents` / `Edit Contents` / `Convert to Linked` / `Embed Linked` / `Relink` are commands; embedded→linked/linked→embedded changes rewrite the block and must be undoable.
- Serialization: `plLd`/`SoLd` (embedded), `SoLE` (CC 2015, preserve), `lnkD`/`lnk2`/`lnk3` (linked). Unknown versions/records preserved verbatim. PSB uses 8-byte lengths for the listed keys.
- External dependency metadata (mtime, file size, asset mod time) is cached in the PSD but must be re-validated on open.
- Smart Filters and transforms are unaffected by whether the source is embedded or linked (`LAY-021`).

## Edge cases

- **CS6 round-trip** — a CS6-authored PSD has only embedded sources; ensure `SoLd`/`plLd` blobs and their descriptors survive byte-for-byte.
- **CC 2014+ linked PSD opened by a CS6-parity engine** — `lnkD`/`lnk3` must be preserved as unknown/known-but-unresolved; the document must open without the external file. Whether PS renders a placeholder or errors is version-specific.
- **Missing link** — link file deleted/moved; show `Missing`, render a placeholder, offer relink; do not corrupt the document.
- **Modified link** — mtime/size changed since save; prompt/auto-update (post-CS6).
- **`liFA` alias** — alias/alias-style link; the spec writes zeros where data would be.
- **Version drift** — `lnkD` version 1–7; unknown future versions must be preserved and not misparsed.
- **PSB** — 8-byte lengths; huge embedded source must stream, not double-buffer.
- **Nested links** — a linked source may itself contain links; resolution must recurse with cycle/depth protection.
- **Packaging collisions** — two different links with the same filename; disambiguate.
- **Security** — resolving a link must not follow untrusted paths or execute anything; treat linked files as untrusted input (see `11-cross-cutting/security-and-sandboxing.md`).
- **Smart Object from layers** — has no original filename; `Export Contents` writes PSB (`LAY-020`).

## Parity acceptance criteria

- Given a CS6 PSD with an embedded Smart Object, opening and re-saving preserves the `SoLd`/`plLd` record and unknown descriptor fields; the object renders identically within tolerance.
- Given `File > Place` in a CS6-parity engine, the placed file is embedded (not referenced), so deleting the original on disk does not affect the document.
- Given `Layer > Smart Objects > Export Contents` on a file-created object, the original format is written; on a layer-created object, PSB is written.
- Given two layers sharing an embedded source (`Layer Via Copy`), editing the source changes both; an independent copy (`New Smart Object Via Copy`) is unaffected.
- Given a PSD from a later Photoshop containing `lnkD`/`lnk3` (possibly version 1–7), a CS6-parity reader parses the documented fields for versions 1–7, preserves unknown trailing fields, and opens the document without the external files.
- Given a linked object whose source is missing, the reader reports a missing-link state and does not fail the open.
- Given a PSB containing a linked-layer block, length fields are read with 8-byte widths where the key list requires it.
- Given a linked source whose file changed since save, the link is reported as modified (post-CS6 extension criterion).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the CS6 Help corpus. Established the **embedded-only** CS6 contract: `File > Place` imports files **as Smart Objects**; create/edit/replace/export/duplicate workflows; Export Contents formats (PSB when created from layers); "Edit one Smart Object and automatically update all its linked instances" (instances means duplicate layers sharing the embedded source, not external links); no link-management commands. Also the `Place Or Drag Raster Images As Smart Objects` preference and the `Liquify`/Blur Gallery smart-filter notes.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe PSD/PSB File Formats Specification. Established: **Linked Layer** block "Key is `lnkD`. Also keys `lnk2` and `lnk3`" with the full record format (per-file length, type `liFD`/`liFE`/`liFA`, version 1–7, Pascal unique ID, Unicode original filename, file type/creator, open descriptor, `liFE` linked-file descriptor + timestamp for version > 3 + file size, `liFA` zeros, raw file bytes, child document ID for version ≥ 5, asset mod time for version ≥ 6, asset locked state for version ≥ 7); `plLd` and `SoLd` embedded placed-layer records; **`SoLE` "Smart Object Layer Data (Photoshop CC 2015)"**; the PSB 8-byte-length key list including `lnk2`.
- `https://bjango.com/articles/photoshopcc2014smartobjects/` — established that **Linked Smart Objects** arrived in **Photoshop CC 14.2 (2014)**, that CC 2014 added **Convert to Linked** and **Embed Linked**, and that these are post-CS6. Used to scope linked-object behavior out of CS6 parity.

Consulted as search-result snippets only (not individually fetched):

- `https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/create-linked-smart-objects.html`, `.../embed-linked-smart-objects.html`, `.../update-linked-smart-objects.html` (HTTP 403 to fetch) — current linked-Smart-Object creation, embedding, and update workflows; `File > Place Linked`.

## Open questions

- **`lnk2` origin and use.** The spec does not date `lnk2`; it is in the PSB 8-byte key list but its user-facing purpose (and whether any CS6 file writes it) is unclear. Resolve from Photoshop-version history and CS6-authored PSDs.
- **`lnk3` vs `lnkD`.** When each key is written, and whether `lnk3` is a CC 2014+ structure distinct from `lnkD`, is not stated. Resolve by parsing CC 2014+ files.
- **`liFD` payload.** Whether "linked file data" (`liFD`) means the linked bytes are cached inside the host while a link is maintained — which would blur embedded vs linked — is unconfirmed. Resolve from reference files.
- **Missing-link rendering in CS6.** Because CS6 has no linked objects, how CS6 itself opens a CC 2014+ `lnkD` file (ignore, placeholder, or error) is untested. Resolve with a CS6 test.
- **`SoLE` taxonomy.** "Smart Object Layer Data (CC 2015)" overlaps conceptually with `SoLd`; the exact relationship and which key wins on save is not sourced.
- **Packaging semantics.** Package, relink, and auto-update rules are post-CS6 and documented only in current Help (403-blocked). Resolve from current Adobe docs if the extension is pursued.
- **Corpus premise.** This file and `LAY-023` contradict the repository's assumption that these features are CS6. Decide whether linked objects are in scope for Kooka Pictura or documented-only; if in scope, add parity criteria against the CS6 (feature-absent) baseline is impossible — they must be treated as extensions.
