# Spec Delta

## Purpose

Offset's undefined area gains CS6's second fill mode alongside the colour
Background, and the fold of alpha into the kernel's working buffer keeps the
observable contract unchanged for the translation itself.

## MODIFIED Requirements

### Requirement: Offset

The system SHALL implement `Filter::Offset { horizontal: i32, vertical: i32, wrap: bool, background: [u8; 3], transparent: bool }` as a translation of the colour planes by `(horizontal, vertical)` with clamp-to-edge or wrap edge handling, and of the transparency plane on a four-channel buffer. A positive `horizontal` SHALL move image content toward increasing x and a positive `vertical` toward increasing y. When `wrap` is true the planes SHALL wrap around (modulo the image dimensions); when `wrap` is false the exposed colour area SHALL be filled with `background` and the exposed transparency SHALL be 255 (`transparent == false`, CS6's "Set to Background") or 0 (`transparent == true`, CS6's "Set to Transparent"). `horizontal == 0 && vertical == 0` SHALL be a bit-exact no-op. Oracle expectation: differential against ImageMagick `-roll +h+v` for the `wrap == true` case within tolerance 0. The `wrap == false` background fill has no faithful ImageMagick operator, so it is classified as no-equivalent and covered by known-value tests with the divergence documented.

#### Scenario: Wrap re-enters on the opposite side

- **WHEN** Offset is applied with `wrap == true` and `horizontal` equal to the image width
- **THEN** the output equals the input and no content is lost

#### Scenario: Background fills the exposed area

- **WHEN** Offset is applied with `wrap == false` and `transparent == false` to an image whose content is distinct from `background`
- **THEN** every exposed colour pixel equals `background`, the exposed transparency is 255, and the moved content matches the input

#### Scenario: Transparent fills the exposed area clear

- **WHEN** Offset is applied with `wrap == false` and `transparent == true` to a four-channel image
- **THEN** every exposed colour pixel equals `background`, the exposed transparency is 0, and the moved content matches the input

#### Scenario: Zero offset is a no-op

- **WHEN** Offset is applied with `horizontal == 0 && vertical == 0`
- **THEN** the buffer is bit-exactly unchanged regardless of `wrap` and `transparent`

#### Scenario: Background fill has no faithful equivalent

- **WHEN** the oracle mapping table is checked
- **THEN** the Offset background-fill case has no ImageMagick operator, tolerance 0, and a non-empty no-equivalent note