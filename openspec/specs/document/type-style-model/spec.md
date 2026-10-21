# document/type-style-model Specification

## Purpose
Defines the character and paragraph attribute sets, the named character and paragraph styles, the hierarchy that resolves the effective formatting of a run or paragraph, and the app edit that applies them under one history state, so type panels and the text engine share one model.

## Requirements

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

### Requirement: Paragraph attribute set

The system SHALL model a paragraph attribute set carrying alignment (horizontal Left, Center, Right; vertical Top, Center, Bottom), justification (Justify Last Left, Justify Last Center, Justify Last Right, Justify All), minimum/desired/maximum word spacing, letter spacing, and glyph scaling, left, right, and first-line indents, space before and after, hanging punctuation, hyphenation (default off), and composer (single-line or every-line). A set created for new type SHALL default the indents and paragraph spacing to 0, desired word spacing to `[0.8 1.0 1.33]` and glyph scaling to `[1.0 1.0 1.0]`, desired letter spacing to `[0.0 0.0 0.0]`, hanging punctuation off, hyphenation off, and the single-line composer.

#### Scenario: Defaults for new type

- **WHEN** a paragraph attribute set is created for new type
- **THEN** alignment is Left, indents and space before/after are 0, hanging punctuation and hyphenation are off, the composer is single-line, and word spacing is the CS6 justified default `[0.8 1.0 1.33]`

#### Scenario: Every modelled paragraph field round-trips

- **WHEN** each paragraph field is set to a non-default value and read back
- **THEN** the read value equals the value set for that field

### Requirement: Character and paragraph styles with hierarchy

The system SHALL model named character styles (a set of character attributes) and named paragraph styles (a set of character and paragraph attributes) within a `TextStyleSheet`, and SHALL resolve an effective attribute for a run or paragraph by CS6 precedence: a manual override wins over an applied character style, which wins over an applied paragraph style, which wins over the document default. A paragraph style's character attributes SHALL act as the character default for text using it. The document SHALL carry a `Basic Paragraph` paragraph style that is editable but MUST NOT be renamable or deletable. Applying a style MUST NOT clear a manual override already present. A style SHALL be identified by its name, and creating, editing, deleting, or applying a style SHALL be undoable.

#### Scenario: Manual override wins

- **WHEN** text carries a manual character override and an applied character style that sets the same attribute differently
- **THEN** the resolved value is the manual override

#### Scenario: Character style overrides paragraph style

- **WHEN** the same attribute is set by both an applied character style and an applied paragraph style with no manual override
- **THEN** the resolved value is the character style's

#### Scenario: Applying a style keeps an existing manual override

- **WHEN** a style is applied to text that already carries a manual override for one of its attributes
- **THEN** the resolved value for that attribute remains the manual override

#### Scenario: Basic Paragraph is editable but not renamable or deletable

- **WHEN** the document's default `Basic Paragraph` style is renamed or deleted
- **THEN** the operation is refused, while an attribute edit to it is accepted

#### Scenario: Editing a style updates all text using it

- **WHEN** an attribute of a named style is edited
- **THEN** every run or paragraph that applies that style resolves to the new value

### Requirement: Type attribute edits commit as one history state

An edit that applies a character or paragraph attribute set to a type layer SHALL cross the app bridge and record exactly one history state, so a field commit is one undo step. A re-set that carries the layer's existing EngineData SHALL NOT record more than one state, and cancelling an edit SHALL restore the layer bit-identically without a state.

#### Scenario: One attribute edit is one undo step

- **WHEN** a character attribute is applied to a type layer and then undone once
- **THEN** the layer returns to its previous attribute value

#### Scenario: A re-set preserves the layer's pixels until commit

- **WHEN** a type edit is cancelled before commit
- **THEN** the layer's rendered pixels and stored attributes are unchanged and no history state was recorded
