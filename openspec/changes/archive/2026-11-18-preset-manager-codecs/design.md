# Design

## Context

Swatch defaults and in-memory edits live in `SwatchGrid`. `panel_host_build.cpp` restores the legacy session's `[{name, color}]` array and distinguishes a missing array from an explicitly empty one. Brush and contour pickers each own static tables. The gradient editor requests built-ins through the Rust bridge. Custom Shape resolves an index into built-in paths. PSD pattern decoding and writing already support some document pattern records, while Pattern Stamp has its own built-in tiles.

## Goals / Non-Goals

**Goals**

- Keep one authoritative list per managed type, with typed payloads shared by the Manager and consumers.
- Give native persistence a stable version and expose format limits at the import and export boundary.
- Have `.csf` consumers receive typed settings and diagnostics without requiring application state or ICC profile discovery.

**Non-Goals**

- Adobe preference migration, workgroup bundle sharing, Curves `.acv`, full standalone preset-authoring panels, and the Color Settings controls tracked in #569.
- Universal Adobe file-version support. The supported matrix in the codec documentation defines the tested interchange subset.

## Decisions

### Put byte codecs in `pictura-codec` and current lists in the app

Keep library envelopes, bounded readers and writers, and typed decode results in `pictura-codec`. Keep current selection, persistent list identity, discovery, and commit policy in one application-scoped store. The Qt model reads that store and sends mutations through it. Each legitimate consumer uses its own typed adapter. A separate crate would introduce a second home for descriptor and pattern primitives already used by PSD and ATN. A Manager-local model would require synchronizing copies with the existing pickers.

Reuse `action_descriptor.rs` where a format's descriptor body matches it. Keep variant headers, length boundaries and version dispatch in the respective adapters. The strict descriptor reader stops at unknown value types, so preserve opaque bytes only where a validated boundary makes preservation safe. Reuse pattern record parsing where the layout matches and retain the decoded name. Keep a clipboard or document payload separate from the external preset envelope. Existing PSD pattern and ATN round-trip tests remain regression checks.

### Make native Save Set the lossless path

Use a versioned per-type envelope containing ordered entries with Unicode names, typed values and retained bounded payloads. Reuse the envelope for current lists, using record presence to distinguish an empty list from defaults. Preserve source color values for swatch interchange and convert only when displaying or applying them. Offer Adobe swatch exports for representable entries and keep Adobe export controls tied to tested writers. Using Adobe files as session storage would make restart fidelity depend on their conversion limits.

Use bounded parsing and aggregate limits for bytes, entries, decompressed payloads and recursion. Parse into a candidate off the GUI thread. Post complete or partial results back with the operation's type and request identity. Serialize commits per type and discard results from cancelled or superseded operations. On a partial result, require confirmation before append or replace. A valid explicitly empty native set replaces with an empty list or appends nothing; failed or partial imports with no usable entries leave the list intact. Save from a snapshot with the app's atomic-write mechanism so another edit can't change the file halfway through.

### Resolve a preset when the tool or dialog uses it

Use stable entry IDs for UI selection and references. Convert the entry to the existing tool or document representation once it is used. Capture sampled brush data and stamp tiles when the stroke begins, and resolve gradient foreground/background stops against the current colors at use time. Reuse the document path model for imported shapes. Apply styles and contours with the existing Layer Style edit transaction. Embed used pattern assets via the PSD document path with a document-local identity that avoids collisions. Gradient, contour and vector values are copied into the resulting edit through their normal document representations. Storing library IDs in a document or active gesture would make unrelated library changes affect existing content.

### Keep the `.csf` codec independent of Color Settings

Expose a versioned import result containing typed values, profile references and diagnostics. Preserve unresolved profile references as strings or their evidenced file representation. Code that uses profiles, decides whether to apply imported values, and presents errors belongs to #569. The [Adobe CS6 Printing Guide](https://helpx.adobe.com/content/dam/help/en/indesign/pdf/CS6_PrintGuide.pdf) describes the workflow. The repository's [Color Settings contract](../../../docs/10-workflow-io/color-settings.md) records the remaining byte-layout questions. Base the supported variant and each mapped value on lawful observation of small, independently authored files. Demonstrate the writer by reopening its output in CS6. An implementation-written native settings file alone cannot establish `.csf` interoperability.

## Risks / Trade-offs

- **Format evidence.** Start with `.aco` v1 and v2, `.ase` v1, ABR 10.1 and `.grd` v5 as the candidate versions from the existing contracts. For `.asl`, `.pat`, `.csh`, `.shc`, `.tpl`, and `.csf`, record exact variant framing and field mapping from public evidence or lawful black-box observations before claiming support. The binary layout of `.tpl` and `.csf` is an implementation dependency. If the required subset cannot be evidenced, report that gap rather than claiming compatibility from a self-round-trip.
- **Unsupported options.** A sampled brush, tool preset, or style can carry values Pictura cannot currently apply. Report them alongside the supported values. Generate project-owned automated fixtures and keep CS6-authored oracle files out of version control, following `CONTRIBUTING.md`.
- **Document portability.** Pattern or style application may depend on an external pattern. Capture any supported dependency when the style is applied and verify the document reopens with the same result. Report a missing dependency before applying the affected portion.

## Migration Plan

Read the existing swatch array only when a native Swatches current list is absent. Preserve the legacy names, colors and order, including a present empty array. Commit the migrated list atomically before it becomes authoritative. Keep the legacy data readable for rollback. Preference reset stages explicit defaults for all types, including Swatches, as a candidate generation and atomically switches a small active manifest before treating reset as successful; it never removes the authoritative Swatches record and thereby re-enables legacy migration. A failed migration or reset write reports failure and preserves the prior authoritative list and legacy data. A corrupt migrated file produces a diagnostic and preserves the last usable list or the defaults. Use the documented XDG per-type library directories for exports and discovery, separate from current-list preferences so reset retains exported sets.

Current lists live in separate native per-type files, not in the session object. Settings export includes the effective list for all eight types in a dedicated `currentPresetLists` section, resolving defaults, native data and legacy swatches without mutating live settings; the timestamped pre-import backup does too. Import validates and stages this section without changing live lists, then applies it before the next session load. A missing section in an older backup leaves all current lists untouched; within a present section, absent types remain untouched and an explicitly empty native set clears that type. Validate all present files and bound their sizes before staging. An invalid section or failed staging leaves live stores and the previous pending import intact. Bound the entire serialized backup against the import limit before committing an export or pre-import backup; report unreadable lists or an oversized backup instead of silently omitting them. Stage validated changed per-type files in a candidate generation, carrying forward unchanged types, then atomically switch one small active manifest as the publication point. A staging or manifest failure leaves the previous generation active and the pending import intact. Report the failure and continue startup with the previous complete generation; retry idempotently before loading lists on a later startup, publishing the complete generation together. Never delete the active files before publication.
