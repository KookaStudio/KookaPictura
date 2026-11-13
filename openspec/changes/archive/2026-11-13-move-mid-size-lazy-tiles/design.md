# Design

## D1. The gap

`refresh_display_rect` leaves a non-authoritative region stale when the
document is not tile-synchronous (over 16 MP) and the aligned area is over
`REPAIR_PIXELS` (4 MP), or when no tile under it is current. `should_leave_tiles`
required a dirty area of at least `DEFERRED_MOVE_PIXELS` (16 MP). Between the two
thresholds the commit was synchronous from the Move's point of view, but the
canvas still had stale tiles.

## D2. Decide by the tiles, not the area

`large_move_area` keeps the document-size and idle-display conditions and returns
the dirty area. `should_leave_tiles` accepts any non-empty area, and
`should_defer_move` keeps its own 16 MP area floor. `request_lazy_move_tiles`
already checks the presented tiles after the refresh, so it decides whether there
is anything to wait for.

## D3. No double report

When nothing is stale, `request_lazy_move_tiles` clears the flag without
emitting. Every caller that shows a preview (`commit_move`, the transform
commits) already emits `moveCommitted` when nothing is pending.

## D4. A chained Move

`commit_move_edit` and `commit_transform_edit` settle any pending work before
they run. For a lazy Move that settle used to emit `moveCommitted`, and the
canvas, which cannot tell one Move's signal from another's, dropped the preview
of the drag just released. `take_over_lazy_move` clears the flag first, so the
new commit's `request_lazy_move_tiles` decides whether to wait; its presented
tile set includes the tiles the earlier Move left stale.

## D5. Stale tiles carry the Move's result

Any preview (a second Move, a transform) draws the presented tiles under a
backdrop patch of its own target. After a chained Move some presented tiles are
still stale and show the earlier Move's older position outside that patch.
Growing each patch over the stale tiles works but composites most of the view at
press time (about a second at 20000 px), so instead the commit fixes the tiles'
stand-in.

The view preview keeps its backdrop and moving image as straight sRGB pixels
(`MoveViewPixels`) next to the `QImage`s. After a lazy commit,
`bake_move_stand_in` paints the backdrop over the layer's old place and the
moving image at its new place into the presented level's stale tiles through
`TilePyramid::paint_stand_in`, which touches stale tiles only and leaves them
stale. The worker rebuild replaces the stand-in exactly. Layers above the moved
one and non-Normal blending are not reproduced in the stand-in (a `ponytail`).

## D6. A reduced stand-in for transforms

`paint_reduced_stand_in` composites a `FilterProxy` copy of each changed rect
(clipped to the presented view, aligned to the level) and paints it into the
stale presented tiles. It is skipped when the document does not reduce
faithfully. `commit_transform_edit` calls it after a lazy commit and, when it
painted, clears `lazy_move_pending`: `endTransformOverlay` keeps the layer's
untransformed image as a move preview until `moveCommitted`, and that image
would hide the correct stand-in. The cost lands inside the transform commit's
progress window.
