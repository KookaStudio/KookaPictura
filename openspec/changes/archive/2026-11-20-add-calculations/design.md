# Design: add-calculations

## Context

See `proposal.md` for motivation. Apply Image (#609) established the pattern
this change follows:

- `pictura_render::apply_image_planes` reads a document-sized plane set for a
  merged image, a layer, a colour plane, Transparency, a layer mask, an alpha
  channel, or a selection. `apply_image_read` exposes it to C++ as an opaque
  snapshot.
- `ApplyImageMode` already carries the layer modes plus Add/Subtract with
  Scale and Offset; `blend::blend` is the shared blend function.
- The app already has writers for a coverage plane: `apply_selection_labeled`
  (selection), `save_selection_shape` (alpha channel, with naming and the
  56-channel cap), and `new_selection_document` (a grayscale document whose
  alpha channel holds coverage).
- No canvas preview exists for an arbitrary plane: `ActiveOp`/`PreviewBaseline`
  write the active layer, `mode_preview` is only read by mode conversion, and
  `mask_solo` is tied to a layer's raster mask.

## Goals / Non-Goals

**Goals:**

- One kernel that turns two single-channel sources plus a mask into one
  8-bit coverage/grayscale plane at document size.
- Reuse the Apply Image readers, mode enumeration, blend function, and the
  existing selection/channel writers; add only the missing raster-document
  writer.
- A modal dialog that mirrors `ApplyImageDialog`'s picker behavior; Selection
  and New Channel each commit one `Calculations` state, and New Document starts
  a new document with its `New` state.

**Non-Goals:**

- A canvas preview (see the preview decision).
- Spot channels, Channel Options, or changes to `imaging/apply-image`.
- Native-depth result documents or channels; results narrow to 8 bits.
- Action/script replay of the dialog's parameters: the command is a modal
  AppNative command, refused by the control server, like Apply Image
  (recordable).

## Decisions

### Read sources through the Apply Image readers

Both pickers use the existing snapshot bridge (`apply_image_read`), and the
calculations FFI entry points live in that same `image_apply_image` bridge,
which owns the opaque `ApplyImageSnapshot` type; a second cxx-qt bridge cannot
take that type. The dialog's `Gray` entry sends the composite code; the kernel
reduces any plane set to a scalar with the Grayscale conversion weights
(Rec.601 `0.299/0.587/0.114`, the same as `document_ops/mode/planes.rs`), not
`nonseparable::lum` (`0.3/0.59/0.11`), so `Gray` matches an Image ▸ Mode ▸
Grayscale conversion. A colour channel (stored as `[i, i, i]`), an alpha
channel, a layer mask, Transparency, or the selection is unaffected because
their planes are already equal on all three slots. This adds one public
grayscale accessor on `ApplyImagePlanes` and no new reader type.

- Alternatives: a dedicated calculations reader (duplicates plane placement,
  native depth, mask geometry); per-channel C++ reads (copies whole planes
  across FFI per control change).

Source alpha is ignored: a channel is opaque brightness data, and the docs do
not define transparency as modulating the calculation. Layer mask and
Transparency remain explicit channel choices.

### New `calculations` module in `pictura-render`

`apply_image.rs` is 636 lines and the repo target is 800; a sibling module
keeps the new kernel and its tests separate. `calculations.rs` reuses
`ApplyImageMode` and `blend::blend`:

```text
A = grayscale(Source 1), inverted if asked
B = grayscale(Source 2), inverted if asked
C = blend(mode, A, B)                 # Add/Subtract use Scale + Offset
C = A + (C - A) * opacity             # opacity mixes toward Source 1
C = C * mask                          # black hides, white reveals
```

Work is unit `f32` per pixel, ray-parallel by row, and written once as `u8`.
Add/Subtract stay unclamped in `f32` until the final narrowing. A layer source
keeps above-1 `f32` values, while a merged 32-bit source is clamped to `[0, 1]`
by the existing composite read (`composite_native`) before the kernel sees it.
16-bit sources keep precision finer than one 8-bit step.

- Alternative: return `Samples` at the active depth and build native result
  documents. Rejected: channels and selections are 8-bit in the model, and a
  native-depth new document needs a layer-store creation path this change
  does not justify.

### Results reuse existing writers, with the proper history label

`calculations_apply` computes once and dispatches Selection or New Channel;
New Document uses the separate grayscale writer from `menu_image.cpp`:

| Result | Writer | History |
|---|---|---|
| Selection | `apply_selection_labeled(shape, New, "Calculations")` | one `Calculations` state |
| New Channel | `save_calculations_channel` on `PictureView` (`impl_selection.rs`) wraps `edit("Calculations", ...)` + `save_selection_shape(NewChannel)` | one state; refusal at the cap returns `Edit::None` and records nothing |
| New Document | a grayscale new-document writer called by `menu_image.cpp` on a fresh `PictureView` | the new document starts with its `New` state; the source is untouched |

The existing coverage bridges (`save_selection_coverage`,
`load_selection_coverage`) are not used directly because their labels are
"Save Selection"/"Load Selection"; the spec requires a `Calculations` state.
`save_selection_shape` is `pub(super)` inside `impl_selection/save_load.rs`, so
the app adds a `PictureView` helper (`save_calculations_channel`) in
`impl_selection.rs` that wraps `edit("Calculations", ...)` and
`save_selection_shape(NewChannel)`; the bridge calls that helper. The dialog
never creates documents: on Accepted, `menu_image.cpp` creates a new
`PictureView`, calls the grayscale writer, and `addDocument`s it, leaving the
source document untouched. The dialog refuses acceptance at the cap and names the 56-channel limit; the
bridge returns false and records no state. The new-document writer mirrors `new_document`'s grayscale
construction (channel `0` plus `-1`, `LayerParts`, `DocumentComposite::refresh`)
rather than `new_selection_document`, whose result is an extra channel, not a
pixel layer.

### Preview is a dialog thumbnail

The bridge returns the full-resolution blended plane bytes and size; the
dialog scales that plane into a `QImage` for display, and Preview shows or
hides it. Sources are snapshotted once per picker change; without a canvas
preview there is no feedback into a source on the same document, so no re-read
protocol is needed.

- Alternatives: reuse the filter preview session (writes a layer; impossible
  without a target); extend `mask_solo`/tiles with an arbitrary solo plane
  (larger: tile keys, damage, session slot, navigation); no preview (spec
  requires one).

`ponytail:` the bridge computes the full-resolution blend and the dialog
shrinks it for display. If slider latency on very large documents matters,
decimate the source snapshots for the preview only; the committed result stays
full resolution.

### Command and dialog wiring

- Catalogue: flip `image.calculations` to implemented, add
  `ImageCalculations` to `const_names.rs`, regenerate `command_ids.h`, add the
  id to `modal_ids()`, update the catalogue/const-name goldens.
- Menu: handler and enabled provider in `menu_image.cpp` beside Apply Image,
  but without the filterable-target and Smart Object checks: Calculations
  writes no layer. It is disabled on Indexed/Bitmap and without a document.
- Dialog: mirror `ApplyImageDialog`'s picker rebuild rules. The Channel list
  omits the composite and always offers Gray; Selection appears only in the
  Mask picker; Add/Subtract are inserted once from the shared blend table and
  Dissolve is omitted (no positional dither, same as Apply Image). Scale and
  Offset rows follow the mode. Result drives the handler's commit path: the
  dialog returns its settings and target on Accepted; Selection and New
  Channel commit through the bridge, and New Document creates a fresh
  `PictureView` in `menu_image.cpp`.
- Enablement and refusal messages are app-level; the spec only requires that
  refusals leave state unchanged and report the reason.

### Documentation

SEL-013's open questions are resolved in `docs/08-selection/channel-based-
masking.md` and marked inferred where CS6 is unverified (Result default,
Selection combine, mask/opacity base, preview location, 8-bit narrowing). The
parity register row SEL-2 becomes SHIPPED, `ROADMAP.md` ticks the item, and
`docs/dev/STATE.md` records the change.

## Risks / Trade-offs

- [Channel and merged reads use the RGB working planes for CMYK/Lab sources] →
  matches Apply Image's documented approximation; recorded in the spec's
  parity notes and the doc's open questions.
- [Mask and opacity bases are inferred] → tests pin the chosen semantics
  (`cal_mask_hides`, `cal_opacity`); docs mark them inferred, not parity.
- [Two source snapshots plus the result multiply memory on large documents] →
  the snapshots are shared (`Arc`) and the result is one byte per pixel;
  preview decimation is the upgrade path if latency appears.
- [A new-document writer could desync the layer and composite] → build both
  from the same plane and test composited pixels against the blend
  (`cal_result_document`).
- [Channel-cap refusal could add history] → return `Edit::None` at the cap so
  no history state is recorded; asserted by `cal_result_channel_cap`.
- [Preview and commit could diverge] → both call the same kernel;
  `previewMatchesCommit` compares the full-resolution preview plane with the
  committed result (`cal_preview_matches_commit`).

## Open Questions

- Whether CS6's Calculations preview shows the grayscale result over the
  active document or only a thumbnail — resolved here as a dialog thumbnail:
  the bridge returns the full-resolution plane and the dialog scales it; a
  canvas preview is a separate rendering capability.
- Whether Photoshop preserves 16/32-bit depth in a New Document result; this
  change narrows to 8 bits and records the deviation.
