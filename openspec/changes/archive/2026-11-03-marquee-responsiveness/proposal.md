# Proposal

## Why

Issue #1150. After #1145, the Marquee still stalled on a 16507×16196 document:
the issue measured a tool switch of 165–185 ms and a release of about 250 ms
plus about 110 ms to settle. A `perf` profile of the GUI thread at 8254×8098,
headless and on an Xvfb display, placed the time:

- **Every pointer move with a selection scanned the whole mask.** The Info
  panel's W/H readout called `Selection::bounds` per move: 9 ms a move at
  8254×8098, 83 % of a hover and 40 % of a drag-and-release.
- **A release built and copied a document-sized mask.** `Selection::none`
  zeroed a vector and `Plane::from` copied it (7–13 %).
- **The outline traced the whole mask** to find a box-sized edge (3–5 %).
- **`pictura::cursor()` rendered its SVG and made a new `QCursor` on every
  call**, and the Marquee asks for one on every hover move (2–5 %). On X11 each
  new pixmap cursor is a new X cursor.
- **The settle was the measurement.** The control server's `status` counted the
  selected pixels, 41 ms at 8254×8098, and `profile-tools.py` times `status`.

The switch itself cost about 5 ms headless and on Xvfb, with or without a
selection; it never read the mask.

## What Changes

- **The selection's bounds and selected-pixel count are read once per selection
  revision** and reused until the selection is written. A selection move preview
  still offsets the cached bounds by its drag delta.
- **The outline traces only the selection's bounds.** The result is unchanged.
- **Rectangle and ellipse masks are built in place** in zeroed pages, without
  the zero-fill and copy.
- **Cursors are built once per asset, hotspot, angle and device pixel ratio**
  and reused.

## Capabilities

### New Capabilities

- `tools/marquee-responsiveness`

### Modified Capabilities

None.

## Impact

`pictura-select` (`contour`, `Selection::{none, all, rect, ellipse}`),
`pictura-app` (`PictureViewRust` bounds and count caches, `combine_selection`,
`pictura::cursor`), and `marquee_selection_profile`.
