# Design

## Measure first

The baseline was the real app on the real file, sampled with `gdb`. The planned
Rust-side profile test is dropped: it would not have run the Qt controls box,
which is where most of the time goes. The script scenario covers that path.

## O(1) divergence

`Bands` records whether a band has been forked. A band only becomes `Owned` in
`write_band`, and only `build_bands` makes all bands shared again, so the flag
is exact and no scan is needed. Every per-pixel `plane[i]` in the tree benefits.

## Content bounds

`coverage_bounds` and a new `plane_coverage_bounds` share one parallel row scan.
An opaque row stops at its first and last sample, so a full-canvas opaque layer
is O(height). Reading through `Plane::row` avoids a 256 MB materialization after
each stroke.

## Neutral duplicates

An opaque copy directly above its source shows exactly what the source showed,
so the stored composite stays valid. Clipping, knockout and anything that
renders around the layer exclude it. Unknown tagged blocks exclude it too, with
an allowlist of metadata keys, because PSD Backgrounds carry `luni` and `lyid`
while a denylist could miss a block that renders.

The composite is only kept for 8-bit RGB. Other modes and depths recomposite,
which is the previous behavior.
