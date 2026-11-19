# Proposal

## Why

`Select > Load Selection` can only read extra (alpha/spot) channels and raster
layer masks, so a document with neither has an unusable dialog: the Channel list
is empty and OK is disabled. The Channels panel already loads the same
document's colour rows by Ctrl-click, and CS6 exposes colour channels as load
sources, so the dialog is both narrower than the panel and narrower than the
contract (`docs/08-selection/save-and-load-selections.md`, issue #1333,
follow-up to #615).

Save Selection needs an explicit decision rather than silence: the CS6 contract
allows "any existing channel", but in this document model the colour
rows are a derived view of the streamed composite, not stored planes, so there
is no single channel to write into.

## What Changes

- Load Selection's Channel list gains the chosen document's colour rows —
  composite plus per-plate rows, in Channels-panel order — ahead of the layer
  mask and extra-channel rows. Each row is labelled as the panel labels it and
  uses the same derivation, with every colour channel treated as visible.
- Load's default selection is the first extra channel when one exists, else the
  first listed colour row (the composite row where the mode has one). Load is no
  longer disabled for a document with a picture and no saved channels.
- Colour rows follow the document mode: RGB/Lab list the composite plus three
  plates, CMYK the composite plus four, Multichannel three plates without a
  composite, and Grayscale/Indexed/Bitmap/Duotone the single composite row.
- Cross-document colour-row loads use the existing same-size coverage transfer;
  a colour row from another open document loads without new Rust bridge surface.
- **Decision — Save Selection does not offer colour rows.** Save keeps its
  current list (New, every extra channel including spot, every raster layer
  mask). Colour rows are derived, and writing a mask into one would first need a
  rule for which layer owns the write, which the dialog has no control for.
  Spot channels remain Save targets because a spot channel is a stored extra
  plane, so the write is the existing undoable channel write and needs no
  destructive-warning confirmation; only colour rows are excluded.
  The divergence from the doc's "any existing channel" wording is recorded in
  the parity register rather than implemented destructively.
- The Channels-panel colour-row labels and order become one shared table used by
  the panel and the dialog, so the colour-row lists cannot drift. The per-mode
  row layout is pinned by the `loadDialogModeRows` test.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/select-menu`: the Load Selection requirement lists colour rows, fixes
  the default source and removes the "disabled with no channels" condition; the
  Save Selection requirement records that colour rows are not Save targets.
- `tools/selection-channels`: loading derives coverage from a colour row using
  the Channels-panel row derivation, in addition to extra channels and masks.

## Impact

- `pictura-app` C++: `save_load_selection_dialog.*` (Channel list, default
  selection, target encoding), `frame_selection_dialogs.cpp` (route `color:`
  targets through the existing coverage transfer), `menu_select.cpp` (drop the
  `SelectLoad` extra-channel/mask gate so the command follows `selectHasDoc`,
  and refresh the registry), and a shared channel-row table extracted from
  `panels/channels_panel.cpp`.
- `pictura-app` Rust: no target-parser change; the existing
  `color_row_coverage` and `load_selection_coverage` bridges carry both
  same-document and cross-document loads.
- Tests: Qt Test cases in `tst_selection_save_load`, including
  `selectLoadEnabledWithoutChannels`, `loadDialogModeRows`, and
  `loadColourRowThroughFrame`.
- Docs: parity register entry for the Save decision; the SEL-012 Save and Load
  lines remain review items in the same pull request.
- No codec, storage-format, or channel-capacity change.
