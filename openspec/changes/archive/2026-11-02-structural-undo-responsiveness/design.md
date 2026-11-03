# Design

## Measurements

`structural_restore_profile`, 16507×16196, 12 threads, before this change:

| Restore | History | Display |
|---|---|---|
| Identical reorder | 48 ms | 408 ms |
| Document channel | 15 ms | 417 ms |
| Undo of a resize | 0.06 ms | 315 ms (level 0 78 ms, pyramid 230 ms) |
| History jump, quarter-area state | 0.15 ms | 712 ms (104 ms without the composite) |

A cheaper synchronous rebuild cannot meet the 100 ms geometry target, because
the pyramid alone costs 230 ms. So the pyramid moves off-thread.

## Restored

`Region` keeps its meaning. `Composite` is new: the composite changed only inside
the rectangle, but masks changed in a way the rectangle does not cover. Only
`Composite` rebuilds a soloed mask, so a bounded mask undo while soloed stays
bounded.

## Deferred pyramid

`ViewPyramid::coarse` allocates every level and fills only the first level at or
under 4 MP by point sampling, plus those below it. The levels above it read as
zero, so the coarse state must never be read at those levels:

- `preview_present_level` forces presentation at the coarse level.
- `settle_pyramid` rebuilds synchronously before a stroke preview, a display
  preview, or a pyramid update. It is the only way to write while deferred, so a
  worker's result is valid exactly when its generation is still pending.

Documents at or under 16 MP stay synchronous and exact.

The worker reuses the refcounted level 0 and lands through `CxxQtThread::queue`.
Landing emits `pyramidSettled`, not `regionDamaged`, because the latter marks
crash recovery stale and no document content changed.

## Limits

`rebuild_mask_solo_source` stays synchronous and serial, and runs only while a
mask is soloed. A reorder that recomposited still compares the whole composite
byte for byte, about 48 ms at this size.
