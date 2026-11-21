# Design

## Gallery ranks are the membership switch

`FilterGalleryDialog::categories()` already filters on
`spec.galleryRank >= 0 && filter_kind_supported(spec.kind)` and sorts by rank.
The eight missing Stylize rows only need ranks; their menu paths, labels,
parameters, and build constructors are already in place.

Ranks are a single ascending order across all families, so each family must
occupy a contiguous block. Stylize takes ranks 40 through 48 in CS6 order:

| Rank | Filter |
|---|---|
| 40 | Diffuse |
| 41 | Emboss |
| 42 | Extrude |
| 43 | Find Edges |
| 44 | Glowing Edges |
| 45 | Solarize |
| 46 | Tiles |
| 47 | Trace Contour |
| 48 | Wind |

Texture moves from 41–46 to 49–54 in its existing order (Craquelure, Grain,
Mosaic Tiles, Patchwork, Stained Glass, Texturizer). The exact rank numbers are
internal ordering data; the contract is the category order and membership.

## Preview and thumbnails

The gallery preview and thumbnails use the existing filter stack path, so the
new entries need no preview changes. Diffuse is a whole-layer filter; the proxy
already applies filters to the reduced target layer, so it previews like the
other gallery entries. The thumbnail test runs every Stylize filter over a
textured sample and fails if a thumbnail falls back to the unfiltered sample.

## Tests

- `tst_filter_gallery::categoriesFollowCs6` keeps the category counts and
  asserts the first Stylize entry.
- `tst_filter_gallery::stylizeFiltersFollowCs6` asserts the exact nine kinds in
  order.
- `tst_filter_gallery::thumbnailsRender` renders all nine Stylize thumbnails and
  verifies each differs from the unfiltered sample.

No new dependency and no engine behavior change.
