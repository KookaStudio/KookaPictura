# Design

## Context

See `proposal.md` - Why. The save path today has two independent hand-maintained
suffix sets: C++ maps a destination to filters, default suffixes, warning rules,
and the worker/raster split; Rust maps it to a writer. GIF is absent from both,
so a `.gif` destination falls through the native branch and is written by the PSD
codec (`impl_core/save.rs`, `raster_writer_for_suffix`, `saveRunsOnWorker`).

The pieces needed for this change already exist:

- `pictura_codec::encode_gif` writes a single-frame GIF89a from an indexed image.
- `pictura_render::convert_to_indexed` flattens and retains `source_palette` and
  a `source_planes` index plane; `quantize` reduces an RGBA image directly.
- `IndexedColorDialog` already presents the palette, colors, dither, and amount
  and drives the existing mode preview, which temporarily swaps the document and
  is cancelled on accept or reject.
- Save As already has a per-format options seam (`SaveFormatKind`,
  `SaveFormatOptions`, session persistence, a test hook).
- Native PSD/PSB, Qt-raster, and PDF writers each end by recording the path and
  clearing the modified flag.

The documented save pipeline is snapshot-based: "The export works from a
snapshot; it never mutates the in-memory document" and Save/Save As/Save a Copy
are not undoable document operations (`docs/10-workflow-io/save-and-save-as.md`).

## Goals / Non-Goals

**Goals:**

- GIF is a declared save destination, encoded from a snapshot, with correct
  palette behavior and no path through the PSD codec.
- Save a Copy is one intent on the shared save path, not four separate
  "remember not to record the path" variants.
- Adding a future save format is one descriptor row plus one writer, and a test
  fails if the two drift.

**Non-Goals:**

- Converting the open document to Indexed Color mode on save. GIF saves a
  snapshot; `Image > Mode > Indexed Color` remains the explicit conversion.
- The GIF Options (row order) dialog, animated GIF, GIF transparency, lossy
  GIF, and GIF metadata (ICC/EXIF/XMP). Row order is carried in the remembered
  settings as `Normal` so the #625 dialog only adds a control.
- Changing the existing Export As / Save for Web paths; they keep using the web
  palette independently.

## Decisions

### D1. Snapshot save, not document-mode conversion

The bytes are encoded from the current rendered composite (or the retained
indexed source), never by converting the live document. This matches the
documented pipeline and keeps Save As total across RGB, Grayscale, CMYK, Lab,
8/16/32-bit, and layered documents. Converting the live document as CS6 does
would require a flatten prompt, a mode history state, and an 8-bit/RGB
pre-conversion for 16-bit and CMYK sources, and would make every future
indexed-only destination repeat the same choreography.

Alternative considered: convert the document before writing (closer to observed
CS6 behavior: flatten prompt, Indexed dialog, row order). Rejected because it
contradicts the snapshot contract and adds several history and prompt paths with
no requirement behind them.

### D2. One descriptor table per language, with a parity test

- C++ owns one table of save-format descriptors: display filter, suffix set,
  default suffix, options-dialog kind, whether the format flattens, and the
  writer family. `saveFileFilters`, `defaultSaveFilterIndex`,
  `defaultSuffixForFilter`, `isKnownImageSuffix`, `isRasterImageSuffix`, and
  `saveRunsOnWorker` derive from it.
- Rust owns one writer dispatch from suffix to `QtRaster`, `EngineGif`,
  `NativePsd`, or `Pdf`.
- A C++ test iterates the descriptor suffixes (skipping the aggregate All
  Formats row) and asks the engine for its writer kind; an unknown writer fails
  the test. This is the guard that would have caught the current GIF-to-PSD
  fall-through and catches the next missing format.
- GIF is appended after PDF in the filter order, so the self-test's hardcoded
  PNG index (2) and its type-ahead expectations stay valid. If the order
  changes, the self-test expectations move with it.

Alternative considered: one cross-language table exposed from Rust. Rejected
because the filter labels are UI text and the table would cross the ABI for no
behavioral gain.

### D3. Save intent through the shared write paths

Introduce `SaveIntent { Save, Copy }` (Save As shares Save's bookkeeping); only
the bookkeeping step depends on it:

```
write snapshot bytes atomically
        |
        +-- Copy: stop here
        |
        +-- Save/SaveAs: record path, clear modified when the snapshot is current
```

The native PSD worker carries a copy flag on its job and skips the path/dirty
application in its finish step. PDF and raster writers take the same flag. The
C++ coordinator only updates the document registry, tab title, window title,
recent list, and recovery snapshot for non-copy intents, and records no action
step for a copy. A copy opens the same Embed Color Profile dialog as Save As for
PSD/PSB/JPEG/TIFF. Tests drive the save-path dialog, the profile dialog, and the
PDF dialog through explicit drivers; the existing `setSaveFormatDialogForTest`
hook only covers the per-format options dialog.

Alternative considered: run the normal save, then restore the path/dirty state.
Rejected: the native save is asynchronous, recovery/recent side effects would
need undoing too, and a failure mid-restore leaves inconsistent state.

### D4. GIF encoding source

```
document carries an Indexed source that still expands to the current pixels?
        |
        +-- yes: retained palette + index plane -> encode_gif   (no re-quantize)
        |
        +-- no:  refresh composite if dirty
                 -> 8-bit sRGB RGBA
                 -> quantize(palette, colors, dither, amount)
                 -> encode_gif
                 -> write_atomically
```

The "still expands" test is the codec's existing `writes_indexed` predicate:
`convert_to_indexed` retains `source_palette` and `source_planes`, but an edit
leaves them stale, and the PSD writer already falls back to the working RGB in
that case. GIF needs the same currency check, so the codec exposes a small
public helper (or the app calls a widened `writes_indexed`) and falls back to
the composite when it is false.

The palette table is built from the 768-byte PSD palette; entries beyond the
highest used index are trimmed before encoding. A document wider or taller than
65,535 pixels is refused before encoding because GIF89a dimensions are 16-bit.
The engine encoder stays the only GIF writer; `gif` is deliberately not added to
the Qt writer map. The GIF-specific settings (palette, colors, dither, amount,
and the future row order) live in `SaveFormatOptions` and are stored on the view
like the other raster options, so a plain Save to a recorded `.gif` path uses the
last accepted settings.

### D5. Reuse the Indexed Color dialog as the GIF format dialog

`SaveFormatKind::Gif` opens the existing `IndexedColorDialog` with a seeding
constructor for the remembered palette, colors, dither, amount, and preview.
The dialog's Preview drives the existing mode preview, which temporarily swaps
the live document; both accept and reject cancel the preview before returning,
so the snapshot written is always the real document and no history state leaks.
When the remembered palette is Exact but the document has more than 256 colors,
seeding falls back to an offered palette. The Preview control is disabled for
16-bit, CMYK, and Lab because `convert_to_indexed` refuses those modes. Cancel
propagates as a failed save, which already writes nothing.

### D6. Command catalogue and classification

Add `file.saveCopy` (`File > Save a Copy…`, no shortcut) to the catalogue, the
frozen const names, the generated `command_ids.h`, the catalogue goldens, the
counts, and the modal-id list. Classify it non-recordable for this change: the
action recorder models Adobe `save` steps as Save or Save As only, and recording
a copy without an Adobe-compatible `copy` parameter would replay the wrong
operation. The File menu enables it with a document and disables it otherwise.

### D7. Fail closed for declared formats

The known image suffix set gains `gif`, and every member of that set has a
declared writer. A suffix with no declaration keeps today's behaviour only for
the native fallback (no extension -> PSD); a known image suffix can never reach
the PSD codec. An extension outside the known set is left to the native
fallback, as today, so agent-control saves with unusual names do not change.

## Risks / Trade-offs

- **Preview state leaks into the write.** The GIF dialog previews by temporarily
  replacing the live document. Mitigation: cancel the preview on both dialog
  outcomes and assert in a test that the document is restored with unchanged
  history.
- **The retained Indexed source is stale after an edit.** Mitigation: the
  `writes_indexed` currency check falls back to quantizing the current composite;
  an edited-indexed test covers it.
- **GIF dimensions are 16-bit.** Mitigation: refuse width or height over 65,535
  before encoding and test the refusal.
- **A copy records an action step.** Mitigation: the copy path records nothing
  and a recorder test asserts the action is unchanged.
- **The C++ and Rust tables drift again.** Mitigation: the filter-to-writer
  parity test; it is the release gate for every future format.
- **GIF quantization is synchronous and can be slow on large documents.** The
  existing raster save is synchronous too; the Indexed Color preview already
  quantizes on change. Accepted for this change; a worker can follow if a large
  document benchmark demands it.
- **Indexed GIFs are opaque.** The current Indexed conversion has no
  transparency option, so a GIF written from a converted document has no
  transparent index. Explicit non-goal; Save for Web/Export As remain the
  transparency path.
- **A copy to the document's own path is not treated specially.** The file is
  written atomically and the document keeps its path/dirty state; no data is
  lost, but the title can still show modified after the write. Accepted;
  Photoshop's Save a Copy has the same ambiguity.
- **Command catalogue counts and goldens must move together.** Mitigation: the
  generator plus the catalogue tests fail on a stale header or golden.
- **PDF copy side effects.** The PDF writer currently records the path and dirty
  state and remembers the job options. The copy intent skips the path/dirty
  bookkeeping; remembering the option set as a session preference is harmless
  and keeps the next PDF dialog seeded.

## Migration Plan

Additive only: no file-format or session-schema migration. Older session stores
without GIF settings load the CS6 defaults. Rollback is deleting the new command
and descriptor rows; existing GIF files remain readable through the unchanged
import path.
