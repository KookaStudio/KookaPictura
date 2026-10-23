## MODIFIED Requirements

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
