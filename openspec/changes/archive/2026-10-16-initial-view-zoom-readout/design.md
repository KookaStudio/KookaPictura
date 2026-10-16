# Design: initial-view-zoom-readout

## Root cause

`setDocument` runs during `addDocument`, before the tab is shown. A widget
that has never been shown receives its laid-out geometry without a resize
event, and the one resize event the canvas does get (the grid re-asserting the
320x240 minimum) arrives while the settle is invisible. `applyInitialView`
then set `userAdjusted_`, so the resize that delivered the real viewport took
the clamp-only branch: the stale minimum-size fit (8.75 %) stayed and its
centre-offset clamped to the top-left.

## The settle flag

`pendingSettle_` is the "the initial fit may be stale" marker, separate from
`userAdjusted_` (which keeps its meaning: a resize of a settled view keeps the
zoom).

- `setImage`/`setDocument` set it when a document or image lands.
- While it is pending, every `resizeEvent` re-fits, so hidden intermediate
  sizes never win.
- `showEvent` re-fits synchronously (the silent geometry is real by then) and
  posts one queued re-fit that ends the settle, covering a multi-pass layout
  that finishes after the show returns.
- Every explicit view change — `panBy`, `setOffset`, `setZoom`, `actualPixels`,
  and the pan press sites — clears the settle, so a user view chosen before the
  show pass is never snapped. `fitOnScreen` leaves it alone: a fit computed at
  a stale viewport should still be corrected by the settle.

## File size

The view-state family (`setImage` through `actualPixels`, plus `setZoom`) moves
to `image_view_document.cpp`, the existing split for what the canvas presents.
Pure moves; `kMinZoom`/`kMaxZoom` move with them.

## Tests

`tst_initial_view` (Qt Test, offscreen):

- a 512x512 document in a 1400x1000 window settles at zoom 1.0, centred;
- a 2000x2000 document settles at the fit zoom for the real viewport, centred;
- a user zoom survives later window resizes (the settle does not snap it).

`tst_smoke` `statusBarChromeAndZoom`:

- a fractional zoom (0.5234) renders as the integer percent "52 %" that fits
  the value box, and the tab title shows the same "@ 52 %".

## The status-bar zoom readout

`NumericField` reserves the value box width from the widest value of the
configured range ("3200" plus the inside suffix), which holds exactly the
integer percent format the field is configured for (`decimals == 0`). Feeding
it the raw zoom percent made `formatValue` fall to its keep-the-fraction path
("52.3438"), which both clips and disagrees with the tab title's `qRound`
percent and the Navigator's `'f', 0` label — the readout conformance the
`ui/application-shell` status-bar requirement already states. `updateStatus`
now feeds the same `qRound` percent the tab title uses, so every displayed value
is an integer in range and fits by construction.
