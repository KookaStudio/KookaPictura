# Design

## Measurements

`structural_restore_profile`, 16507×16196, 8 cores, same test build with each
change reverted for the before column:

| Measurement | Before | After |
|---|---|---|
| Undo of a resize: full-size level 0 install | 217 ms | 0 ms (shares the composite) |
| Undo of a resize: deferred display | — | 5.7 ms |
| Identical reorder: undo history | 50.6 ms | 0.2 ms |
| Identical reorder: capture | 80 ms | 135 ms |

The reorder's composite scan moves from the undo to the edit that already
recomposited the whole document; the undo across that boundary then reports an
empty rectangle without reading a composite pixel. `#1098` measured the scan at
42 ms on its reference machine.

`solo_mask_profile`, 8000² (the 16507² run swaps on a 7 GiB machine):

| Solo mask plane | Time |
|---|---|
| One core | 694 ms |
| All cores | 151 ms |

At 16507² the plane is about 630 ms parallel and the exact pyramid about
300 ms, so a soloed-mask restore still holds the GUI thread for about a second;
before this change the plane alone was several times that.


## Banded level-0 source

`Planes` gains a second backing: a planar `Plane<u8>` read through `Plane::row`,
which serves a flat plane's row from the base and a banded plane's row from its
band without materializing anything. `Planes::rows(y)` returns the four channel
rows of one line; the pyramid's four level-0 readers (`sample_from_planes`,
`shrink_from_planes`, the level-0 branch of `crop`, and the premultiply helper)
fetch rows per output row instead of indexing four slices per pixel. The
arithmetic is unchanged, so every level stays byte-identical.

`planes_of` builds the planar backing from the buffer's `data` plane, so the
app's level 0, the mask level 0 and any other RGBA `PixelBuffer` reach the
pyramid without a contiguous copy.

## Shared level 0

`level0_from_buffer` no longer copies a frame that shares its source. It
fractures the frame instead, and `level0_buffer` fractures the document
composite first, so the first write to either forks one 64-row band instead of
copying the full frame. The frame still owns the converted plane when the
document needs a display conversion.

The whole-frame branch of `refresh_level0_region` follows the same rule: it
fractures the frame and the document composite instead of copying.

## Unchanged composite at a segment break

`diff_into` returns early when tracked planes moved (a reorder, a layer
insertion or deletion, a geometry change), so a capture never compared the
composite. `capture_live_with` now calls `composite_unchanged` in that branch
unless the document geometry or profile changed: it fractures both composites
and compares their bands across all cores, stopping at the first difference.
When they are equal the live document takes the current state's plane, which
also drops one full composite from the history's retained bytes.

The undo across that boundary then finds the two composites sharing every band,
so `composite_change` skips the whole scan and reports an empty rectangle. The
comparison moves to the edit that already recomposited the whole document.

## Solo mask source

`mask_display_plane` fills its four planes as parallel row work: the alpha plane
is a constant fill and each colour plane is sampled per row on every core. The
plane is allocated with `PixelBuffer::zeroed`, so the fill touches only the
pages it writes. The pyramid built from it was already parallel.

## Limits

The comparison still costs a full pass over the composite when the picture did
change at a segment break; it runs on the edit, not the undo. A soloed mask
still builds its exact pyramid synchronously, so a full-size soloed restore
holds the GUI thread for the parallel plane fill plus that rebuild; deferring
the mask pyramid the way the composite pyramid is deferred is a follow-up.
