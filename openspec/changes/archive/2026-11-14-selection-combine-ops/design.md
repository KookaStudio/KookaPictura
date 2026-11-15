# Design

## Context

See `proposal.md` for motivation. The current state that shapes the approach:

- `Selection::combine` in `crates/pictura-select/src/lib.rs` already implements Replace, Add, Subtract, and Intersect over 8-bit coverage, and every selection tool uses it.
- `impl_selection.rs` saves a new channel and ignores the dialog name; loads by positional `Alpha N` and always uses New.
- `Document.image_resources` is the verbatim PSD image-resource section. The writer preserves it through `icc -> paths -> guides`; unknown blocks and the unparsed tail are byte-identical.
- `Document.channels` is a `Vec<Channel>` where `Channel` is just `id` plus an 8-bit plane; per-layer channels use the same type.
- The C++ frame owns every document view (`documentCount`, `viewAt`, `document_width`, `document_height`, `documentName`), so cross-document dialogs can be built without new document plumbing.
- `Edit::Meta` captures history without compositing; `Edit::Regions` refreshes the canvas. A layer-mask write changes the composite; a channel write does not.

## Goals / Non-Goals

**Goals:**

- One combine algebra for tools, save, and load.
- Names that survive PSD round-trip without changing the channel plane format.
- A dialog and bridge surface that stays usable for same-document and cross-document operations.
- One undo state per applied action, with refusals recording nothing.

**Non-Goals:**

- Selected Areas / Masked Areas polarity (#611).
- 16/32-bit channel storage (#1331).
- Cross-image channel duplication and selection dragging (#1332).
- Colour channels as Save/Load targets (#1333).
- Engine-wide channel cap enforcement and anti-aliased rasterisation (#614).

## Decisions

### 1. Reuse `Selection::combine`

The four operations already have one tested implementation. CS6's soft-coverage blend (`d + s(1-d)`, `d(1-s)`, `d*s`) is community-inferred and differs only for partial coverage; binary masks agree. A second algebra would make Add mean two things and double the edge-case surface. If a CS6 oracle arrives later, changing the shared function fixes every caller at once.

Alternative rejected: a Save/Load-only Photoshop blend implementation.

### 2. Names live in PSD image resources 1006/1045

Resource 1045 (Unicode, `u32` character count + UTF-16BE) is authoritative; 1006 (MacRoman Pascal strings, even-padded) is the legacy fallback. Names map to extra channels by order. The writer rewrites both blocks and keeps every other block and the unparsed tail byte-for-byte; unchanged names return the original section.

Alternatives rejected:

- `Channel.name`: 493 literal constructors and a name field on per-layer channels where it has no meaning.
- `Document.channel_names`: a parallel vector that can desync with `doc.channels`.

The existing `set_vanishing_point_data` and `with_document_guides` patterns are the template. `Alpha N` remains the display fallback, and the Ctrl+number labels stay positional.

### 3. Layer-mask target writes existing masks only

The Channel list offers every existing raster layer mask in the chosen document. A layer without a mask is not a target; creating one stays `Layer > Layer Mask > Reveal Selection`. Save normalizes the mask rectangle to the canvas, writes selected coverage as white (revealed), preserves density, feather, flags, and enabled state, and drops the retained native `-2` plane through `replace_layer_mask`.

Alternatives rejected:

- Creating a mask on Save: not CS6 behavior and duplicates the Reveal Selection path.
- Keeping a smaller stored mask rectangle: a document-sized selection cannot be represented exactly without a complex remap, and the compositor already handles a canvas-sized mask.

The write is an `Edit::Regions` using `pictura_render::damage::influence`, because the composite changes. A disabled mask still records one state even though the composite does not move.

### 4. Cross-document operations transfer coverage bytes

The C++ dialog lists same-size documents and orchestrates the operation. The source view exposes its selection or channel coverage as bytes; the destination view applies the bytes. This avoids a two-view cxx-qt function and the aliasing questions of two mutable `PictureView` references.

Trade-off: one document-sized buffer copy for an explicit dialog action. The mask is already copied by history when the destination records the state.

### 5. One dialog class, two modes

A single `SaveLoadSelectionDialog` switches between Save and Load layouts. Targets are encoded as `"new"`, `"alpha:<index>"`, or `"mask:<layer path>"`; load sources use the same encoding. Index-based targets keep duplicate channel names unambiguous.

### 6. Modifier table

Channels panel rows: Ctrl/Cmd New, +Shift Add, +Alt Subtract, +Shift+Alt Intersect.

Layers panel raster-mask thumbnail: Ctrl New (unchanged), Ctrl+Shift Add, Ctrl+Alt Subtract, Ctrl+Shift+Alt Intersect. The docs' Shift-alone/Alt-alone mask bindings are not adopted because the panel already uses Shift for enable and Alt for solo.

### 7. History and damage

Channel save/load is `Edit::Meta`. Mask save is `Edit::Regions` with the mask's influence. Both record one state; refusals return `Edit::None`.

### 8. New image

The new document is a grayscale document with the selection as its extra channel. Multichannel was the original choice, but the codec does not write Multichannel (it has no colour plates to retain), so the design's grayscale+alpha fallback is used.

## Risks / Trade-offs

- [Large cross-document copy] -> the transfer is explicit and one-shot; a future optimization can stream tiles.
- [1006 cannot represent every Unicode name] -> 1045 is authoritative; 1006 is written ASCII-lossy for legacy readers.
- [Mask normalization changes the stored rectangle] -> the compositor already handles canvas-sized masks; metadata is preserved.
- [56-channel guard is Save-only] -> documented; the engine-wide cap remains #614.
- [Duplicate channel names] -> dialogs address channels by index, never by name.
- [New grayscale+alpha document may not round-trip as expected] -> verified by saving and reopening it as a PSD in the Qt test.

## Migration Plan

No data migration. Existing PSDs keep their image-resource bytes unless channel names change; undo snapshots already carry the resource section. Reverting the change leaves written files readable because names are ordinary PSD resources.
