# Proposal

## Why

`File > Place…` still accepts one file per dialog, so bringing a batch of
references into a document takes one dialog and one transform session per file,
repeated by hand. The `File > Open` half of #755 already shipped in #1118; the
Place half is the remaining gap. Photoshop CS6 places one file at a time, so
multi-file Place is an intentional enhancement over the CS6 contract, not a
parity claim.

## What Changes

- **`File > Place…` accepts a multi-file selection and offers an `Images (…)`
  filter.** The dialog uses `QFileDialog::ExistingFiles`, keeps the existing
  filter list plus the `Images (…)` entry beside the Photoshop entries, and
  places each selected file in the order the dialog returns.
- **Each file is placed sequentially with its own Free Transform.** A success
  records its own `"Place"` state and starts Free Transform on that layer. The
  next file is placed only after the user commits with Enter or the apply
  control, cancels with Escape or the transform's Cancel control, or ends the
  session another way. A changed commit records one `"Free Transform"` state; an
  identity commit records none and still advances.
- **Escape or the Cancel control cancels the current item.** The just-placed
  layer and its `"Place"` history state are removed. If more selected files
  remain, one dialog asks whether to cancel the whole Place or continue with the
  next item; Escape or Cancel on the last item ends without the dialog.
- **A non-Escape, non-interruption session end keeps the placement.** A tool
  switch choosing Don't Apply, or another such session end that leaves the
  `"Place"` state, keeps the placed layer and its state and continues with the
  next file. If an undo already removed the `"Place"` state, the item is treated
  as cancelled without a second undo.
- **Interruptions abort the batch.** Another document becoming active or closing
  the document aborts the batch; the original document keeps its placed layers
  and no summary is shown.
- **File failures are summarized once.** After the sequence, one dialog names
  files that could not be placed (missing, unreadable, undecodable, unsupported,
  over budget, or refused by the placement). A profile-warning cancel, a
  transform cancel, and a cancelled batch are not failures and are not listed.
- **Place is unavailable without an active document or while a transform is
  active.** The command handler refuses before opening the dialog when the
  active view has no document or a transform session is active; `placeDocuments`
  rechecks the same condition at entry for dispatch and re-entry, and a
  per-window set ignores a re-entrant batch.
- **The canvas-drop path is unchanged:** it still places every dropped file and
  transforms only the last, and cancelling that session keeps the layer.
- **Fix the per-file profile-policy leak.** `place_image` consumes the pending
  incoming-profile override before any read or decode failure, so one failed
  raster cannot apply its colour choice to the next file.
- Not in scope: `File > Open` changes, Open Recent, Open As, Open As Smart
  Object, Replace Contents, PDF/AI vector Place, video Place, multi-PDF
  Import-dialog queueing, async progress, and batch undo grouping.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/smart-object-layer-actions`: the Place command gains a
  multi-file selection, per-file routing, and a single failure summary while
  keeping one `"Place"` state per successful file.
- `document/free-transform`: the Place session rule is extended for the menu
  command — one session per placed file in sequence, with Escape or the Cancel
  control cancelling the current placement and asking before the next item, and
  every other non-interruption session end keeping the placement and continuing;
  the canvas-drop rule keeps its current last-layer and keep-on-cancel
  behaviour.

## Impact

- `crates/pictura-app/cpp/frame_menus.cpp` (handler and enablement),
  a new `frame_place.{h,cpp}` for the sequence, wait, and dialogs,
  `crates/pictura-app/cpp/dialogs.cpp` (the `Images (…)` filter),
  `crates/pictura-app/cpp/options_bar_transform.cpp` (the Cancel-control
  signal or filter for the placement cancel), `cmake/sources/frame.cmake`, and
  `crates/pictura-app/src/cxxqt_object/impl_layers_smart_object.rs`
  (`place_image` policy consumption).
- `crates/pictura-app/cpp/tests/tst_multi_file_open.cpp` (new Place cases and a
  dialog/transform driver); no new test target.
- `docs/10-workflow-io/open-and-new.md`, `docs/05-layers/smart-objects.md`,
  `docs/dev/STATE.md`, and `ROADMAP.md`.
- No new dependencies, no engine or codec change, and no change to the
  drag-and-drop router.
