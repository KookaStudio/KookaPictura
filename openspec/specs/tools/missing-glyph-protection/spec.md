# tools/missing-glyph-protection Specification

## Purpose
Defines per-character font fallback: when the active type face lacks a glyph and
Missing Glyph Protection is enabled, the character is shaped, laid out, measured,
and rendered in a registered face that covers it.

## Requirements

### Requirement: Uncovered characters use a covering face

While `Enable Missing Glyph Protection` is on, the system SHALL shape and render
a character the active face does not cover with a registered face that does
cover it, keeping the text's character order and the layer's other attributes.
A character the active face covers SHALL keep the active face. The fallback face
SHALL be used for that character's advance, glyph offset, wrapping width, caret
stops, and ink bounds, so measurement and rendering agree.

#### Scenario: A single uncovered character falls back

- **WHEN** a type layer set in Liberation Sans renders U+0370 (Greek Heta) while the protection is on and a registered face covers it
- **THEN** the U+0370 glyph comes from that face and the other characters keep the active face

#### Scenario: A mixed line measures with the fallback advance

- **WHEN** a paragraph line mixes covered and uncovered characters and is wrapped to a box
- **THEN** the line's width, break point, and caret stops use the fallback glyph's advance

#### Scenario: The preference can be off

- **WHEN** the same layer renders while Missing Glyph Protection is off
- **THEN** no fallback face is used and the active face renders every character

### Requirement: Fallback selection is deterministic

The active face SHALL be preferred whenever it covers the character. Otherwise
the system SHALL use the first face, in the active family's ordered fallback
list, that covers it; when no registered face covers it, the active face SHALL
be used. The fallback list SHALL be rebuilt for the active family in a sorted,
deterministic order rather than accumulating document history. Selection SHALL
NOT recurse through a fallback face's own missing glyphs, and the result SHALL
be the same for every render of the same text, active family, and preferences.

#### Scenario: The active face wins when it covers the character

- **WHEN** the active face and a later registered face both cover a character
- **THEN** the glyph comes from the active face

#### Scenario: No covering face keeps the active face

- **WHEN** no registered face covers a character and the protection is on
- **THEN** the active face renders the character and rendering does not fail
