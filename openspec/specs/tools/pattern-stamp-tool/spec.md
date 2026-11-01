# tools/pattern-stamp-tool Specification

## Purpose
The Pattern Stamp tool: clone from a built-in pattern.

## Requirements

### Requirement: Built-in patterns

`pictura_paint::pattern` SHALL provide eight named 64 × 64 opaque greyscale
tiles, each with visible contrast and no seam where opposite edges meet, and
SHALL return none past the end of the list. `pictura_paint::stamp::tiled`
SHALL repeat a tile over a surface with the tile's top-left corner on a given
origin, wrapping points left of or above the origin.

#### Scenario: Every tile is seamless and textured

- **WHEN** each built-in pattern is rendered
- **THEN** it is 64 × 64, fully opaque, spans more than 30 grey levels, and its edges continue into each other

#### Scenario: A negative origin wraps

- **WHEN** a tile is repeated from origin (−3, −5)
- **THEN** the surface's first pixel is the tile's pixel (3, 5)

### Requirement: Pattern Stamp tool

The Pattern Stamp SHALL paint the chosen built-in pattern through the source
stroke and record exactly one "Pattern Stamp" history state per stroke that
changed pixels. With Aligned on (the default) the tile SHALL be pinned to the
document origin; with it off, to the point where each stroke starts. The
options bar SHALL offer Size, Hardness, Mode, Opacity, Flow, a pattern picker
listing the built-in patterns with swatches, Aligned, and Impressionist
(disabled).

#### Scenario: An aligned stroke paints the document-pinned tile

- **WHEN** the `pattern_stamp_tool` self-test strokes the Checkerboard across x = 32 on white
- **THEN** one "Pattern Stamp" state is recorded, light grey (215) left of x = 32 and dark grey (90) right of it

#### Scenario: Unaligned pins the tile to the stroke

- **WHEN** an unaligned click at (20, 50) paints the Checkerboard
- **THEN** the pixel there is the tile's corner colour (215), where the document-aligned tile is dark

### Requirement: Pattern Stamp Airbrush

The Pattern Stamp options bar SHALL carry an Airbrush toggle, off by default and persisted with
the tool's options. While a stroke is open with Airbrush on, the tool SHALL keep
laying dabs at the pointer at a bounded time-driven rate even when the pointer is
still, accumulating toward the Opacity cap. With Airbrush off, a still pointer SHALL
add no paint. Held dabs SHALL stop when the stroke ends, whether by release, tool
switch, or session cancel, and SHALL NOT add a second history state.

#### Scenario: Airbrush off adds nothing while held [pattern_hold_airbrush_off]

- **WHEN** a Pattern Stamp stroke with Airbrush off is held still at low Flow
- **THEN** the pixels under the pointer do not change

#### Scenario: Airbrush on builds while held [pattern_hold_airbrush_on]

- **WHEN** a Pattern Stamp stroke with Airbrush on is held still at low Flow
- **THEN** the pixels under the pointer move toward the source and release records one history state

#### Scenario: Toggle sets and persists the option [pattern_hold_airbrush_toggle]

- **WHEN** the Pattern Stamp Airbrush toggle is clicked
- **THEN** the tool's option is on and survives the options JSON round trip
