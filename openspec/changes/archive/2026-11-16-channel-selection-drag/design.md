# Design

## Context

See `proposal.md` for motivation. The pieces that shape the approach:

- `Selection` is an 8-bit coverage plane with `combine`, `invert`, and channel
  conversion in `crates/pictura-select/src/lib.rs`.
- `impl_selection` already transfers selection coverage between views as bytes
  for the Save/Load dialogs. Those paths require equal dimensions.
- `FileDropRouter` already routes Layers-panel drags to another document's tab
  or canvas and is installed on every document canvas.
- The Channels panel already starts a channel drag with
  `application/x-pictura-channel-row`, but the payload carries only the row
  index and only the Load button accepts it.
- ToolController already detects a pointer over another document's tab for the
  Move tool's layer drag.

## Goals / Non-Goals

**Goals:**

- One resize policy for every cross-size drag.
- Duplicate Channel through the menu and through both drop targets.
- Copy semantics for the selection outline: the source keeps its selection.
- One history state in the destination, none in the source.

**Non-Goals:**

- Changing the Save/Load dialogs' same-size rule.
- Move-tool content transfer; that remains a layer drag.
- Resampling selection channels at 16/32-bit precision (#1331).

## Decisions

### 1. Scale to fit, preserve aspect ratio, centre

`Selection::fit_centered(target_width, target_height)` computes
`scale = min(target_w / source_w, target_h / source_h)`, scales the source plane
to the rounded fitted size with bilinear sampling, and places it at the integer
centre of the target. The uncovered border is 0. Same-size input returns a clone
so no resampling touches the common path.

Alternatives rejected:

- **Stretch to the target:** distorts masks and makes the policy surprising.
- **Top-left crop/pad:** loses pixels when the source is larger and leaves the
  content off-centre when the target is larger.

The exact Photoshop CS6 mapping is undocumented; this policy is inferred and
marked as such in the parity register. It is explicit, deterministic, and
shared by both drag paths.

### 2. Channel drag payload identifies the source

The drag carries the source `PictureView` address, the panel row, the alpha
index (`-1` for a colour row), the visible-channel mask, and the source display
name. The drop reads the source coverage on demand instead of copying a
document-sized plane into `QMimeData`. This mirrors the Layers-panel drag, which
carries the source document and a layer path.

### 3. Coverage transfer stays in Rust

The C++ layer never resamples. It asks the source for row coverage and passes
the bytes, source dimensions, name, and Invert to the destination:

- `color_row_coverage` for a colour/composite row.
- `channel_coverage` for an alpha row.
- `duplicate_channel_coverage` fits to the destination, applies Invert, adds a
  new named channel, and records one `Duplicate Channel` state.
- `load_selection_coverage_fit` fits to the destination and records one
  `Load Selection` state.

The existing same-size dialog transfers keep `save_selection_coverage` and
`load_selection_coverage` unchanged.

### 4. Selection drag is copy-only

The selection-tool drag cancels the in-canvas move (restoring the source
selection), then starts a `QDrag` carrying the source view. The drop router
loads the source's live coverage into the destination. The source records no
history; the destination records one `Load Selection` state.

The Move tool is excluded: its drag already carries a layer to another document.
A Ctrl-drag content move is also excluded for the same reason.

### 5. Duplicate Channel dialog

The dialog takes the source channel's display name and the same-size open
documents. It adds a `New` destination at the end. The default name is
`<source> copy`; the default destination is the active document. `New` reuses
the grayscale-plus-alpha fallback from Save Selection because the codec does
not write Multichannel.

### 6. Menu and command wiring

`Duplicate Channel…` becomes a real command in the catalogue and the panel menu
map, with an enabled provider that requires a colour or alpha row. The handler
opens the dialog. The panel footer's New Channel button becomes a drop target
for channel rows.

### 7. History and refusals

- Same-document duplicate: one `Duplicate Channel` state.
- Cross-document duplicate: one `Duplicate Channel` state in the target, none in
  the source.
- New document: the normal `New` state for the created document.
- Selection drag: one `Load Selection` state in the target, none in the source.
- At the 56-channel cap, an invalid payload, or a missing document: no state.

## Risks / Trade-offs

- [Full-plane coverage copy per drag] -> user-initiated and one-shot; the same
  trade-off the Save/Load dialogs already accept.
- [Bilinear resampling softens an upscaled mask edge] -> coverage is already
  fractional; no thresholding is introduced.
- [Source view address in MIME data] -> valid only inside this process, which is
  where the drag runs. A missing view is refused without history.
- [Drag source cannot be driven in offscreen Qt] -> the drop router and the
  preparation seam are automated; the platform drag loop stays a manual check.

## Migration Plan

No data migration. Existing channels and selections are untouched; the new
operation only appends channels or replaces the live selection.
