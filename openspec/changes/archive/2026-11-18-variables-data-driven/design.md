# Design

## Context

The command catalogue in `pictura-script/src/command_catalogue/entries.rs` owns the five stub entries. Layer comps already use PSD `lyid` blocks and resource `1044` for identity, a document store carried by `Snapshot`, and a scratch document for export. Variables need the same identity and lifecycle plus editable content replacement. The existing type replacement operation in `pictura-render/src/type_layer.rs` refuses blank text and authors the model's supported type run in 8-bit RGB documents. The PSD resource reader accepts `MeSa`, while the existing `frame_image_resource` helper emits `8BIM`.

## Goals / Non-Goals

**Goals**

- Use the same validated data set evaluation for preview, Apply, and every exported document.
- Preserve stable layer identity, unrelated PSD data, and document history across operations.
- Use public reference files for independent observations and generated fixtures for repeatable checks.

**Non-Goals**

- Photoshop scripting DOM extensions, non-PSD batch formats, conversion of procedural layers to pixels, and native Photoshop write verification as a release gate.

## Decisions

### Document ownership and edits

Store one variable per name and kind, with an internal identity for its values, and bindings that carry a stable layer ID plus binding-specific placement options. A variable rename updates its name once. Linked layers can use different alignment or placement settings with the same file value. Keep the ordered data set collection in document metadata so history, Duplicate Document, recovery, and scratch clones follow the existing document snapshot path. Share the store across clones until it is edited. Use the layer-comp ID helpers and make the ID seed lifecycle independent of whether comps exist. Addressing a layer by display path would make reorder and duplicate operations ambiguous. For current-state defaults, use the first eligible binding in top-to-bottom stack order, visiting each group before its children; fail initialization if no eligible binding remains. Pixel defaults remain Do Not Replace.

Explicit delete operations remove bindings in the same document transaction. Re-check eligibility at evaluation time because a layer can change kind after the original binding. The Variables dialog stages both metadata and any assigned layer IDs, then commits once through the existing metadata edit path. Keep existing text editing behavior at its current entry point. Factor the type content update so variable replacement can author editable blank text and preserve the supported layout and style. Keep the established type engine mode limits and produce a clear refusal when a set reaches an unsupported target.

### Apply, preview, and export

Resolve targets and assets and validate placement against a copy of the current document before committing. Use existing layer, type, decoder, and mode/profile conversion paths. For raster replacement, compute the placement from the pre-operation rectangle, resample with alpha-aware sampling, write in the document's mode/depth, and invalidate old raster channel authority through the existing checked transitions. Retain layer properties and the document-space mask position. An unsupported conversion or allocation failure returns an error before the live commit. Commit the staged result once through the edit transaction and let it refresh the composite and record history.

The dialog and Apply picker use the same immutable preview baseline and an isolated preview presentation. Preserve the live document, editor selection and history while cycling rows. Export also starts every selected set from one baseline. Keep one working output at a time and use the existing PSD serialization path with the updated composite and document profile. Apply resolves the current layer frame at the start of each explicit command. Preview cycling and batch export each resolve from their fixed baseline.

### PSD resources

Inspect files through the shared bounds-checked resource reader. Capture the original `lyid` multiplicities and original layer ID blocks before the layer-comp reader can normalize or repair IDs. Resolve variable references only against this original identity evidence, never against repaired IDs; duplicate IDs remain unresolved. Preserve affected original ID blocks and dependent variable and comp resource bytes through unrelated saves so reopening cannot silently make an ambiguous reference unique. Refuse edits that would overwrite those preserved identities or opaque dependent comp data. Both eager and lazy desktop-open paths invoke the same metadata resolution after the layer tree is available, before exposing template commands. The observed wire format uses an empty resource name and the `MeSa` signature for both resources. `7000` has a `variableSets` root in `http://ns.adobe.com/Variables/1.0/`, one `variableSet`, and `variable` entries with `varName`, `trait`, `docRef` and placement attributes. `7001` uses an unnamespaced `sampleDataSets` root with `psdPath`, named `sampleDataSet` rows, and child tags keyed by variable name. Treat those tag names as literal XML names to retain admitted colon-bearing variables. Parse strictly within codec budgets. Reject unresolved XML structure and preserve its raw bytes. Keep an untouched or partly unrecognized resource available for byte preservation and expose the read limitation if a subsequent edit would overwrite it.

The [Photoshop 2026 template](https://www.usenativ.com/blog/photoshop-language-versions-csv/) provides `textcontent` and Unicode examples. The [older tutorial](https://manifestationccs.com/tutorials/01-photoshop-variables-data-sets/) provides `fileref`, `placementMethod="ratio"`, and an exported PSD that retains both resources. A [firsthand resource edit](https://github.com/psd-tools/psd-tools/issues/375#issuecomment-1775659783) identifies `fit` and reports successful recognition with `MeSa` for `7000`. For the observed `ratio` / `center` / `middle` / `clip=false` combination, the tutorial archive’s `Tutorial 01 - Engaging.pdf`, Focus 02, pages 6–7, explicitly selects As Is for each of the five pixel variables. Its exported PSD and companion PNGs provide a geometry cross-check: its exported layer rectangles retain the corresponding PNG dimensions even where source and template frame dimensions differ. Other `ratio` combinations, remaining placement tokens, visibility values, and the representation of Do Not Replace need independent semantic evidence before interpretation; preserve and report affected bindings as unsupported until then. Recognizing a token without knowing its semantics is insufficient. Unrecognized wire values are preserved and reported. Use a single explicit wire mapping for authored values. Give any Kooka-specific fallback a distinguishable representation, document it in the implementation record, and decode it for Kooka save/reopen without treating it as a Photoshop value; native validation remains follow-up. Keep `psdPath` as preserved template provenance on untouched save. Resolve usable image paths from the source template or import location and retain inaccessible foreign paths as editable file choices. Opening a template only parses references. Image decoding happens when a set is evaluated.

Reuse the existing `quick-xml` dependency version already in the workspace if a parser dependency is needed in `pictura-codec`. Add a narrow signature-aware framing path while keeping the existing public helper's `8BIM` contract. Chain variable, comp, and other image-resource updates over the same section in writer order. Store the active layer IDs inline as `lyid` and advance their shared `1044` seed. Keep a no-resources fast path for legacy documents.

Kooka-authored resources identify their representation with `kp:format="1"` in `urn:kooka-pictura:variable-data:1`. This is the authority for Kooka-only Fill and Conform tokens, unverified alignment and clipping combinations, boolean data-set values, and the `kp:keep="1"` value. Observed Photoshop wire values remain readable without the marker.

### Import and output boundaries

Decode bytes at the file edge, then use one quote-aware parser for CSV and TSV with field and record locations. Detect an unquoted header delimiter; reject a header with both. With no header delimiter, use `.csv` for comma and `.tsv` or `.txt` for tab, case-insensitively, and reject unknown extensions. Subsequent records use that delimiter, so unquoted commas in single-column TSV are data. Resolve the headers to document variable identities before reading typed rows. The optional name comes from the first real value. Validate the full batch and target names before applying the collection edit. A delimited pixel cell requires a file reference. Users can set Do Not Replace in the dialog. Keep selected file paths portable within the active OS by resolving relative paths at import and leaving foreign path text editable.

Default the export pattern to base name, underscore, and set name. Support tokens for base name, set name, and one-based index plus literal pieces. Reuse the existing export naming conventions for sanitization and UTF-8 truncation. Consider existing entries and planned names when uniquifying targets. Use the existing sibling-temp write pattern with publication that fails if another process creates the final path first. The existing `write_atomically` helper renames over the target, so the batch path needs explicit no-replace publication. Report and skip every set rejected by evaluation, including unreadable replacement assets, invalid targets, unsupported conversions or modes, and allocation failures, then proceed to the next set. Report output I/O failure and stop. Preserve completed files on both failure and cancellation.

## Risks / Trade-offs

- **Unknown Photoshop wire details.** Use observed blocks as the interoperability boundary and retain unsupported blocks untouched. Native reopening and applying a Kooka-authored mixed template is a separate verification result.
- **Large template or replacement.** Keep source tiles shared, enforce decoded-image budgets, and release each scratch output and its serialization buffer before starting the next set. A PSD that exceeds the supported output size reports an error.
- **Mixed-run type.** Replacement uses the type engine's existing modeled style behavior. Preserve the layer's editable type representation and test the modeled styles without claiming fidelity for unsupported runs.
- **Overlapping edits and filesystem races.** Block entry from incompatible edit sessions. Validate the live document identity before commit and publish completed exports without overwriting new files at their computed targets.

## Migration Plan

No document migration is required. Files without variable resources have an empty store. Unsupported resource bytes flow through ordinary resaves. Keep the local reference downloads outside test execution and generate fixture documents from original artwork, values, and layer structures. Tests cover both unedited preservation and author/edit/reopen so a format regression can be detected before shipping.
