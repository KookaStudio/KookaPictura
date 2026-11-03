# imaging/filter-gallery-preview Specification

## Purpose
Filter Gallery previews a reduced copy of the document for speed.

## Requirements

### Requirement: Filter Gallery previews a reduced copy

The Filter Gallery SHALL render its preview from a copy of the visible part of the document, plus a small margin, box-averaged down to at most one sample per device pixel of the preview pane. The copy is never scaled up. Each change to the stack SHALL re-filter only that copy. Preview cost therefore follows the pane's size and not the document's. While the gallery is open, neither the document nor the canvas SHALL change. OK SHALL apply the visible stack to the full-resolution layer as one "Filter Gallery" history state. Below 100% zoom the filters run at the copy's resolution. Every parameter the descriptor marks as a document-pixel distance SHALL be scaled by the copy's scale before the copy is filtered, rounded on an integer control and held to its range, so the preview's pixel distances match the commit's downsampled geometry. Every other parameter SHALL keep its value. The catalog thumbnails SHALL likewise
scale their opening pixel-distance values by the document-to-thumbnail scale,
rounded on an integer control and held to its range, before filtering the
thumbnail.

#### Scenario: A large picture previews at pane resolution

- **WHEN** the gallery opens on a 4000×3000 document in a pane narrower than 1000 device pixels
- **THEN** the preview image is narrower than 1000 device pixels and keeps the 4:3 aspect

#### Scenario: Previewing leaves the document alone

- **WHEN** the gallery previews a visible effect and is cancelled
- **THEN** the canvas is pixel-identical to before it opened and the history is unchanged

#### Scenario: Full-scale copy matches the commit

- **WHEN** the copy is taken at scale 1 over the whole document and filtered
- **THEN** it equals the composite of the document with the same filter applied to the layer

#### Scenario: Pixel distances scale with the copy [fgp_pixel_scale]

- **WHEN** a gallery stack contains a filter with a pixel-distance parameter (for example Mosaic Tiles Tile Size 12) and is previewed on a copy at a quarter of the document's size
- **THEN** every pixel-distance value is scaled by 0.25, while a non-distance control such as Seed keeps its value

#### Scenario: Catalog thumbnails scale their opening distances [fgp_thumbnail_scale]

- **WHEN** the gallery renders the Mosaic Tiles catalog thumbnail for a 3000×2400 document
- **THEN** the default Tile Size and Grout Width are scaled by the document-to-thumbnail scale, rounded and held to their ranges, before the thumbnail is filtered

### Requirement: Reduced copies scale vector masks

A reduced copy of the document SHALL map each vector-mask path through the copy's region and scale, and SHALL scale the mask's feather radius by the copy's scale, so a mask covers the same part of the picture in the copy as in the document.

#### Scenario: A reduced vector mask keeps its place [proxy_vector_mask]

- **WHEN** a layer with a vector mask is reduced, whole or as a cropped region
- **THEN** the mask's edge falls at its scaled position relative to the region

#### Scenario: A reduced vector mask scales its feather [proxy_vector_feather]

- **WHEN** a layer with a feathered vector mask is reduced
- **THEN** the feather radius is multiplied by the copy's scale

#### Scenario: A full-scale copy with a vector mask matches the document [proxy_full_scale]

- **WHEN** a Multiply layer with a vector mask is copied at scale 1 with no filter
- **THEN** the copy's composite equals the document's
