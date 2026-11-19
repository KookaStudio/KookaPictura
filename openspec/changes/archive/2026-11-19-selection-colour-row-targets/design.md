# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- The Save/Load dialog emits string targets — `new`, `alpha:<index>`,
  `mask:<layer path>` — parsed in `impl_selection/save_load.rs` and executed by
  `save_selection_target` / `selection_from_target`.
- A spot channel is an ordinary extra channel (`doc.channels`, display-info
  mode 2), so both dialogs already list it through `alpha:<index>`.
- Colour rows are derived, not stored: `channel_row_selection_from`
  (`impl_selection/channel_rows.rs`) derives a row from the streamed picture and
  is already the source of the Channels panel's Ctrl-click load and Duplicate
  Channel. It is exposed to C++ as `color_row_coverage(view, row, mask)`.
- `frame_selection_dialogs.cpp` already owns cross-document transfer: it copies
  a source coverage plane and calls the destination view's
  `load_selection_coverage`, which enforces same size, invert, combine, and one
  history state.
- Row names and shortcuts exist only as the local `channelsForMode` table in
  `panels/channels_panel.cpp`.
- The `Select > Load Selection` enablement provider in `menu_select.cpp` gates
  the command on a same-size document having an extra channel or raster mask;
  with colour rows every same-size document has a source.

## Goals / Non-Goals

**Goals:**

- One source of truth for colour-row labels and order.
- Colour-row loads identical in coverage to the Channels panel's Ctrl-click,
  with every colour channel treated as visible.
- Load is enabled for every document, not only ones with saved channels.
- No new Rust target arm, no new codec or storage surface.
- Save's list unchanged, with the exclusion documented rather than implied.

**Non-Goals:**

- Writing a selection into a colour channel (Save target). The parity gap and
  the conditions for revisiting it are recorded instead.
- Modelling channel visibility in the Load dialog. The dialog has no eye
  column; it always loads as if every colour channel is visible.
- Adding spot-specific rows or labels; spot keeps its extra-channel row. Spot
  also stays a Save target: it is a stored extra plane, so the write is the
  existing undoable channel write and needs no destructive warning.
- Any change to the Channels-panel load behaviour.

## Decisions

### 1. Load-only: colour rows are not Save targets

The colour row is a projection of the composite. In a layered document the
composite is a cache (`picture.rs` / `document_ops/mode/planes.rs`), so a write
has no authoritative plane unless a layer is chosen — and the dialog has no
layer control. Implementing it would need a target-layer rule, a destructive
confirmation, RGB-only inverse mapping for CMYK/Lab, and a damage path; all of
that is a separate decision.

Alternatives rejected: writing the derived composite cache (invisible after the
next recomposite); offering colour rows only for single-layer or non-RGB
documents (a mode-dependent list is worse than a stable one).

The Help-derived "any existing channel" wording in
`docs/08-selection/save-and-load-selections.md` is corrected in the parity
register and the doc line in the same pull request (docs commit,
`TASK-ALLOWS-DOCS`).

### 2. Colour targets ride the coverage path, not a new `SelectionTarget`

The dialog encodes a colour row as `color:<row>`. `frame_selection_dialogs.cpp`
detects that prefix and, for both same-document and cross-document, obtains
coverage with `color_row_coverage(source_view, row, 0x7)` and applies it with
`load_selection_coverage(active, ...)`.

Why: `selection_from_target` in Rust only receives `&Document`, while the
derivation needs the streamed picture; threading the stream into the target
parser would duplicate the colour arm for save and load. The coverage pair is
the same one the cross-document path and Duplicate Channel already use, and it
records the "Load Selection" state and applies Invert/Operation exactly once.

Alternative rejected: adding `SelectionTarget::Colour` — more bridge surface for
identical behaviour.

### 3. List order and default

Channel list order mirrors the panel: composite row, colour plates, layer-mask
rows, extra channels. The default selection is the first extra channel when one
exists, else the first listed colour row (the composite row where the mode has
one). The first-extra-channel default is our inferred choice; the contract does
not state CS6's default (`docs/08-selection/save-and-load-selections.md:89` and
`:198` leave it open).

The `Select > Load Selection` enablement provider in `menu_select.cpp` is
reduced to `selectHasDoc`: once colour rows are a source, every same-size
document with a picture has at least one row, so the old extra-channel/mask gate
would keep the command disabled for the exact empty-document case this change
fixes.

Alternative rejected: colour rows last. It preserves the old default by
accident but stops matching the panel, which is the mental model both surfaces
share.

### 4. The composite row loads with every channel visible

The dialog passes `mask = 0x7`. The panel's `mask_` reflects its eye column; the
dialog has none, so "all visible" is the only truthful constant. This is stated
in the spec so the panel's visibility-sensitive behaviour stays distinct.

### 5. One row table shared by panel and dialog

Move `channelsForMode` into a small header (`channels_rows.h`) with the row name
and shortcut list, and have both `channels_panel.cpp` and
`save_load_selection_dialog.cpp` read it. Pure move: no behaviour change, no new
translation unit; the header is listed in `cmake/sources/dialogs.cmake` for
IDE grouping only. The shared table pins
the colour-row labels and order; the per-mode row layout remains
`channel_rows.rs` and is pinned by the `loadDialogModeRows` test.

### 6. Coverage derivation reused as-is

No change to `channel_row_selection_from`: the dialog inherits the mode layout
(RGB/Lab composite + 3, CMYK composite + 4, Multichannel 3 without composite,
single-row modes one composite) and the naive CMYK/Lab conversions the
thumbnails already use, including the existing `ponytail:` note. Lab plate rows
also use the luminance (the panel has no separate Lab plane).

### 7. Test shape

Dialog-level behaviour (listing, order, default, same- and cross-document load)
is Qt Test in `tst_selection_save_load`. The derivation itself is already
unit-tested in `impl_selection/tests.rs`; the plan cites those tests rather than
duplicating them. `loadColourRowThroughFrame` drives the real command/frame path
(registry dispatch of `SelectLoad` plus a QTimer that fills and accepts the
dialog, or a minimal frame test seam if the modal is impractical), so the
`color:` routing in `frame_selection_dialogs.cpp` is exercised rather than
reimplemented in the test.

## Risks / Trade-offs

- [Streaming a whole picture for one dialog action] → the coverage route returns
  a full `Vec<u8>` across FFI and `load_selection_coverage` copies it, unlike
  the panel's in-Rust path. It buys Invert/Operation reuse; it is not copy-free.
- [Existing tests assert the old empty list] → update
  `loadDialogWithNoSavedChannels`, `loadDialogCrossDocument`, and the
  channel-count assertions in `loadDialogDefaults`; the new behaviour is the
  point of the change.
- [Row labels drifting between panel and dialog] → the shared table pins the
  colour-row labels/order, and `loadDialogModeRows` pins the Rust mode layout
  against the dialog list.
- [Docs contradict the shipped Save list] → parity-register row and the SEL-012
  line updated in the same pull request.
- [Loading a colour row is destructive only to the selection] → it records one
  normal "Load Selection" state, so undo is unchanged.

## Migration Plan

No data migration. No codec or file-format change; targets are runtime strings.
Reverting the change restores the previous dialog list without affecting stored
documents.
