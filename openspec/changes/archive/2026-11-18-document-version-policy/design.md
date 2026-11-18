# Design

## Context

The eager and lazy PSD readers share native-header and image-resource reading in `pictura-codec/src/read.rs`. The lazy application open may skip merged pixels or keep a temporary preview while recompositing the layers. Native application saves take a snapshot and call the cached PSD/PSB encoder. The shared writer assembles resources separately from merged-image output. File Info already has a read-only Photoshop metadata tab. See [proposal.md](proposal.md) for scope.

## Goals / Non-Goals

**Goals.** Share format decisions across native opens and saves, keep compatibility state with the document session, and retain existing encoder output for callers using the default codec write API.

**Non-Goals.** The decoder continues to apply its existing dimension, resource, and memory limits. The native version policy applies to local PSD/PSB. The first release uses an empty migration chain for the current private schema.

## Decisions

### Keep native and private versions independent

Retain header versions 1 and 2. Parse resource 1057 with the shared bounds-checked reader. Support resource version 1 using Adobe's documented field layout and keep the file version as an opaque numeric value for display. For a new 1057, use layout version 1, file version 1, and the Kooka Pictura product name. Validate these writer defaults in the manual Photoshop matrix. [Adobe PSD/PSB format specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)

Use proposed private resource ID 4973 in the plug-in range, the Pascal name `KookaPicturaDocumentVersion`, and the four-byte marker `KPVR`. The exact resource name identifies ownership so a foreign block with the same numeric ID survives. A matching name with invalid contents is reported as damaged. The payload begins with the marker, a big-endian 16-bit layout version, then two big-endian 16-bit numbers for the document version and minimum reader version, followed by a length-prefixed UTF-8 writer version. Keep that prefix legible for future layouts so an older reader can evaluate the floor. Use one stable XMP namespace URI, `https://kookastudio.github.io/KookaPictura/ns/document-version/1.0/`, with the three property names in the spec. Match by namespace URI regardless of prefix.

A missing private record is the legacy representation. Version 1 has reader floor 1. Treat zero, a floor above the stated document version, duplicate owned resources, and unreadable version headers as damaged metadata. For an accepted newer pair, keep the owned record byte-for-byte, including extensions this reader cannot interpret. Report a disagreement in the version pair when both carriers supply values. Adobe's writer/version metadata remains descriptive. XMP-only authority would let a metadata edit change the read policy. [Adobe XMP Basic namespace](https://developer.adobe.com/xmp/docs/xmp-namespaces/xmp/)

### Classify file features using the codec's existing decode evidence

Return structured compatibility information through eager and lazy native reads. Build it from known tags and parsed descriptor variants at the places the codec interprets them. Start with artboard structure, linked Smart Object variants and dependencies, and adjustment records whose appearance cannot be reconstructed. The codec classifies known inert resources separately from layer and global records affecting rendering or rewrite. Unclassified layer/global rendering or rewrite-critical records and unknown dependency-sensitive edit types are unsafe until explicitly proven safe; restrict the document to read-only rather than assume preservation implies safe edits. Keep known inert resources editable and byte-preserved. The codec returns structural evidence; the application combines it with renderer support from pictura-render without adding a codec-to-render dependency. Test unknown layer and global records, unsupported dependent edits, known inert resources, and supported controls.

### Keep original merged pixels available in compatibility mode

Identify a real merged image from the output flag where supported and a successfully decoded image-data section. For absent or unreadable 1057, use a source composite only when other supported format evidence establishes that the section carries real merged pixels. A supported false flag rules it out. Use the existing source-backed or budgeted merged decoder and normal display color management. Decode is fallible and source-backed data must be checked for changes since open; on failure report the reason and do not create or replace a copy destination. Preserve this view for the lifetime of a read-only session. The ordinary lazy preview expires on editor events and has additional restrictions, so compatibility viewing needs its own lifetime and eligibility. A missing or unusable source composite leaves a diagnostic view and no flattened-copy source. [Adobe's compatibility composite guidance](https://helpx.adobe.com/photoshop/desktop/troubleshoot/file-format-issues/recover-damaged-or-corrupted-photoshop-files.html)

### Enforce the policy at operation boundaries

Install access state with the decoded document and replace it on new, open, and revert. Use the existing asynchronous open completion to carry one warning per open and pass a specific invalid-container version error up to Qt. Put edit and save checks at shared application mutation and write boundaries, including direct tool paths and action playback. Disabling menu commands gives feedback at the point of use. Keep File Info available for inspection.

Use the existing Save a Copy coordinator, confirmation, PNG encoder, and atomic write. For compatibility copies, use only validated source merged pixels. Verify distinct source and destination even when the path is spelled differently or aliases the source. The path-based API must require explicit copy intent and confirmation, just as the dialog does. Use a common destination check to guard every output capable of targeting any open compatibility source, including control-server `export_png` and output from another active tab, against source paths and filesystem aliases; unrelated destinations retain their normal export behavior.

### Stamp native application output through an explicit codec write mode

Route interactive and direct native application writes through the shared writer with a version-aware option. Apply it after the save snapshot determines the final merged output and profile policy. Preserve defaults for raw codec serialization and internal payload consumers. Only a completely framed resource section establishes absence or uniqueness of owned resources and 1057. Any incomplete section is indeterminate even when a valid owned record occurs in its prefix: open read-only and refuse version-aware writes. The default raw writer retains its existing tail behavior. Preserve resource order for fully framed sections. Open files with malformed, unsupported, or duplicate 1057 inspectable read-only with a reason, and refuse version-aware writes because the merged flag cannot be made reliable without altering opaque bytes. This save-safety verdict does not change the private reader floor. Reuse the namespace-aware XMP editor and preserve unrelated bytes. If unrelated XMP cannot be amended, preserve it and return a save warning. Carry the warning through asynchronous completion without marking a successful write as failed.

## Risks / Trade-offs

- **Opaque data can depend on edited layer structure.** Restrict known unsafe variants to read-only. Exercise an unrelated edit and an affected feature in the test plan, and record observed support per variant.
- **A file may contain a compatibility placeholder.** Require evidence of a real merge as well as successful bounded decoding before using the pixels for viewing or export.
- **Adobe may use the selected plug-in resource ID.** Check the repository and the representative Adobe specimens for that ID. Preserve matching IDs with foreign names and verify a collision fixture.
- **Local Photoshop coverage depends on installed builds.** Record the exact releases and specimens exercised. Keep Photoshop-authored files local under the repository's provenance rules. Missing required matrix cases block shipment unless the requirement is deliberately revised and documented.

## Migration Plan

Legacy and foreign files require no conversion before open. Their version metadata is written on the next successful version-aware native save. Rollback verification uses unstamped and version-1 files and confirms that older readers can open supported PSD/PSB content with the extra image resource present.
