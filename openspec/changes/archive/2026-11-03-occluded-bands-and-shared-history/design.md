# Design

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

`bucket_fill_profile` (from `paint-bucket-fill-responsiveness`) at
`PICTURA_PROFILE_DIVISOR=4` on the laptop: the region composite 271 ms to 7 ms,
the history capture 84 ms to 15 ms, the undo's history step 81 ms to 28 ms.
