# Spec Delta

## MODIFIED Requirements

### Requirement: TypeTool exposes the font set and first-run style

`read_psd` SHALL fill `TypeTool.fonts` with the embedded source's font-set names
and `TypeTool.style` with the first style run's effective character attributes —
font name, font style, size, language, leading including its auto flag, tracking,
kerning mode and value, horizontal scale, vertical scale, baseline shift, tsume,
anti-aliasing method, fill colour, and the character toggles — resolved against
the paragraph/style defaults when a run omits a field. It SHALL expose the first
paragraph run's effective paragraph attributes — alignment, justification,
direction, word/letter spacing and glyph scaling, indents, space before/after,
hanging punctuation, hyphenation, and composer — resolved the same way. The
anti-aliasing method SHALL come from the text descriptor's `AntA` enum when it is
present, falling back to the EngineData `/AntiAlias` flag. When the `Txt `
descriptor has no EngineData, or the EngineData cannot be parsed, the style SHALL
be `None`, the font list empty, and the document read SHALL still succeed with the
`Txt ` string and raw descriptor bytes preserved.

#### Scenario: A real text layer exposes its font and size

- **WHEN** the fixture whose first style run sets `FontSize` 150 and defaults to font index 1 is read
- **THEN** `TypeTool.style` reports the font name at index 1, size 150, and the run's fill colour

#### Scenario: A run omitting a character attribute resolves to the sheet default

- **WHEN** the first style run omits `Tracking` and the style sheet sets it to 40
- **THEN** `TypeTool.style` reports tracking 40

#### Scenario: A run omitting a paragraph attribute resolves to the paragraph default

- **WHEN** the first paragraph run omits `SpaceAfter` and the paragraph sheet sets it to 12
- **THEN** `TypeTool`'s paragraph attributes report space after 12

#### Scenario: The anti-aliasing method is reported from the descriptor

- **WHEN** the fixture's text descriptor `AntA` names the Crisp method
- **THEN** `TypeTool.style` reports anti-aliasing Crisp

#### Scenario: The EngineData flag is the anti-aliasing fallback

- **WHEN** the text descriptor has no `AntA` and the EngineData `/AntiAlias` flag is set
- **THEN** `TypeTool.style` reports anti-aliasing Sharp

#### Scenario: A missing EngineData leaves the style unset

- **WHEN** a `TySh` block's `Txt ` descriptor has no `EngineData` value
- **THEN** `TypeTool.style` is `None` and the document read succeeds

#### Scenario: A malformed EngineData does not fail the document

- **WHEN** the `EngineData` value is not parseable
- **THEN** `TypeTool.style` is `None` and the layer still reads with its text and raw `TySh` bytes

#### Scenario: Language resolves to the sheet default

- **WHEN** the first style run omits `/Language` and the style sheet sets it to `German`
- **THEN** `TypeTool.style` reports language `German`

#### Scenario: Tsume, language, and direction decode

- **WHEN** the first style run carries `/Tsume` 60 and `/Language` `French`, and the shape carries `WritingDirection` 1
- **THEN** the decoded style reports tsume 60, language `French`, and direction RTL

#### Scenario: A missing tsume or direction takes the default

- **WHEN** the first style run omits tsume and the shape omits its writing direction
- **THEN** the decoded style reports tsume 0 and direction LTR
