## MODIFIED Requirements

### Requirement: Character attribute set

The system SHALL model a character attribute set carrying font family, font style, font size, leading (auto or a fixed value), kerning mode (Metrics, Optical, or a manual 1/1000-em value; Optical has no groundable PSD key, so it is saved as Metrics kerning and reopens as Metrics), tracking (1/1000 em), horizontal scale, vertical scale, baseline shift, anti-aliasing method (None, Sharp, Crisp, Strong, Smooth), fill colour, and the All Caps, Small Caps, Superscript, Subscript, Underline, and Strikethrough toggles (All Caps and Small Caps are mutually exclusive, as are Superscript and Subscript; All Caps and Superscript win). It SHALL also carry the layer-wide Fractional Widths setting. A set created for new type SHALL default both scales to 100 percent, kerning to Metrics, anti-aliasing to Sharp, offsets and spacing to 0, the toggles off, and Fractional Widths on.

#### Scenario: Defaults for new type

- **WHEN** a character attribute set is created for new type
- **THEN** horizontal and vertical scale are 100, kerning is Metrics, anti-aliasing is Sharp, tracking and baseline shift are 0, every toggle is off, and Fractional Widths is on

#### Scenario: Every modelled character field round-trips

- **WHEN** each character field is set to a non-default value and read back
- **THEN** the read value equals the value set for that field

#### Scenario: Optical kerning crosses the bridge

- **WHEN** a character setting with kerning mode Optical is converted to the model and back
- **THEN** the model holds Optical kerning and the setting reads back as Optical
