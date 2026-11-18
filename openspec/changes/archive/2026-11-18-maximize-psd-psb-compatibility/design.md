# Design

## Context

The save coordinator routes interactive native Save and Save As through `saveActiveAs` and native Save a Copy through `saveCopyAs`. Both dispatch to the Rust save worker. Internal Smart Object saves and recovery use their own entry points. The worker clones the document, optionally refreshes the native composite, stages it with `pictura_render::save_view`, and writes atomically.

The codec already reads and writes layered files that end after the layer-and-mask section. That reader rejects the same layout without layers and cannot restore document-level channel pixels from an absent image-data section. The writer checks composite geometry and buffer length even when it omits the section. A layered RGB document may keep no composite buffer in memory.

## Goals / Non-Goals

Keep the preference at the application boundary and the resolved choice with the save job. Preserve layers, document-level channel data, supported color modes and bit depths, and the existing save transaction. Scope Adobe verification to representative documents using the supported feature set and a recorded Photoshop CC build.

## Decisions

1. **Use the existing session settings.** Store one stable value for Always, Ask, or Never. Missing and invalid values load as Always. Adobe's CS6 Help describes the three modes without giving a factory default. Always is a deliberate Kooka default that preserves ordinary saves and gives applications such as Lightroom the merged pixels they need.
2. **Resolve the choice before dispatch.** Share the decision between interactive native Save, Save As, and Save a Copy. Check whether the document can safely omit the merged image. In Ask, make Include the default, explain that other apps may require it, and treat closing the prompt as Cancel. Pass the resolved choice as an explicit worker input. Identify the save origin explicitly through shared entry points: interactive commands apply the preference, while control-server pathless and explicit-path saves and Action replay retain their existing noninteractive policy. Stage the ICC save choice until after compatibility acceptance so cancellation of Save As or Save a Copy cannot mutate it. The live document and its history still carry their original composite metadata after either outcome. A per-view preference override would require clearing transient state after cancellations and before internal saves.
3. **Separate the real image from its serialized representation.** Before changing the writer, inspect equivalent CS6 and current CC PSD/PSB pairs made with Always and Never. Record the file offsets, image-resource 1057 Version Info including `hasRealMergedData`, and any placeholder or extra channel data. Try opening the existing Kooka omission output in both Adobe versions. Reuse the existing omission path where that exchange works. If Adobe requires a placeholder or metadata, use an explicit app-only write option for the fixture-supported representation; preserve the direct writer default of no image-data section and its buffer preconditions. Add an automated regression using a provenance-compliant Adobe Never fixture covering its observed representation, edit, and Include output pixels and metadata. Imported placeholder pixels must not replace the layer-derived picture. Preserve any separately carried channel data and keep merged-image metadata consistent with the emitted image data.
4. **Stage the snapshot for its intended output.** For omission, bypass unnecessary merged recomposition and native refresh where layers supply the data. For detached buffers, satisfy the existing direct writer buffer preconditions with allocation-free staging; this change does not relax those preconditions. Derive header mode and depth from the established source/layer state. With Include, reconstruct from layers if the input had no real merged image, even if it has a resident display buffer or placeholder. For supported RGB or Grayscale 16/32-bit layer-only inputs with retained native layer samples, create the missing native composite store before native refresh, preserving native layer samples rather than widening the display buffer. Follow established RGB, Grayscale and native-depth paths and verify the current composite against the edited layer stack. Keep the atomic write and Save a Copy finish behavior.
5. **Give data preservation priority.** Keep required image data for layerless files and document-level alpha, spot, or saved-selection channels. Keep existing raster, PDF, recovery, internal Smart Object and headless output policies. Use the current application representation check for eligibility rather than a heuristic based on layer count greater than one.

## Adobe fixture observations

The local black-box reference `never01.psd` reports Adobe Photoshop 2021 in Version Info. It is an 8-bit RGB PSD with two layer records and `hasRealMergedData = 0`. The file still carries a 33,686-byte PackBits image-data section. Every sample in all three planes decodes to 255, giving a white placeholder.

The implementation uses this layout for interactive Never saves and uses PSB's 32-bit PackBits row counts in PSB output. The 32-bit float placeholder uses zero samples so wide PSD scanlines fit the 16-bit row count. The automated fixture is generated from the observed layout with project-owned pixels. Paired fixtures from CS6 and current desktop CC and Adobe's PSB output were unavailable for this implementation.

## Risks / Trade-offs

- A file accepted by Kooka's reader may fail or render differently in Photoshop. Verify both directions in both Adobe releases and record the exact build and observed file layout. A Kooka-only round-trip cannot establish Adobe interoperability.
- Omission can hide a depth or channel regression while leaving a file readable. Compare mode, depth, channel samples, editable layers, and rendered pixels after reopen. Cover a detached working RGB composite and a native high-depth document.
- The file-size reduction depends on the encoded layers and composite. Assert a reduction on a controlled layered fixture.

## Migration Plan

Read absent or invalid preference values as Always and persist only supported values. Previously written documents continue to open. Keep the codec's existing layer-only read path for older files.
