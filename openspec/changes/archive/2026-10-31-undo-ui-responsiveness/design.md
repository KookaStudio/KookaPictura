# Design

## Where the time goes

`show_restored` emitted `sessionCancelled` for every restore, and the handler
added in #974 runs `refresh()` and `refreshPanels()` inline for gestures that
were cancelled. `changed` had already run `refresh()`, so a restore paid for the
frame refresh twice and bypassed the panel debounce.

An in-place undo forks the bands it writes, so the live plane has diverged.
Indexing a `Plane` derefs through `as_slice()`, which materializes the whole
plane. `layer_thumbnail_image` indexes for every sample.

## Decisions

**A new signal, not a flag on `sessionCancelled`.** The tools need the
notification either way; only the inline panel pass is unwanted. A restore that
ended a live session keeps `sessionCancelled` because its display may have been
rebuilt.

**`Plane::sample`, not `Plane::row`.** `row` trusts the width the plane was
fractured at, while `sample` addresses the same logical index the old indexing
did, so output stays byte-identical at any stride.

**Mask tiles join the damage rectangle.** The picture change is already in the
composite tiles. Mask tiles are added so a solo mask view, which
`apply_refreshed_region` patches through `sync_mask_solo_region`, is covered even
when the composite did not change. A mask tile maps to the document only through
its mask rect, so a walk over any state that places a mask differently reports
the whole document.

## Risks

Panels lag the canvas by up to 120 ms after undo, as they do after a paint commit.
