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

## Occluded bands

`composite_banded` asks `first_shown_root` per band. In an 8-bit document with no
knockout, when the bottom shown root is *opaque over the band* (a plain pixel
layer: no effects, clipping, smart object, group or other source; full opacity
and fill, no type curve, Blend If or mask, no native store; covering the band
with every alpha sample 255) and the roots above it are plain, the band starts at
the topmost such root in Normal mode.

Why it is exact: the bottom over the transparent canvas yields alpha
`1 + 0·0 = 1`. Each plain layer over alpha 1 yields `as + 1·(1 − as)`. For
`as ∈ [0.5, 1]`, `1 − as` is exact (Sterbenz); below that it rounds by at most
2⁻²⁵, and `1 + e` with `|e| ≤ 2⁻²⁵` rounds to 1, ties going to the even 1. So the
backdrop alpha is exactly 1. A Normal source of alpha 1 then gives
`(0·cs + 1·cs + 0·c) / 1 = cs` over it and `(1·cs + 0 + 0) / 1 = cs` over a
transparent canvas, with alpha 1 both ways: identical bits, so everything
composited above sees the same canvas. When nothing is shown above the start
root, the band is that layer's bytes and alpha 255, because
`to_u8(k / 255) = k` for every byte (checked exhaustively).

## History sharing

`diff_into` with no hint takes every differing tile, so the current state's plane
ends equal to the live plane byte for byte; it now takes `after.clone()` instead
of forking its bands and copying the tiles in. `swap_tiles` receives the live
document only when `planes_agree` held, so after the swap each touched plane of
the current state equals what the live one must become; the live plane takes a
clone of it instead of a second tile write. Tiles produced by `diff_into` are one
band high and band-aligned, so `swap_plane_tiles` takes the bands with
`write_bands` and swaps them in parallel, falling back to the row path for any
other tile layout. A hinted capture keeps the tile copy, since tiles outside the
hint are not taken.

## Measurement

`bucket_fill_profile` is the click as `fills::imp::apply` runs it, plus the
undo, with `PICTURA_PROFILE_DIVISOR` for machines without the memory for full
size and `PICTURA_PAINT_TIMING` printing the region refresh's phases.
