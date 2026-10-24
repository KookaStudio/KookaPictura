# Spec Delta

## MODIFIED Requirements

### Requirement: Character attribute set

The system SHALL model a character attribute set carrying font family, font style, font size, the spelling language by its CS6 display name (default `English: USA`), leading (auto or a fixed value), kerning mode (Metrics, Optical, or a manual 1/1000-em value; Optical has no groundable PSD key, so it is saved as Metrics kerning and reopens as Metrics), tracking (1/1000 em), horizontal scale, vertical scale, baseline shift, tsume (0–100, default 0), anti-aliasing method (None, Sharp, Crisp, Strong, Smooth), fill colour, and the All Caps, Small Caps, Superscript, Subscript, Underline, and Strikethrough toggles (All Caps and Small Caps are mutually exclusive, as are Superscript and Subscript; All Caps and Superscript win). It SHALL also carry the layer-wide Fractional Widths setting. A set created for new type SHALL default both scales to 100 percent, kerning to Metrics, anti-aliasing to Sharp, offsets and spacing to 0, language to `English: USA`, tsume to 0, the toggles off, and Fractional Widths on.

#### Scenario: Defaults for new type

- **WHEN** a character attribute set is created for new type
- **THEN** horizontal and vertical scale are 100, kerning is Metrics, anti-aliasing is Sharp, tracking and baseline shift are 0, language is `English: USA`, tsume is 0, every toggle is off, and Fractional Widths is on

#### Scenario: Every modelled character field round-trips

- **WHEN** each character field, including language and tsume, is set to a non-default value and read back
- **THEN** the read value equals the value set for that field

#### Scenario: Optical kerning crosses the bridge

- **WHEN** a character setting with kerning mode Optical is converted to the model and back
- **THEN** the model holds Optical kerning and the setting reads back as Optical

### Requirement: Paragraph attribute set

The system SHALL model a paragraph attribute set carrying alignment (horizontal Left, Center, Right; vertical Top, Center, Bottom), justification (Justify Last Left, Justify Last Center, Justify Last Right, Justify All), minimum/desired/maximum word spacing, letter spacing, and glyph scaling, left, right, and first-line indents, space before and after, hanging punctuation, hyphenation (default off), paragraph direction (LTR or RTL, default LTR), and composer (single-line or every-line). A set created for new type SHALL default the indents and paragraph spacing to 0, desired word spacing to `[0.8 1.0 1.33]` and glyph scaling to `[1.0 1.0 1.0]`, desired letter spacing to `[0.0 0.0 0.0]`, hanging punctuation off, hyphenation off, direction LTR, and the single-line composer.

#### Scenario: Defaults for new type

- **WHEN** a paragraph attribute set is created for new type
- **THEN** alignment is Left, indents and space before/after are 0, hanging punctuation and hyphenation are off, direction is LTR, the composer is single-line, and word spacing is the CS6 justified default `[0.8 1.0 1.33]`

#### Scenario: Every modelled paragraph field round-trips

- **WHEN** each paragraph field, including direction, is set to a non-default value and read back
- **THEN** the read value equals the value set for that field
