# raster-export delta

## ADDED Requirements

### Requirement: Export As WebP quality spans 0 through 100

The Export As Quality control SHALL accept 0 through 100 when WebP is lossy, so
the value matches the engine encoder and the control surface.

#### Scenario: Quality 0 is selectable [webp_quality_range]

- **WHEN** the Format is WebP and Lossless is unchecked
- **THEN** the Quality control accepts 0 and 100, and both values encode a VP8
  stream
