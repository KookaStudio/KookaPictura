# Spec Delta

## ADDED Requirements

### Requirement: Paragraph direction orders and shapes runs

A line SHALL be shaped and ordered from its paragraph's direction. A
left-to-right line SHALL keep the current left-to-right order. A right-to-left
line SHALL place its first character at the line's right edge and shape its runs
in right-to-left order. A line that mixes directionality SHALL be split into
directional runs ordered by the Unicode Bidirectional Algorithm, and each run
SHALL be shaped with its own direction. The line's glyph order, advances, and
offsets SHALL be in visual order, and its width SHALL be the sum of its runs.

#### Scenario: An RTL line reverses its visual order

- **WHEN** the right-to-left text `אבג` is shaped in a right-to-left paragraph
- **THEN** the glyphs render in the order `גבא` from left to right and the line's width is unchanged

#### Scenario: Mixed directions keep their runs intact

- **WHEN** a right-to-left paragraph contains a left-to-right Latin word inside right-to-left text
- **THEN** the Latin word's glyphs stay in left-to-right order while the surrounding text keeps right-to-left order

#### Scenario: An LTR line is unchanged

- **WHEN** the same text is shaped in a left-to-right paragraph
- **THEN** its glyph order and advances equal the pre-direction shaping
