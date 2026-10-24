# psd-type-tool Specification

## Purpose
Decodes TySh into a typed TypeTool view, round-trips unmodified bytes, and rebuilds a well-formed block.

## Requirements

### Requirement: TySh decodes into a typed TypeTool view

The system SHALL parse a layer's preserved `TySh` additional-layer-info block
into a derived `TypeTool` view when the payload is well-formed. The view SHALL
carry: the six `f64` affine transform values (`xx`, `xy`, `yx`, `yy`, `tx`,
`ty`); the text string from the text descriptor's `Txt ` key when present; the
four `i32` bounds `left`, `top`, `right`, `bottom`; and the text and warp
descriptors as parsed descriptor objects. A payload whose version is not `1`,
whose text version is not `50`, whose warp version is not `1`, or whose
descriptors do not parse SHALL leave the view `None` and MUST NOT fail the
document read. Detection of type-layer kind SHALL remain presence of `TySh` in
`extra_blocks`.

#### Scenario: A well-formed TySh yields a TypeTool view

- **WHEN** a layer carries a `TySh` block with version 1, a 6-value transform,
  text version 50, parseable text and warp descriptors, and four bounds
- **THEN** `layer.type_tool()` is `Some` with that transform, the `Txt ` string,
  and those bounds

#### Scenario: Missing Txt yields an empty text string

- **WHEN** the text descriptor parses but has no `Txt ` key
- **THEN** the view's text string is empty and the other fields still decode

#### Scenario: Malformed TySh leaves the view unset without failing the document

- **WHEN** `TySh` has version other than 1 or a truncated descriptor
- **THEN** document read succeeds and `type_tool()` is `None`

### Requirement: TySh bytes round-trip on unmodified save

An unmodified document whose layer carries `TySh` SHALL re-emit the same
`TySh` key and payload bytes on write, because serialization SHALL continue to
use `Layer.extra_blocks` rather than the derived view.

#### Scenario: Open-save preserves TySh bytes

- **WHEN** a document with a well-formed `TySh` is written and read back
- **THEN** the layer's `TySh` payload is byte-identical to the input

### Requirement: encode_type_tool rebuilds a well-formed block

`pictura-codec` SHALL expose `encode_type_tool(&TypeTool) -> Vec<u8>` that
emits version 1, the six transform doubles, text version 50, the text
descriptor (version-16 block), warp version 1, the warp descriptor, and the
four `i32` bounds. Decoding the encoder's output SHALL yield a `TypeTool` equal
to the input for transform, text string (via `Txt `), and bounds.

#### Scenario: Encode then decode preserves fields

- **WHEN** a `TypeTool` is encoded and the bytes are decoded
- **THEN** transform, `Txt ` text, and bounds match the input view

#### Scenario: Encoder output is version 1 with text version 50

- **WHEN** `encode_type_tool` is called on any well-formed view
- **THEN** the first `u16` is 1 and the `u16` after the transform is 50

### Requirement: Authored EngineData carries the modelled type style

When a type block is built for a layer, the author SHALL write each modelled
character attribute into the `StyleRun` `StyleSheetData` keys — `/Font` (an index
into `/FontSet`), `/FontSize`, `/Leading` with `/AutoLeading`, `/Tracking`,
`/Kerning` with `/AutoKerning`, `/HorizontalScale`, `/VerticalScale`,
`/BaselineShift`, `/Tsume`, `/Language`, the fill colour `/FillColor`, and the
modelled toggle keys — and each modelled paragraph attribute into the
`ParagraphRun` `ParagraphSheet` `/Properties` keys: `/Justification`,
`/FirstLineIndent`, `/StartIndent`, `/EndIndent`, `/SpaceBefore`, `/SpaceAfter`,
`/AutoHyphenate`, `/LeadingType`, and `/EveryLineComposer`, with the paragraph
direction written as `WritingDirection` in the `/Rendered` shape and its line
cache. The `/Rendered` shape SHALL be rebuilt from the model rather than merged;
unmodelled keys inside that shape are a documented ceiling, while the style and
paragraph merges preserve their unknown keys. An unset attribute SHALL be written
with its documented default rather than an arbitrary value. The anti-aliasing
method SHALL be authored into the `TySh` text descriptor's `AntA` enum, and the
EngineData `/AntiAlias` flag SHALL agree with it. Authoring a non-default tsume,
language, or direction and reading the block back SHALL report the same value.

#### Scenario: A modelled character value survives author and re-read

- **WHEN** a type block is authored from a spec whose tracking is 120 and horizontal scale is 85 percent
- **THEN** reading the block back reports tracking 120 and horizontal scale 85

#### Scenario: A modelled paragraph value survives author and re-read

- **WHEN** a type block is authored from a spec whose first-line indent is 24 and space after is 12
- **THEN** reading the block back reports first-line indent 24 and space after 12

#### Scenario: The anti-aliasing method is authored to the descriptor and the flag

- **WHEN** a type block is authored with anti-aliasing None, Sharp, Crisp, Strong, or Smooth
- **THEN** the text descriptor's `AntA` encodes that method's CS6 spelling and the EngineData `/AntiAlias` flag agrees

#### Scenario: Tsume, language, and direction survive author and re-read

- **WHEN** a type block is authored with tsume 60, language `French`, and direction RTL
- **THEN** reading the block back reports tsume 60, language `French`, and direction RTL

#### Scenario: A re-author keeps an existing direction

- **WHEN** a type block whose shape carries RTL is re-authored for an unrelated attribute
- **THEN** the rebuilt `/Rendered` reports direction RTL again

### Requirement: Re-author preserves unmodeled EngineData keys

When the author rewrites a type block that already exists on a layer, it SHALL
parse the existing EngineData and overwrite only the modelled keys of the text's
single style run and single paragraph sheet, so every other key of that block
survives with its value. When the layer carries no existing EngineData, the
author SHALL emit a complete skeleton with the documented defaults. Text whose
EngineData holds more than one style run is outside the model: its re-author
collapses to the single modelled run, and per-paragraph properties beyond the
first paragraph are likewise collapsed, both as documented ceilings.

#### Scenario: An unmodeled key survives a re-set

- **WHEN** a single-run type layer whose EngineData carries a key the model does not cover is re-set through the author
- **THEN** that key and its value are still present, valid, in the rewritten block

#### Scenario: A layer without EngineData is authored complete

- **WHEN** a new type layer is authored with no existing EngineData
- **THEN** the emitted block contains the full style and paragraph skeleton with documented defaults

#### Scenario: A multi-run block is re-authored to the model's single run

- **WHEN** a type layer whose EngineData holds two style runs is re-set
- **THEN** the result holds one style run (the collapse is a documented ceiling, not a recorded event)
