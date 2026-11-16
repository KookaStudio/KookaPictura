# Design

## Context

See `proposal.md` for motivation. The implementation sits on four existing
seams:

- **Command plumbing.** `crates/pictura-script/src/command_catalogue/entries.rs`
  already carries the disabled `file.scripts.load.files.into.stack` row;
  `commands.cpp` builds actions from it and enables an action only when
  `implemented` is set, a handler is registered, and any enabled provider
  passes. The frozen identifiers come from `const_names.rs`.
- **New-document installation.** `impl_core::new_document` builds a `Document`,
  refreshes its composite with `DocumentComposite`, and installs it with
  `replace_document`; the shell creates a fresh `PictureView` and hands it to
  `DocumentRegistry::addDocument`.
- **Decode and colour.** `helpers::decode_import` probes/decodes a raster
  through the engine's WebP path or Qt, and `open_jobs::convert_placed_rgba`
  applies the document colour policy. `pictura_codec::read_psd` +
  `pictura_render::composite_rgba` flatten a PSD/PSB source.
- **Auto-Align and Smart Objects.** `pictura_render::plan_auto_align` /
  `apply_auto_align` own the alignment; `smart_object.rs` owns the embedded
  source recipe (`write_psd` payload + `SmartObject`).

The dialog is the CS6 **Load Layers** dialog. Its source list is the only
state; the options are booleans.

## Goals / Non-Goals

**Goals:**

- One command, one dialog, one new document per run; no mutation of an open
  document.
- Keep format/decoding logic in Rust and the Qt file/selection UI in C++.
- Reuse Auto-Align and the embedded smart-object writer rather than duplicate
  either.
- Bound memory with the existing governor before building the stack.

**Non-Goals:**

- A `.jsx` interpreter or the other Scripts entries.
- Loading a folder recursively or watching it for changes.
- Adding PDF/EPS page selection to the stack; the source filters are the
  Open/Place raster and PSD/PSB set.
- A separate "Load Files into Stack" history state per source; the new
  document starts with one state.

## Decisions

### D1. The bridge builds the document, the shell creates the view

`stack_files.rs` exposes one free function over `PictureView`:

```text
load_files_into_stack(view, paths, align, smart_object) -> failed paths
```

It decodes the paths, builds a `Document`, optionally aligns and combines it,
refreshes the composite, and installs it in `view`. The C++ handler creates a
fresh `PictureView`, calls the bridge, and only then adds it to the
`DocumentRegistry`. This mirrors `File ▸ New` and keeps the command working
with no document open. The bridge returns the failed paths so the shell can
report them after the document exists.

### D2. Decode each source to RGBA, then use the raster-layer primitive

A raster source uses `decode_import` + `convert_placed_rgba`; a PSD/PSB source
uses `read_psd` + `composite_rgba` and its merged pixels. Both become one
`add_raster_layer_from_rgba` layer. Layers are appended in reverse list order so
the first listed file is topmost, matching the dialog and the Layers panel.

The canvas is `max(width)` × `max(height)`; each layer keeps its native rect at
`(0, 0)`. A source larger than the canvas cannot occur because the canvas is
the maximum. The memory gate charges the sum of the decoded RGBA buffers before
the document is built.

### D3. Auto-Align reuses the engine plan synchronously

When the option is on and there are at least two layers, the bridge calls
`can_auto_align`, then `plan_auto_align` with `AutoAlignProjection::Auto` and no
lens profile, then `apply_auto_align` on the in-memory document. It runs before
the composite refresh and before the smart-object combination, so the embedded
source contains the aligned stack. A failed or empty plan leaves the layers
where they are; it does not fail the load.

### D4. Smart Object embeds the whole stack

Add `pictura_render::stack_into_smart_object(doc) -> bool`:

1. Build an embedded `Document` with the same canvas, mode, and depth.
2. Clone the document's layer stack into it.
3. Serialize it with `pictura_codec::write_psd`.
4. Replace the document's layers with one channel-less embedded
   `SmartObject` layer at the canvas rect, named after the topmost layer.

This is the multi-layer counterpart of `convert_to_smart_object`, so the
embedded source keeps the individual layers and a later `Edit Contents` or
`Rasterize` sees the stack. A refusal (non-8-bit, empty stack, writer error)
leaves the layered document untouched.

### D5. Folder mode is a filtered directory listing

`Folder` mode chooses one directory and lists its supported image files by
extension, sorted by name. It does not recurse and does not remember a folder
between runs. `Remove` and `Add Open Files` are disabled in this mode because
the list is the folder's contents, not a user-editable selection.

### D6. One capability, no modified capability

The dialog, the source rules, the stack, and the two options form one
externally visible contract, so they live in one capability
(`interop/load-files-into-stack`). Auto-Align and smart-object behavior reuse
their existing capabilities without changing them.

## Risks / Trade-offs

- **Auto-Align cost.** Alignment runs synchronously before the new tab appears;
  a large stack may pause the UI. The existing Auto-Align command has a worker
  and progress window; the script path is bounded by the sources the user chose
  and is the CS6-like behavior. A follow-up can move the load to a worker if it
  becomes noticeable.
- **Memory.** The decode buffers plus the layer planes plus the composite can
  be large. The memory gate refuses the run before allocation; per-file decode
  limits still apply.
- **PSD flattening.** A PSD source becomes its merged composite, not its layer
  tree; that is what "load the file as a layer" means for a stack and avoids
  importing groups and adjustments into the new document.
- **Smart-object size.** The embedded PSD duplicates the stack's pixels. The
  option is off by default, and the user opted into a smart object.

## Migration

None: the command is new and no document format or saved state changes.
