## MODIFIED Requirements

### Requirement: Offset

The system SHALL implement `Filter::Offset { horizontal: i32, vertical: i32, wrap: bool, repeat: bool, background: [u8; 3], transparent: bool }` as a translation of the colour planes by `(horizontal, vertical)` with clamp-to-edge, wrap, or fill edge handling, and of the transparency plane on a four-channel buffer. A positive `horizontal` SHALL move image content toward increasing x and a positive `vertical` toward increasing y. When `repeat` is true the planes SHALL be read with clamp-to-edge, so every exposed pixel repeats the nearest edge sample; `wrap` SHALL be ignored in that case. When `repeat` is false and `wrap` is true the planes SHALL wrap around (modulo the image dimensions). When both are false the exposed colour area SHALL be filled with `background` and the exposed transparency SHALL be 255 (`transparent == false`, CS6's "Set to Background") or 0 (`transparent == true`, CS6's "Set to Transparent"). `horizontal == 0 && vertical == 0` SHALL be a bit-exact no-op regardless of the edge mode. Oracle expectation: differential against ImageMagick `-roll +h+v` for the `wrap == true` case within tolerance 0. The background-fill and Repeat Edge Pixels cases have no claimed faithful ImageMagick operator, so they are classified as no-equivalent and covered by known-value tests with the divergence documented.

#### Scenario: Wrap re-enters on the opposite side

- **WHEN** Offset is applied with `wrap == true`, `repeat == false`, and `horizontal` equal to the image width
- **THEN** the output equals the input and no content is lost

#### Scenario: Background fills the exposed area

- **WHEN** Offset is applied with `wrap == false`, `repeat == false`, and `transparent == false` to an image whose content is distinct from `background`
- **THEN** every exposed colour pixel equals `background`, the exposed transparency is 255, and the moved content matches the input

#### Scenario: Transparent fills the exposed area clear

- **WHEN** Offset is applied with `wrap == false`, `repeat == false`, and `transparent == true` to a four-channel image
- **THEN** every exposed colour pixel equals `background`, the exposed transparency is 0, and the moved content matches the input

#### Scenario: Zero offset is a no-op

- **WHEN** Offset is applied with `horizontal == 0 && vertical == 0`
- **THEN** the buffer is bit-exactly unchanged regardless of `wrap`, `repeat`, and `transparent`

#### Scenario: Background fill has no faithful equivalent

- **WHEN** the Offset oracle mapping row is checked
- **THEN** the row records `-roll` for the wrap case and its note states that the background-fill and Repeat Edge Pixels cases have no ImageMagick operator (tolerance 0), with a non-empty no-equivalent note

#### Scenario: Repeat edge pixels clamps the exposed edge [of_repeat_edge]

- **WHEN** Offset is applied with `repeat == true` and a positive horizontal shift
- **THEN** every exposed output pixel equals the source sample at the nearest edge, and the shifted interior matches a plain translation

### Requirement: Clamp-to-edge borders and tiny images for Other filters

Every neighborhood Other filter (Maximum, Minimum, High Pass, and Custom) SHALL
sample with clamp-to-edge at the image borders, so an out-of-range index maps
to the nearest edge sample. Offset SHALL read with clamp-to-edge when `repeat`
is true, modulo when `repeat` is false and `wrap` is true, and fill the exposed
area per the Offset requirement when both are false. A 1×1 image and a 1-pixel-wide or
1-pixel-tall image MUST NOT panic and MUST return either a result or a typed
error.

#### Scenario: A 1x1 image does not panic

- **WHEN** every Other variant is applied to a 1×1 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** Maximum, Minimum, High Pass, or Custom is applied to a small buffer and a border pixel is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading out of bounds
