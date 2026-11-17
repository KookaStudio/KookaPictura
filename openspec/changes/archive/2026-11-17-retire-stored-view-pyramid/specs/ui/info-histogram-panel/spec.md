## REMOVED Requirements

### Requirement: Panels do not re-composite per refresh

**Reason**: The former cost claim does not hold for thin images or tile generation.

**Migration**: Use Tile-based panel sampling and bounded histogram binning.

## ADDED Requirements

### Requirement: Tile-based panel sampling and bounded histogram binning

The Info panel's pixel sampling SHALL read the requested current composite
sample through a region or cached tile rather than building a full-resolution
image or requiring a stored planar composite. The Histogram SHALL fetch the
coarsest tile-pyramid level and, when either dimension exceeds 512, scale the
fetched image to fit within 512×512 before binning. The source level itself
need not fit that bound: a 4096×2 document has only level 0 and fetches that
level before scaling. Binning is bounded by 512×512 pixels; source fetching and
tile generation can depend on document dimensions and cache state. Histogram
refresh MUST NOT call `PictureView::image()` or force a full-resolution image
rebuild. Both panels SHALL retain their current visual results.

#### Scenario: Sampling a large document performs no full composite

- **WHEN** a pixel is sampled in the Info panel on a 4000×4000 document whose display tiles are already current
- **THEN** the sample is read through the current composite region or tile and no full-document composite is run

#### Scenario: Histogram bins a bounded image

- **WHEN** the Histogram panel refreshes on a 4000×4000 document
- **THEN** it bins an image of at most 512×512 and retains the current plotted distribution

#### Scenario: Histogram refresh uses a tile level

- **WHEN** the Histogram panel refreshes after a region refresh
- **THEN** it reads the coarsest tile-pyramid level without calling `PictureView::image()` or rebuilding the full-resolution image

#### Scenario: A thin document is scaled after fetching level 0

- **WHEN** the Histogram panel refreshes on a 4096×2 document
- **THEN** it fetches the 4096×2 level 0 and scales it to fit within 512×512 before binning

#### Scenario: Histogram binning is bounded for larger documents

- **WHEN** the Histogram panel refreshes on a document at least twice as large in each dimension
- **THEN** the image it bins is at most 512×512, regardless of the dimensions of the fetched source level
