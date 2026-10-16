# selection-model Specification

## Purpose
The selection coverage mask with boolean algebra, invert/none/all, feather, expand/contract, border, and smooth.

## Requirements

### Requirement: Selection coverage representation

A selection SHALL be a document-sized single-channel 8-bit coverage mask with
explicit `width` and `height` and exactly `width * height` bytes of pixel data.
A byte value of 0 MUST mean "fully outside the selection" and 255 MUST mean
"fully inside"; any intermediate value MUST be retained as partial coverage and
MUST NOT be thresholded, quantized to binary, or otherwise discarded by boolean
or modify operations.

#### Scenario: Soft coverage is preserved

- **WHEN** a selection whose pixels include values strictly between 0 and 255 is inspected
- **THEN** each pixel returns its exact stored byte

#### Scenario: Empty and all-pixels selections

- **WHEN** `none(width, height)` and `all(width, height)` are created
- **THEN** the empty selection has every byte equal to 0 and the all-pixels selection has every byte equal to 255, both with the requested dimensions

### Requirement: Boolean combine algebra

The system SHALL combine a selection with another selection under one of four
operations and MUST require matching dimensions. For existing coverage `a` and
incoming coverage `b`, the result MUST be: Replace = `b`; Add = `max(a, b)`;
Subtract = `a - b` clamped at 0 (saturating subtraction); Intersect =
`round(a * b / 255)`.

#### Scenario: Replace copies the incoming selection

- **WHEN** `combine(b, Replace)` runs on any selection `a`
- **THEN** the result coverage equals `b` byte for byte

#### Scenario: Add takes the per-pixel maximum

- **WHEN** `combine(b, Add)` runs
- **THEN** each pixel is `max(a, b)`, so the result is never less than either input

#### Scenario: Subtract removes incoming coverage

- **WHEN** `combine(b, Subtract)` runs
- **THEN** each pixel is `max(a - b, 0)`, and subtracting a selection from itself yields the empty selection

#### Scenario: Intersect scales by coverage

- **WHEN** `combine(b, Intersect)` runs
- **THEN** each pixel is `round(a * b / 255)`, so binary selections intersect to their overlap and intersecting with `all` is the identity

### Requirement: Invert, none and all

The system SHALL provide `invert`, `none`, and `all`. `invert` MUST return a new
selection with every pixel replaced by `255 - v`; it MUST be an involution, and
MUST map `none` to `all` and `all` to `none` at the same dimensions.

#### Scenario: Invert is an involution

- **WHEN** any selection is inverted twice
- **THEN** the result equals the original byte for byte

#### Scenario: Inverting the extremes

- **WHEN** `none(w, h)` is inverted
- **THEN** the result equals `all(w, h)`; inverting `all(w, h)` yields `none(w, h)`

### Requirement: Dimension mismatch is an error

The system MUST return an error, and MUST NOT panic or silently truncate, when
any operation combines or interprets two selections of differing dimensions.
This includes `combine` on selections of different `width`/`height` or coverage
length, `grow`/`similar` against an image of different dimensions, and
`from_channel` against a channel whose length does not match the document.

#### Scenario: Combining mismatched selections errors

- **WHEN** `combine` is called with a selection whose dimensions differ from the receiver
- **THEN** it returns an error and the receiver is unchanged

#### Scenario: Loading a mismatched channel errors

- **WHEN** a channel holds a byte count other than `width * height`
- **THEN** `from_channel` returns an error instead of allocating a wrong-sized selection

### Requirement: Feather

`feather(radius)` SHALL blur the coverage with a separable Gaussian to build a
transition ramp across hard edges. A radius of 0 or less, or a non-finite
radius, MUST return the input unchanged. A positive radius MUST produce
intermediate coverage values at edges and MUST preserve monotonicity of the
ramp. The Gaussian MUST use the inferred mapping `sigma = radius / 2` with
clamped edge sampling. For `sigma >= 2` the implementation SHALL approximate
the Gaussian with three variance-matched box passes per axis, whose widths
`w_i` satisfy `sum (w_i^2 - 1) / 12 = sigma^2`, so a pixel costs `O(1)`
instead of `O(sigma)`; the approximation MUST keep the ramp's monotonicity,
its width, and its 50 % crossing, and MUST stay within 6 coverage levels of
the direct Gaussian on a hard edge whose ramp is clear of the canvas edge.

#### Scenario: Zero radius is identity

- **WHEN** `feather(0.0)` is applied
- **THEN** the returned selection equals the input byte for byte

#### Scenario: A hard edge becomes a ramp

- **WHEN** a hard-edged rectangular selection is feathered with a positive radius
- **THEN** some boundary pixels hold values strictly between 0 and 255 and coverage falls monotonically across the edge

#### Scenario: A wide kernel stays close to the direct Gaussian

- **WHEN** a hard step is feathered with radius 4, 8, 20, 100, or 250
- **THEN** every coverage byte is within 6 levels of the direct separable
  Gaussian with `sigma = radius / 2`, and the 50 % crossing moves by less
  than one pixel
- **AND WHEN** a disk whose ramp is clear of the canvas edge is feathered
  with radius 4, 8, or 20
- **THEN** every coverage byte is within 6 levels of the direct Gaussian

### Requirement: Expand and contract

`expand(radius)` SHALL dilate the coverage and `contract(radius)` SHALL erode
it, using a separable square structuring element of side `2 * radius + 1`. A
radius of 0 MUST be the identity. Sampling MUST clamp at the canvas edge so that
a selection touching the canvas border is not eroded from that border. `expand`
and `contract` by the same radius MUST be inverses on a binary selection clear
of the canvas edge.

#### Scenario: Zero radius is identity

- **WHEN** `expand(0)` or `contract(0)` is applied
- **THEN** the selection is returned unchanged

#### Scenario: Expand then contract restores a binary block

- **WHEN** a binary rectangular selection clear of the canvas edge is expanded and then contracted by the same radius
- **THEN** the result equals the original selection

#### Scenario: Canvas edge is exempt

- **WHEN** a selection touches the canvas border and is contracted
- **THEN** coverage along the canvas edge is not eroded from the edge side

### Requirement: Border

`border(width)` SHALL replace the selection with a band centred on the original
edge, extending approximately half the width outside and half inside. A width of
0 MUST yield the empty selection. The band MUST combine a dilation of the outer
half with an erosion of the inner half so that the deep interior and far
exterior are cleared.

#### Scenario: Border band straddles the edge

- **WHEN** `border(6)` is applied to a rectangular selection
- **THEN** pixels on the edge, a few pixels outside, and a few pixels inside are selected, while the deep interior and far exterior are 0

#### Scenario: Zero-width border is empty

- **WHEN** `border(0)` is applied
- **THEN** the result is the empty selection with the same dimensions

### Requirement: Smooth

`smooth(radius)` SHALL apply a majority/median filter over a square window of
side `2 * radius + 1`: a pixel MUST remain selected when more than half of its
window is selected and MUST be cleared otherwise. A radius of 0 MUST be the
identity. The operation MUST remove isolated selected pixels and fill isolated
holes.

#### Scenario: Isolated pixel is removed

- **WHEN** a single selected pixel surrounded by unselected pixels is smoothed with radius 1
- **THEN** that pixel becomes 0

#### Scenario: Lone hole is filled

- **WHEN** a single unselected pixel inside an otherwise fully selected neighbourhood is smoothed with radius 1
- **THEN** that pixel becomes 255

### Requirement: Determinism and parameter safety

All operations SHALL be deterministic: identical inputs MUST yield identical
outputs, with no dependence on wall-clock time, uninitialized memory, or thread
scheduling. Out-of-range radii MUST be clamped to the documented ceilings
(feather 250, expand/contract 100, border 200, smooth 100) rather than
panicking or overflowing.

#### Scenario: Repeated calls agree

- **WHEN** the same operation runs twice on the same input
- **THEN** the two results are byte-identical

#### Scenario: Huge parameters do not panic

- **WHEN** `expand`, `contract`, `border`, `smooth`, or `feather` are called with very large parameters
- **THEN** they return a same-sized selection instead of panicking

### Requirement: ImageMagick oracle with documented divergence

The change SHALL ship a differential oracle, `scripts/select_oracle.py`, that
applies ImageMagick grayscale morphology and Gaussian blur to a raw 8-bit mask,
and tests that diff `expand`/`contract` against `-morphology Dilate/Erode
Square:r` and `feather` against `-gaussian-blur 0x(radius/2)`. The divergence
between the implemented separable square structuring element and the
spec-proposed Euclidean disk (`Disk:r`, which differs at diagonal corners) MUST
be recorded in the test mapping and README.

#### Scenario: Oracle skips cleanly without ImageMagick

- **WHEN** the oracle tests run with no `magick` binary on `PATH`
- **THEN** the differential rows skip with a message and the ImageMagick-independent property tests still run

#### Scenario: Structuring-element divergence is documented

- **WHEN** the operation-to-ImageMagick mapping is inspected
- **THEN** expand/contract map to `Square:r` and the mapping notes that the Euclidean `Disk:r` diverges at the diagonal corners
