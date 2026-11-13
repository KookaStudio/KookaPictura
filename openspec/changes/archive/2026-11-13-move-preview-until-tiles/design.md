# Design

## D1. Why the preview must stay

The canvas paints the live Move preview directly and bypasses the present-crop
tile path. Ending the preview immediately exposes the stale tiles underneath:
the old layer position stays visible until the tile worker rebuilds the
presented level. Keeping the preview shows the correct moved image at once.

## D2. Pending state

`PictureViewRust::lazy_move_pending` records that a lazy commit left its tiles
stale. `move_composite_pending` includes it, so the C++ release keeps the preview
and `moveCommitted` is the only thing that drops it.

`commit_move_edit` and `commit_transform_edit` ask the presented tiles to build
right after the edit. Without that request the preview paint path would never
ask for them.

## D3. Completion

`land_tiles` checks, after a worker lands, whether the lazy Move is pending and
the presented tiles are all current. If so it clears the flag and emits
`moveCommitted`. A reader that settles a pending move first (`settle_pending_composite`)
clears the flag and emits the signal too, so an edit or another press never
inherits a stale preview.

## D4. Cost

The visible tiles still rebuild through the normal tile worker, which composites
their level-0 footprints. The change does not make that worker cheaper; it makes
the canvas show the correct moved image while the worker runs. Off-screen tiles
stay stale until presented.
