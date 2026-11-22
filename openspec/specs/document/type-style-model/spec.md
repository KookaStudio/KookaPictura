# document/type-style-model Specification

## Purpose
Defines the character and paragraph attribute sets, the named character and paragraph styles, the hierarchy that resolves the effective formatting of a run or paragraph, and the app edit that applies them under one history state, so type panels and the text engine share one model.

## Requirements

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

### Requirement: Named style edits are one validated document operation

The system SHALL apply a named character or paragraph style edit — an optional
rename from its source name to a target name plus the replacement character and
paragraph attribute sets — as one document operation that validates the whole
request before changing anything. The operation MUST refuse a request whose
source style does not exist, whose target name already names another style of
the same kind in the sheet, that renames the protected `Basic Paragraph` style,
or that renames another style to `Basic Paragraph`. A refused request MUST leave
the style sheet,
every type layer's stored applied-style reference, the rendered type content, the
composite pixels, and history unchanged, and MUST report failure to its caller.

On acceptance the operation SHALL rename the style when the names differ, write
the replacement attributes, rewrite the applied-style reference of every type
layer that applied the source name to the target name — including layers nested
inside groups — and, where the document's type layers are editable (8-bit RGB),
re-resolve every type layer applying the target name against the updated sheet
while preserving each layer's manual overrides. Editing `Basic Paragraph` under
its own name SHALL re-resolve every type layer that applies another character or
paragraph style, because those layers inherit `Basic Paragraph` as their base; a
type layer that applies no style keeps its authored attributes as local
formatting and is left as it is. Where they are not editable, the sheet and
reference changes still apply and the rendered content is left as it was. An accepted edit, including a rename combined with attribute changes, SHALL
be one undoable state that restores the style sheet, the applied references, and
the resulting type content together. A refused edit SHALL record no history
state.

#### Scenario: Renaming an applied paragraph style keeps it applied

- **WHEN** a paragraph style applied to type layers, including a type layer nested in a group, is renamed together with an attribute change and the rename is accepted
- **THEN** the style sheet holds the new name, every applying layer references it, and a later attribute edit to the renamed style still updates those layers' content

#### Scenario: A rename and its attribute change are one undo step

- **WHEN** an accepted edit renames an applied paragraph style and changes an attribute, and then undo runs once
- **THEN** the style name in the sheet, the applying layers' references, and their rendered content are restored together, and redo reapplies all three together

#### Scenario: A duplicate target name is refused

- **WHEN** a rename request names a target name already used by another style of the same kind in the sheet
- **THEN** the operation reports failure and the style sheet, the applied references, the rendered content, the composite pixels, and history are unchanged

#### Scenario: A missing source style is refused

- **WHEN** an edit or rename request names a source style that does not exist
- **THEN** the operation reports failure and the style sheet, the applied references, the rendered content, the composite pixels, and history are unchanged

#### Scenario: Basic Paragraph is never renamed

- **WHEN** a request renames `Basic Paragraph` to another name or renames another paragraph style to `Basic Paragraph`
- **THEN** the operation reports failure and leaves the sheet, applied references, content, and history unchanged, while an attribute edit to `Basic Paragraph` under its own name is still accepted

#### Scenario: Editing Basic Paragraph reaches the layers that apply another style

- **WHEN** `Basic Paragraph` is edited under its own name and a type layer applies another character or paragraph style
- **THEN** the operation re-resolves that layer against the updated sheet, so an attribute it inherits from `Basic Paragraph` changes while the applied style's own attributes still win

#### Scenario: Character styles share the same edit contract

- **WHEN** an applied character style is renamed with an attribute change, or a character-style request is refused for a missing source or duplicate target
- **THEN** the accepted edit keeps every applying layer's reference and records one undoable state, and the refused request changes nothing and reports failure

#### Scenario: Non-editable type content is left as it is

- **WHEN** a rename is accepted in a document whose type layers cannot be re-set (not 8-bit RGB)
- **THEN** the style sheet and the applying layers' references carry the new name, and the rendered type content is left as it was
