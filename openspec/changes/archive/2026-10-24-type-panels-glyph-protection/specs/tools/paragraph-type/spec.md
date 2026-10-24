# Spec Delta

## ADDED Requirements

### Requirement: Paragraph direction controls the line order in the box

A paragraph-type layer whose direction is right-to-left SHALL place its first
logical character at the line's right edge; a right-aligned line puts that edge
at the box's right edge. Wrapping SHALL still break the same logical text at the
same words or characters; only the visual placement of the line within the box
changes. Left, Center, and Right alignment SHALL keep their named edge
regardless of direction, and a justified line SHALL still fill the box width.
Overflow SHALL count the same logical lines, and hidden lines SHALL be those
that do not fit the box.

#### Scenario: The first character starts at the right edge

- **WHEN** a paragraph box is set to RTL and laid out
- **THEN** the first logical character's glyph sits at the line's right edge and later characters run leftward

#### Scenario: Wrapping breaks at the same words

- **WHEN** the same text is laid out in an LTR and an RTL box of the same width
- **THEN** both layouts break at the same word boundaries and produce the same number of lines

#### Scenario: Alignment keeps its named edge

- **WHEN** an RTL paragraph is centered and then right-aligned in a box
- **THEN** the centered line is centered in the box and the right-aligned line touches the box's right edge

#### Scenario: Overflow is counted in logical lines

- **WHEN** RTL text needs more lines than the box holds
- **THEN** the lines that do not fit are hidden and the box reports overflow

### Requirement: Justified layout honours the spacing bounds

A justified line SHALL distribute its leftover width in CS6 order: word spaces
first, then letter spacing, then glyph scaling. Word spacing SHALL clamp to the
Minimum and Maximum around the Desired value; letter spacing and glyph scaling
SHALL do the same. Glyph scaling SHALL scale each glyph's advance and outline
width. When the bounds cannot absorb the leftover width, the line SHALL keep the
closest permitted spacing instead of exceeding a bound. Desired values SHALL
apply to justified and non-justified lines; Minimum and Maximum SHALL apply only
where justification spreads the line.

#### Scenario: Word spacing absorbs the leftover first

- **WHEN** a line justified to a column has room to grow and its word-spacing Maximum permits the growth
- **THEN** the added width goes to the word spaces and letter spacing and glyph scaling stay at Desired

#### Scenario: Excess width spills into letter spacing

- **WHEN** the word-spacing Maximum cannot absorb the leftover width
- **THEN** the remainder goes to letter spacing within its Minimum and Maximum

#### Scenario: Bounds clamp the spread

- **WHEN** the leftover width exceeds what word, letter, and glyph bounds permit
- **THEN** the line uses the Maximum values and keeps the remainder ragged

#### Scenario: Auto leading uses the stored value

- **WHEN** Auto Leading is 130 and a line's leading is Auto
- **THEN** its baseline-to-baseline distance is 130% of the font size

