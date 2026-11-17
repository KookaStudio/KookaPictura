# Proposal

## Why

`SEL-012` shipped cross-document Save/Load Selection through dialogs (#615), but
the two more permissive drag paths are still missing (issue #1332):

- The Channels panel has no `Duplicate Channel` command, and a channel drag
  cannot leave the panel except onto the Load Selection button.
- A live selection outline cannot be dragged into another document.

Both paths deliberately relax the dialogs' same-size rule, so the resize policy
must be explicit rather than silently inherited from the same-size dialog code.

## What Changes

- Add the Channels panel `Duplicate Channel…` command: Name, Destination
  (same-size open documents plus `New`), and Invert.
- Duplicate within the document, into another same-size document, or into a new
  grayscale document whose extra channel holds the duplicated coverage.
- Dragging a channel row onto the Create New Channel button duplicates it within
  the document.
- Dragging a channel row onto another document's tab or canvas duplicates it
  across documents without a size requirement.
- Dragging a live selection outline from a selection tool into another document
  copies the selection; the source keeps its selection and records no state.
- Cross-size drags scale the source coverage to fit the destination while
  preserving aspect ratio and centre it; areas outside the fitted plane are
  empty. The source plane is never stretched.
- Record one `Duplicate Channel` state in the destination for channel
  duplication and one `Load Selection` state in the destination for selection
  drags. Refusals record nothing.
- Add the shared `Selection::fit_centered` engine operation and tests for it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/channels-panel`: gains Duplicate Channel, the duplicate dialog, the
  Create New Channel drop target, and cross-document channel drags.
- `tools/selection-channels`: gains `Selection::fit_centered`, cross-document
  selection-outline drags, and the fit policy shared with channel drags.

## Impact

- `pictura-select`: `Selection::fit_centered` and its unit tests.
- `pictura-app` Rust bridge: colour-row coverage, coverage duplication with
  fit/invert, fit-loading a transferred selection, and an Invert argument for
  the New-document path.
- `pictura-app` C++: Duplicate Channel dialog, Channels panel menu and drop
  handling, selection drag source, document drop routing, command catalogue and
  menu wiring.
- Tests: `pictura-select` unit tests, `tst_channels_panel`, and
  `tst_selection_save_load`.
- Docs: parity register and `ROADMAP.md` close-out for #1332.

## Out of Scope

- Selected Areas polarity (#611), 16/32-bit channels (#1331), colour channels as
  Save/Load dialog targets (#1333).
- Spot channels, Split/Merge, Channel Options, and Panel Options (#513).
- Move-tool content drags between documents; the Move tool keeps its existing
  layer drag.
