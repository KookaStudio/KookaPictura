# filter-application Specification

## Purpose
The destructive filter entry point: colour-channel extraction, mask-confined writes, and alpha-preserving output.

## Requirements

### Requirement: Destructive filter application entry point

`pictura-render` SHALL expose `pub fn apply_filter(layer: &mut pictura_core::Layer, filter: &pictura_filters::Filter, mask: Option<&pictura_core::LayerMask>) -> Result<(), pictura_filters::FilterError>`. It SHALL apply `filter` destructively to the layer's color channels and gate the write by the optional document-coordinate `mask`. When `layer.rect.width()` or `layer.rect.height()` is `<= 0` it MUST return `Ok(())` without modifying the layer. When the layer's lock state includes the pixel lock, it MUST return a refusal error without modifying the layer.

#### Scenario: A non-empty layer is filtered in place

- **WHEN** `apply_filter` is called on a layer with a non-empty `rect` and a valid filter
- **THEN** the layer's color channels are rewritten in place and `Ok(())` is returned

#### Scenario: A degenerate rect is a no-op

- **WHEN** `apply_filter` is called on a layer whose `rect` has zero or negative width or height
- **THEN** it returns `Ok(())` and the layer is bit-identical to its input

#### Scenario: A pixel-locked layer is refused [lfa_pixel_locked]

- **WHEN** `apply_filter` is called on a layer whose lock state includes the pixel lock
- **THEN** it returns a refusal error and the layer's channels are bit-identical to their input

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

### Requirement: Mask-confined write

For each local pixel `(lx, ly)`, the coverage SHALL be the mask value at document coordinates `(rect.left + lx, rect.top + ly)`, or `mask.default_color` when that point lies outside the mask rect; when `mask` is `None` the coverage SHALL be `255`. The function SHALL write `round(orig + (filtered - orig) * coverage / 255)` into color channels `0`, `1`, and `2`, where `orig` is the pre-filter sample and `filtered` the filter's output. Coverage `255` MUST reproduce the filter output, coverage `0` MUST leave the pixel unchanged, and intermediate coverage MUST interpolate.

#### Scenario: No mask filters the full frame

- **WHEN** `apply_filter` is called with `mask` `None`
- **THEN** every pixel receives the filter output as if coverage were 255

#### Scenario: Full coverage equals the filter output

- **WHEN** every pixel under the layer rect has mask coverage 255
- **THEN** the written pixels equal the filter output

#### Scenario: Zero coverage leaves the pixel unchanged

- **WHEN** a pixel under the layer rect has mask coverage 0
- **THEN** that pixel is bit-identical to its pre-filter value

#### Scenario: Partial coverage interpolates

- **WHEN** a pixel has mask coverage between 1 and 254
- **THEN** the written sample is the rounded linear interpolation of `orig` and `filtered` by `coverage / 255`

#### Scenario: Outside the mask rect uses the default color

- **WHEN** the layer rect extends beyond the mask rect
- **THEN** pixels outside the mask rect use `mask.default_color` as their coverage

### Requirement: Alpha and layer metadata preservation

On a layer that is not transparency-locked, `apply_filter` SHALL filter the transparency channel (`id == -1`) only for a filter whose kernel moves, spreads, or ranks samples independently of their value, or that composites over the picture (Picture Frame), by running that kernel over the alpha plane, so the layer edge moves with the colour or the composited border adds opacity. Smart Blur moves the alpha plane in Normal mode only. Which filters move the alpha plane is decided by the filter type's own alpha fact (see `imaging/filter-engine-facts`) and is not enumerated by this requirement. Every other filter MUST leave the transparency channel bit-identical. Offset with `repeat` false and `wrap` false SHALL fill the area it exposes on the transparency channel per its undefined-area mode: fully opaque (255) for CS6's "Set to Background", fully clear (0) for CS6's "Set to Transparent". Offset with `repeat` true SHALL translate the transparency plane with clamp-to-edge, so the exposed edge repeats the nearest edge alpha. `apply_filter` MUST NOT change `layer.rect`, `layer.mask`, `layer.opacity`, or `layer.blend`.

#### Scenario: Transparency is bit-identical

- **WHEN** `apply_filter` applies Solarize, Clouds, Difference Clouds, Fibers, Lens Flare, Find Edges, Trace Contour, Tiles, Emboss, High Pass, or Add Noise to an unlocked layer that is opaque or partly transparent
- **THEN** the channel with id `-1` equals the input bit for bit

#### Scenario: A moving kernel keeps an opaque layer opaque

- **WHEN** a blur, Median, Mosaic, a geometric Distort filter, or Offset with repeat off, wrap off, and a black Background runs on an unlocked layer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255

#### Scenario: Offset moves the layer edge

- **WHEN** Offset with repeat off and wrap off shifts a layer whose right half is transparent to the right
- **THEN** the transparent half moves right with the colour and the exposed left columns are opaque

#### Scenario: Offset can expose transparency

- **WHEN** Offset with repeat off and wrap off runs in "Set to Transparent" mode on an unlocked layer
- **THEN** the exposed area's transparency channel is 0 while the colour planes take the dialog's Background colour

#### Scenario: Every moving kernel moves a soft layer edge

- **WHEN** any filter whose alpha fact is that its kernel moves runs on an unlocked layer whose alpha is a soft-edged disc with a stray speck
- **THEN** the channel with id `-1` differs from the input

#### Scenario: Picture Frame may add opacity

- **WHEN** Picture Frame runs on an unlocked layer that is partly transparent
- **THEN** the transparency channel gains the composited border's opacity where the frame covers a clear pixel

#### Scenario: Layer metadata is unchanged

- **WHEN** `apply_filter` returns success
- **THEN** `rect`, `mask`, `opacity`, and `blend` equal their pre-call values

#### Scenario: Offset repeats the edge alpha [fa_offset_repeat_alpha]

- **WHEN** Offset runs in Repeat Edge Pixels mode on an unlocked layer whose alpha has a soft edge
- **THEN** the translated alpha plane repeats the nearest edge alpha where the translation exposes pixels, rather than filling the exposed area opaque or clear

### Requirement: Errors instead of panics on bad layer data

`apply_filter` MUST return a `pictura_filters::FilterError` — never panic — for any malformed layer color data, and MUST leave the layer unchanged on every error path.

#### Scenario: Malformed layer data returns an error without panicking

- **WHEN** `apply_filter` receives a layer with a missing or wrong-length color channel
- **THEN** it returns a `FilterError` and does not panic

#### Scenario: The layer is unchanged after an error

- **WHEN** `apply_filter` returns an error
- **THEN** every channel of the layer is bit-identical to its input

### Requirement: Filter application targets the active layer

The application-level filter entry point SHALL apply a filter to the active
layer, where the active layer is the exactly-one layer resolved by the shared
active-layer resolver, rather than defaulting to the topmost raster layer. When
no layer is active or more than one layer is selected, the application SHALL
refuse the filter and leave the document unchanged with no history state. The
refusal SHALL be surfaced to the user.

#### Scenario: A filter applies to the active layer [lfa_active_layer]

- **WHEN** a filter is applied with exactly one layer active
- **THEN** that layer is filtered and the other layers are unchanged

#### Scenario: No single active layer refuses [lfa_no_active_layer]

- **WHEN** a filter is applied with no layer active or with more than one layer
  selected
- **THEN** the filter is refused, no layer is mutated, and the document is
  unchanged

### Requirement: Filter respects the transparency lock per pixel

When the active layer's lock state includes `TRANSPARENCY`, a filter SHALL change
only the color channels of pixels whose pre-existing alpha is greater than zero
and SHALL write each such pixel's pre-existing alpha back unchanged; a pixel whose
alpha is zero SHALL be left untouched. The filter SHALL NOT lower any pixel's
alpha, and applying it SHALL leave every pixel's alpha value equal to its
pre-filter value.

#### Scenario: Filtering a transparency-locked layer keeps alpha [lfa_transparency_alpha]

- **WHEN** a filter runs on a transparency-locked layer
- **THEN** the color channels change where alpha is greater than zero and every
  pixel's alpha equals its pre-filter value

#### Scenario: Fully transparent pixels are untouched [lfa_transparency_clear_pixel]

- **WHEN** a filter runs on a transparency-locked layer at a pixel whose alpha is
  zero
- **THEN** that pixel's color and alpha are unchanged

### Requirement: Region-bounded filter application

`pictura-render` SHALL expose `apply_filter_region`, which applies a filter over a document rectangle clamped to the layer rect and leaves every pixel outside that rectangle unchanged. A neighborhood filter reads up to its support outside the rectangle, where the source is clamped as if the rectangle were the whole layer. The filter crate's own apron fact (`Filter::preview_apron()`, see `imaging/filter-engine-facts`) SHALL return a conservative support margin per filter so a viewport preview expanded by it is exact across the visible area.

#### Scenario: Only the region changes

- **WHEN** `apply_filter_region` is called with a region smaller than the layer
- **THEN** pixels outside the region are bit-identical to their input

#### Scenario: The region interior matches a full apply

- **WHEN** the same filter is applied over the whole layer and over a region expanded by the filter's apron
- **THEN** pixels at least one apron inside the region match the full apply

### Requirement: Ported photorust filter engine

The system SHALL run the Artistic, Brush Strokes, Sketch, and Texture filters, and the Add Noise, Dust & Scratches, Average, Emboss, Find Edges, Diffuse, Glowing Edges, Extrude, Tiles, Trace Contour, Wind, Mosaic, Crystallize, Pointillize, Facet, Fragment, Mezzotint, Color Halftone, Twirl, Pinch, Spherize, Ripple, Wave, Polar Coordinates, and ZigZag filters through the engine ported from photorust (GPL-3.0). `apply` SHALL validate the buffer and each parameter against its capability's ranges before converting to the engine's interleaved image, SHALL write back only the colour planes so alpha is never modified, and SHALL fold each variant's `seed` into the engine's noise so the same seed reproduces a result bit for bit and different seeds re-roll it. The engine SHALL be deterministic regardless of thread count. Solarize SHALL stay on its ImageMagick-exact implementation.

#### Scenario: Ported filters keep the shared contract

- **WHEN** each ported filter is applied to RGB and RGBA buffers, twice, with an out-of-range parameter, and to 1×1, 1×N, and N×1 buffers
- **THEN** it succeeds with alpha unchanged, repeats bit for bit, rejects the bad parameter without writing a sample, and never panics

#### Scenario: Seeds re-roll the stochastic filters

- **WHEN** a ported filter that takes a `seed` is applied with two different seeds
- **THEN** the two outputs differ
