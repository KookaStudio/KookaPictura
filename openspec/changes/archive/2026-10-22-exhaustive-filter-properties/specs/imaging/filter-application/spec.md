# Spec Delta

## MODIFIED Requirements

### Requirement: Alpha and layer metadata preservation

On a layer that is not transparency-locked, `apply_filter` SHALL filter the transparency channel (`id == -1`) only for a filter whose kernel moves, spreads, or ranks samples independently of their value, or that composites over the picture (Picture Frame), by running that kernel over the alpha plane, so the layer edge moves with the colour or the composited border adds opacity. Smart Blur moves the alpha plane in Normal mode only. Which filters move the alpha plane is decided by the filter type's own alpha fact (see `imaging/filter-engine-facts`) and is not enumerated by this requirement. Every other filter MUST leave the transparency channel bit-identical. Offset without wrap SHALL fill the area it exposes on the transparency channel per its undefined-area mode: fully opaque (255) for CS6's "Set to Background", fully clear (0) for CS6's "Set to Transparent". `apply_filter` MUST NOT change `layer.rect`, `layer.mask`, `layer.opacity`, or `layer.blend`.

#### Scenario: Transparency is bit-identical

- **WHEN** `apply_filter` runs Solarize, Clouds, Difference Clouds, Fibers, Lens Flare, Find Edges, Trace Contour, Tiles, Emboss, High Pass, or Add Noise runs on an unlocked layer that is opaque or partly transparent
- **THEN** the channel with id `-1` equals the input bit for bit

#### Scenario: A moving kernel keeps an opaque layer opaque

- **WHEN** a blur, Median, Mosaic, a geometric Distort filter, or Offset with wrap off and a black Background runs on an unlocked layer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255

#### Scenario: Offset moves the layer edge

- **WHEN** Offset with wrap off shifts a layer whose right half is transparent to the right
- **THEN** the transparent half moves right with the colour and the exposed left columns are opaque

#### Scenario: Offset can expose transparency

- **WHEN** Offset with wrap off runs in "Set to Transparent" mode on an unlocked layer
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

### Requirement: Region-bounded filter application

`pictura-render` SHALL expose `apply_filter_region`, which applies a filter over a document rectangle clamped to the layer rect and leaves every pixel outside that rectangle unchanged. A neighborhood filter reads up to its support outside the rectangle, where the source is clamped as if the rectangle were the whole layer. The filter crate's own apron fact (`Filter::preview_apron()`, see `imaging/filter-engine-facts`) SHALL return a conservative support margin per filter so a viewport preview expanded by it is exact across the visible area.

#### Scenario: Only the region changes

- **WHEN** `apply_filter_region` is called with a region smaller than the layer
- **THEN** pixels outside the region are bit-identical to their input

#### Scenario: The region interior matches a full apply

- **WHEN** the same filter is applied over the whole layer and over a region expanded by the filter's apron
- **THEN** pixels at least one apron inside the region match the full apply
