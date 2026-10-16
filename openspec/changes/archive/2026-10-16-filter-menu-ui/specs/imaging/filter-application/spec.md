# Spec Delta

## MODIFIED Requirements

### Requirement: Color-channel extraction and validation

The function SHALL extract the color channels into a planar 8-bit `PixelBuffer` of `rect.width() × rect.height()` with 3 channels. A color layer contributes channels `0`, `1`, and `2`, in that order. A Grayscale layer, which carries only channel `0`, SHALL be treated as three identical planes copied from channel `0`, and the filtered plane MUST be written back to channel `0` alone so no color planes are added. A missing channel `0`, a color channel whose `data.len()` is not `rect.width() * rect.height()`, or a layer that carries exactly one of channels `1`/`2` MUST return `FilterError::InvalidParams` and MUST NOT panic. Filter-level validation errors MUST propagate to the caller unchanged, and no error path may partially write the layer.

#### Scenario: Missing color channel errors

- **WHEN** the layer has no channel with id `1`, but channel `2` is present
- **THEN** `apply_filter` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Wrong channel length errors

- **WHEN** a color channel's `data.len()` is not `rect.width() * rect.height()`
- **THEN** `apply_filter` returns `FilterError::InvalidParams` and the layer is unchanged

#### Scenario: Filter parameter errors propagate

- **WHEN** the filter rejects its own parameters (for example a negative Gaussian radius)
- **THEN** `apply_filter` returns that filter's `FilterError` and the layer is unchanged

#### Scenario: A Grayscale layer filters its single channel

- **WHEN** `apply_filter` is called on a layer that carries channel `0` and no channels `1`/`2`
- **THEN** channel `0` is rewritten in place, no color planes are added, and any transparency channel is filtered per the alpha rule

#### Scenario: A missing channel zero is refused

- **WHEN** the layer carries no channel with id `0`
- **THEN** `apply_filter` returns `FilterError::InvalidParams` and the layer is unchanged

## ADDED Requirements

### Requirement: Region-bounded filter application

`pictura-render` SHALL expose `apply_filter_region`, which applies a filter over a document rectangle clamped to the layer rect and leaves every pixel outside that rectangle unchanged. A neighborhood filter reads up to its support outside the rectangle, where the source is clamped as if the rectangle were the whole layer. `preview_apron` SHALL return a conservative support margin per filter so a viewport preview expanded by it is exact across the visible area.

#### Scenario: Only the region changes

- **WHEN** `apply_filter_region` is called with a region smaller than the layer
- **THEN** pixels outside the region are bit-identical to their input

#### Scenario: The region interior matches a full apply

- **WHEN** the same filter is applied over the whole layer and over a region expanded by `preview_apron`
- **THEN** pixels at least one apron inside the region match the full apply
