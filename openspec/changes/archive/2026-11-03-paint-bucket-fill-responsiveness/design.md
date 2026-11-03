# Design

## Flood

`bucket::flood` builds the match mask with `par_iter` over the surface. The
non-contiguous answer is that mask. For a contiguous flood a mask with no zero
means every pixel matches, so all are 4-connected to the seed and the mask is
returned; otherwise `mask_ops::flood` runs with a lookup into the mask as its
predicate. The predicate only ever tested the destination pixel against the
seed's colour, so the result is the same.

## Anti-alias

`antialias_mask` splits the output into rows with `par_chunks_mut`. A row whose
3×3 neighbourhood is all 255 or all 0 is already its own answer; otherwise
per-column sums over the neighbourhood rows give each pixel's box sum, with the
same integer division and 128 floor as before.

## Fill

`fill_planes` keeps the last `(backdrop, colour, strength bits)` and its blend
per row task. The brush blends are pure in those inputs except for Dissolve's
`roll`, so Dissolve keeps calling the blend with the pixel noise.

## Measurement

`bucket_fill_profile` is the click as `fills::imp::apply` runs it, plus the
undo, with `PICTURA_PROFILE_DIVISOR` for machines without the memory for full
size and `PICTURA_PAINT_TIMING` printing the region refresh's phases.
