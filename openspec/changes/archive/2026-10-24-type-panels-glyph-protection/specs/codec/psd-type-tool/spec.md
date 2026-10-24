# Spec Delta

## MODIFIED Requirements

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
