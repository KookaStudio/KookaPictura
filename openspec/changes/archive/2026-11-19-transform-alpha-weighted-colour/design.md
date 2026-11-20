# Design

## D1. Weighting in the kernel, not premultiplied planes

Premultiplying the 8-bit planes first and dividing afterwards would quantise
the colour of low-alpha pixels to a few levels. The kernel instead reads the
colour and alpha windows together and divides in `f64`, as the warp engine
does.

## D2. Opaque taps keep the existing value

When every tap with a non-zero weight is fully opaque, the sampler returns the
plain kernel's value, computed in the same order as before. Opaque layers,
integer-offset moves, and the tiled-versus-dense oracle tests therefore stay
byte-identical. A transparent result (`Σ wᵢ·aᵢ ≤ 0`) stores colour 0, the
value an out-of-source sample already has. The weighted colour is clamped to
the unit range, so a cubic's negative lobes over a near-transparent footprint
cannot produce an extreme colour.

## D3. Scope

Only planes with id `≥ 0` (colour) are weighted, by the layer's `-1` plane of
the same depth. A layer without an alpha plane resamples as before. Nearest
returns the source pixel and needs no weighting.

## D4. A stale store is refreshed, not trusted

A brush or eraser stroke edits only the 8-bit planes and leaves the retained
store behind; the PSD writer already uses a store plane only while it narrows
to the current 8-bit plane. The transform resampled the store as-is and then
narrowed it back over the 8-bit channels, which restored erased pixels (#1446).
`prepare_layer` now applies the writer's rule per plane: a stale plane is
replaced by the current 8-bit plane widened to the store depth, so the planes
the stroke did not touch keep their native precision and the store stays
complete for the native brushes. Free Transform and Warp share
`prepare_layer`, so both are covered. A converted source mode's store is
excluded, as `write_layer` never narrows it back.

## D5. Auto-Align reads premultiplied luma

The Perspective auto-align test transforms its moving layer onto a trapezoid.
Its edge pixels used to darken towards black, which happened to give the
matcher a smooth luma ramp; with the colour kept, straight luma steps from the
full colour to zero along a stair-stepped slanted edge, the Harris detector
fires along it, and RANSAC found no homography. The work image now weights luma
by coverage, which is what the layer looks like when rendered and makes the
matcher independent of the colour stored under partial alpha.
