# Artboards

- **Spec ID:** `LAY-023`
- **Status:** `Draft`
- **Parity tier:** `Non-goal (CS6 parity)` — artboards are **not** a Photoshop CS6 feature. Documented here because `INDEX.md` and `ARCH-008` list them; the corpus premise must be corrected. Implementing them would be a post-CS6 extension, not parity.
- **New in CS6:** `No` — **correction:** artboards were introduced to Photoshop in **CC 2015 (version 16.0)**, three years after CS6. The Adobe PSD specification itself labels the artboard block **"Artboard Data (Photoshop CC 2015)"**. Illustrator had artboards long before Photoshop; the feature was ported to Photoshop post-CS6, not in CS6.
- **Depends on:** `ARCH-008` document-model, `LAY-020` smart-objects, `10-workflow-io/export-formats.md`, `10-workflow-io/save-and-save-as.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. **This file is a post-CS6 extension proposal, not a
> CS6 parity contract.** Artboard facts are from the Adobe PSD specification
> (which dates the block to CC 2015) and from community/Adobe support statements;
> anything else is marked *(inferred)*.

## CS6 behavior

**None — artboards do not exist in Photoshop CS6.** This section is deliberately negative so the parity contract is unambiguous.

What CS6 has instead of artboards:

- **Multiple documents.** Each design region is its own `.psd`/`.psb` document; there is no single canvas holding several bounded regions.
- **Layer groups** for organising a region's content, and **layer comps** for saving visibility/position/appearance variants (`LAY-022`).
- **Crop / canvas size** (`04-image-ops/canvas-size.md`) to define one working area.
- **Slices** and export scripts for cutting one canvas into regions (`10-workflow-io/web-export-and-slices.md`).

Evidence: the CS6 Help PDF's only occurrences of "artboard" are in an **Illustrator** topic ("Clip To Artboard (Illustrator only)"), not a Photoshop feature. Adobe's community answer states that artboards were added to Photoshop CC and are not available in CS6; the Photoshop version history places them in 16.0 (CC 2015).

### Post-CS6 behavior (what the feature does, for reference)

Artboards let a single document hold multiple canvases. A document is created with artboards via `File > New` and an **Artboard** document type / preset (sizes such as common device and print presets). Each artboard is a top-level container holding its own content; the surrounding application window shows one or more artboards on a pasteboard. Observable behaviors commonly documented for current Photoshop:

- Create artboards when creating the document or convert an existing document; add/duplicate artboards.
- Select, move, and resize an artboard as a unit using the canvas handles or the Properties panel; content is positioned relative to its artboard.
- Per-artboard background/content; artboards behave like special layer groups and can contain layers, groups, and effects.
- Export: `File > Export > Artboards to Files` (one file per artboard) and `File > Export > Artboards to PDF` (one page per artboard). `File > Export > Export As` can also target artboards.
- Limitations commonly cited: artboards are not supported alongside certain features/modes; a document's artboards share the same color mode, bit depth, and resolution. *(Specific CS6-era limitations are moot; current-version limitations are documented in `## Open questions` because they are not in the fetched CS6 material.)*

## UI surface

CS6 has **no** artboard UI. The following is the post-CS6 surface, listed for reference only.

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > New` — Artboard document type | Dialog | `Ctrl/Cmd+N` | Post-CS6 (CC 2015+) |
| Properties panel — artboard W/H/X/Y | Panel | — | Move/resize the selected artboard |
| Canvas handles on a selected artboard | Gesture | `Drag` | Move/resize |
| Layers panel | Panel | `F7` | Artboards appear as top-level container rows |
| `File > Export > Artboards to Files` | Menu (script) | — | One file per artboard |
| `File > Export > Artboards to PDF` | Menu (script) | — | One page per artboard |
| Artboard context menu | Context menu | `Right-click` | Add/duplicate/delete artboard *(inferred from current UI)* |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Artboard name | string | "Artboard N" | free text | Post-CS6 |
| Artboard X / Y | int (px) | 0 / 0 | document pasteboard, unbounded | Position in document space |
| Artboard W / H | int (px) | preset | 1–30,000 (PSD) / 300,000 (PSB) | Shares document size limits (`ARCH-008`) |
| Artboard background | enum/color | transparent/white | per artboard | *(inferred)* |
| Document type | enum | — | Artboard presets | `File > New` |

## Algorithms & pipeline

### Container semantics (post-CS6)

*(inferred)* An artboard is a container node with a bounding rectangle and its own coordinate origin, composited into the document pasteboard at its rectangle offset. Rendering a document with artboards is equivalent to rendering each artboard subtree clipped to its rectangle, then compositing the results at their offsets. Export renders one artboard per output page/file.

### PSD serialization

The Adobe PSD specification defines an **Artboard Data** additional-layer-information block:

- **Key is `artb`, `artd`, or `abdd`.** *(sourced)*
- **Version `16`**, followed by a variable **descriptor based on the Action file-format descriptor structure** (i.e. the same keyed-descriptor encoding used for other layer settings). *(sourced)*
- The block is labelled **"Artboard Data (Photoshop CC 2015)"** in the specification. *(sourced)*

The inner descriptor keys (rectangle, name, background, clip/bleed, whether it is the last artboard) are not enumerated in the fetched spec text — see `## Open questions`. Note the related `vstk`/`vscg` "Vector Stroke" blocks are CS6, but the artboard block is not.

### Relationship to `ARCH-008`

`ARCH-008` already sketches `NodeKind::Artboard{rect}` and states artboards are "New in CS6". The node kind is fine to keep as a forward-looking container; **the "New in CS6" claim is wrong and should be corrected** (artboards are CC 2015). No CS6-authored file will contain `artb`/`artd`/`abdd`, so a CS6-only implementation can safely treat the block as an unknown additional-layer block to preserve verbatim.

## Rust module mapping

Post-CS6 proposal only:

- `pictura_core::node::NodeKind::Artboard { rect: Rect }` — container node (already sketched in `ARCH-008`).
- `pictura_core::artboard` — `Artboard { id, rect, name, background, children }`; document-level artboard enumeration and ordering.
- `pictura_io::psd::artboard` — parse/write `artb`/`artd`/`abdd` descriptor; preserve unknown keys verbatim.
- `pictura_export::artboards` — render one artboard per file/page; reuses the export pipeline.

Crossing types: `NodeId(u64)`, `Rect`, `ArtboardId`, and export request records.

## Qt6 component mapping

Post-CS6 proposal only:

| Proposal | Base | Responsibility |
|---|---|---|
| `ArtboardModel` | `QAbstractListModel` | Document artboards: name, rect, thumbnail, order |
| `ArtboardOverlay` | `QGraphicsItem` | Draw/select/move/resize artboards on the pasteboard |
| `PropertiesPanel` artboard page | `QWidget` | X/Y/W/H, name, background |
| `ExportArtboardsDialog` | `QDialog` | Files/PDF destination, prefix, format, quality |

## Data-model impact

- A document may contain zero or more artboards; a CS6 document contains zero. Artboards are top-level containers, not regular groups.
- If implemented as a post-CS6 extension, serialization uses `artb`/`artd`/`abdd` (descriptor v16). A CS6-parity engine that does not implement artboards **must preserve the block as an unknown additional-layer block** so files from later Photoshop round-trip byte-for-byte.
- Undo: create/move/resize/delete/rename an artboard are commands; moving an artboard should move its subtree.
- Interplay with layer comps and smart objects is post-CS6 and not part of this spec's parity criteria.

## Edge cases

- **CS6 file with no artboard block** — normal case; nothing to do.
- **CC 2015+ file opened by a CS6-parity engine** — treat `artb`/`artd`/`abdd` as unknown, preserve verbatim, and render the document as a normal canvas (artboard content may be spread across the pasteboard). Do not crash.
- **Artboard larger than PSD limit** — 30,000 px (PSD) / 300,000 px (PSB) per dimension (`ARCH-008`).
- **Overlapping artboards** — pasteboard overlap; each renders clipped to its rectangle.
- **Empty artboard** — zero-content container is legal.
- **Color mode/depth** — artboards in one document share the document's mode and bit depth.
- **Export with missing/unsupported script** — fail clearly, do not crash.
- **Unknown descriptor keys** — preserve.

## Parity acceptance criteria

Because artboards are out of CS6 parity, the testable statements are about **correct non-support and round-trip preservation**, not about implementing the feature:

- Given a CS6 document, the UI exposes no artboard commands or panel.
- Given a CC 2015+ PSD containing `artb`/`artd`/`abdd`, opening and re-saving preserves the block byte-for-byte even though the engine does not interpret it.
- Given a CC 2015+ PSD with artboards opened in a CS6-parity engine, the document opens without error and its layers are addressable; the render is the flattened/pasteboard composite, explicitly marked as non-parity.
- If the project later opts into the artboard extension: given two artboards, `File > Export > Artboards to PDF` produces a two-page PDF, one page per artboard.

## Sources

Fetched for this document:

- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe PSD/PSB File Formats Specification. Established the authoritative version fact: the spec labels the block **Artboard Data (Photoshop CC 2015)**, keyed `artb`, `artd`, or `abdd`, version `16`, a variable Action-format descriptor.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the CS6 Help corpus. Established the **absence** of artboards from Photoshop CS6: the only "artboard" occurrence is `Clip To Artboard (Illustrator only)` in an Illustrator topic. (`ARCH-008`'s "New in CS6 — CS6 adds artboards" claim is therefore unsupported by this source.)

Consulted as search-result snippets only (not individually fetched; community/support):

- `https://community.adobe.com/questions-712/how-to-put-artboards-in-photoshop-cs6-1086264` — states that artboards were added to Photoshop CC and are not available in CS6.
- `https://www.reddit.com/r/photoshop/comments/6tcw9b/artboards_in_cs6/` — places artboards at version 16.0 (CC 2015).
- `https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/create-artboard-documents.html` (HTTP 403 to fetch; seen as a search result) — current artboard creation workflow.
- `https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-artboards-as-pdf.html` (HTTP 403 to fetch; seen as a search result) — `File > Export > Artboards To PDF`, and `Artboards to Files`.

## Open questions

- **Corpus correction.** `INDEX.md` and `ARCH-008` state artboards are "New in CS6". This file contradicts them with sourced evidence. Decide whether `LAY-023` should be reclassified as a post-CS6 extension spec or dropped; update `ARCH-008`'s New-in-CS6 line accordingly.
- **`artb`/`artd`/`abdd` descriptor schema.** The inner keys (rect, name, background, index, last-artboard flag) are not enumerated in the fetched spec. Resolve by parsing CC 2015+ PSDs with artboards (`psd-tools`/`PhotoshopAPI` have partial support).
- **Which key is canonical.** The spec lists three keys without saying when each is written. Resolve from reference files.
- **Exact artboard limitations in current Photoshop.** Which features/modes are incompatible is not sourced. Resolve from the current Help (403-blocked) or a test matrix.
- **Export option sets.** The precise `Artboards to Files`/`to PDF` options (format, quality, prefix, transparency) are not sourced. Resolve from the current dialog.
- **Scope decision.** Whether Kooka Pictura implements artboards at all is a product decision, not a parity requirement; if implemented, this spec must be rewritten as a normal (non-negative) contract.
