## Context

An Indexed document stores 8-bit index planes and a planar 768-byte palette alongside expanded RGB working pixels. The web codec already implements palette quantization and ACT encoding. Adobe identifies PSD resources 1046 and 1047 as the active color count and transparent index.

## Goals / Non-Goals

- Add document-level palette editing and conversion controls, with undo and reversible previews.
- Keep unrelated GIF save-dialog options and existing parameter defaults intact.
- Use the current quantization and white-matte model. System palettes and Photoshop-exact quantizers are outside this change.

## Decisions

- Edit a palette in a modal Qt dialog. An accepted edit updates the working planes from retained indices and records one history entry. Reject a shorter loaded table if it would omit a used entry.
- Preserve source opacity on conversion by compositing the visible stack without an opaque backdrop when transparency is requested. The existing indexed quantizer interprets alpha below 128 as transparent and composites the remaining pixels over white.
- Use the web palette reducer for forced colors, preserving every distinct forced color. A required clear entry counts against the 256-entry format capacity; reject a 256-distinct-color forced set when source pixels require that entry, without changing the document or history. Do not inherit the reducer’s existing silent loss of transparency.
- Compute Exact availability from the union of distinct opaque quantization inputs after the selected transparency and white-matte treatment and distinct forced colors, plus one entry if a clear pixel requires it. Exact is available only when that total fits 256. Recompute when options change; if Exact becomes unavailable, switch the dialog to Local (Adaptive). Reject an invalid direct Exact request atomically, before changing the document or history. Preview and commit use the same eligibility and conversion rules.
- Write transparency and active count as big-endian PSD image resources. Derive working alpha on read, and compare the retained indices against the current rendered pixels, disregarding invisible RGB at the transparent index. PDF may use its existing RGBA path.
- Preserve existing bridge callers and use extended arguments for the document conversion dialog and Actions.

## Risks / Trade-offs

- Exact availability depends on both the source and the selected options; changing options may switch the selected palette to Local (Adaptive).
- Indexed PSDs with legacy or irregular layer data are editable only when indices can be reconstructed safely. RGB fallback protects subsequent edits that diverge from the retained indexed data.
