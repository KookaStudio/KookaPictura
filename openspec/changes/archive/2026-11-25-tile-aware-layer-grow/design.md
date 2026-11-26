# Design

## Context

Layer planes and the stroke's masks are tiled (256² tiles that are `Empty`,
`Uniform` or a shared `Arc` body, #1213). `padded_plane` ignored that: it read
the old plane in strips and wrote every strip into a fresh plane. Its cost was
proportional to the old layer's area, and a long stroke grows the layer dozens
of times.

## Decisions

- **Per tile, not per row.** `TiledPlane::padded` visits the old tiles and
  skips `Empty` ones. When the offset is a whole number of tiles and the
  destination tile has the same clipped size, it clones the tile (an `Arc`
  bump, or a copied `Uniform` value) together with its generation. Otherwise
  it copies that tile's samples into the new plane with `write_rect_from`, and
  a settle folds uniform results.
- **Generations travel with shared tiles.** Generations are compared only
  between planes of the same geometry (`shares_tile`). Grown planes never match
  their old geometry, so carrying a generation over cannot make two different
  tiles look equal.
- **Sharing is copy-on-write.** History may still hold the pre-grow planes. A
  later write forks the tile when its body is not uniquely held, as it already
  does for any shared tile.
- **Tight rect kept.** Growing ahead with slack and trimming at finish was
  rejected: each grow and the trim would still copy the whole layer, and the
  tile-aware grow makes a right or down grow almost free.
- **The governor reservation is unchanged.** `grow_layer_to` still reserves the
  old layer's area per plane. Now that a shared tile allocates nothing, this
  over-reserves. It stays conservative.

## Risks

- A grow to the left or up by a non-tile offset still copies every non-empty
  tile. Its cost is the painted area, not the layer area.
