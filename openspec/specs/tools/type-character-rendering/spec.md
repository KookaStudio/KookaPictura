# tools/type-character-rendering Specification

## Purpose
Character-attribute rendering for type layers: OpenType feature toggles, caps and script styles, faux styles, decorations, and kerning modes.

## Requirements

### Requirement: OpenType feature toggles reach the shaper

The bundled renderer SHALL shape a type layer's text with the OpenType features
its character attributes select: Standard Ligatures (`liga`, `clig`), Contextual
Alternates (`calt`), Discretionary Ligatures (`dlig`), Swash (`swsh`), Oldstyle
(`onum`), Stylistic Alternates (`salt`), Titling Alternates (`titl`), Ornaments
(`ornm`), Ordinals (`ordn`), and Fractions (`frac`). A toggle that is off SHALL
disable its feature even when the shaper enables it by default. A face without
the feature SHALL shape as if the toggle were off.

#### Scenario: Turning standard ligatures off breaks the ligature

- **WHEN** text the face ligates is shaped with Standard Ligatures on and then off
- **THEN** the on run yields fewer glyphs than the off run

#### Scenario: A discretionary feature changes the glyphs only when on

- **WHEN** a face that carries a toggled OpenType feature shapes text the feature affects, with the toggle off and then on
- **THEN** the glyph ids differ between the two runs

#### Scenario: A feature the face lacks changes nothing

- **WHEN** a toggle whose feature the face does not carry is turned on
- **THEN** the shaped glyphs equal the glyphs shaped with it off

### Requirement: Per-font OpenType support

`pictura-render` SHALL report, for a font family, which of the Character
panel's eight OpenType features the resolved face carries, as a bit mask in
panel order: Standard Ligatures, Contextual Alternates, Discretionary
Ligatures, Swash, Stylistic Alternates, Titling Alternates, Ordinals,
Fractions. A bit SHALL be set exactly when the face's GSUB or GPOS feature list
names the feature's tag (`liga` or `clig` for Standard Ligatures).

#### Scenario: Support follows the face's feature list

- **WHEN** the support mask of a face is requested
- **THEN** each bit is set exactly when the face's layout tables list that feature's tag

### Requirement: Caps, position, and faux styles render

The renderer SHALL render the layer-wide character toggles as follows:

- **All Caps:** the text sets in upper case.
- **Small Caps:** the face's `smcp` feature, when the face carries it.
  Otherwise lower-case letters set as capitals at 70% of the size.
- **Superscript and Subscript:** the text sets at 58.3% of the size, its
  baseline raised (superscript) or lowered (subscript) by 33.3% of the size.
- **Faux Bold:** thickens each outline, and widens each advance by the same
  strength.
- **Faux Italic:** slants each outline about 12° to the right.

These proportions are approximations of CS6 and SHALL be marked as such.
Leading SHALL keep using the authored size.

#### Scenario: All Caps sets upper case

- **WHEN** lower-case text is rendered with All Caps on
- **THEN** its glyphs equal the glyphs of the same text in upper case

#### Scenario: Synthesized small caps are smaller capitals

- **WHEN** lower-case text is rendered with Small Caps on in a face without `smcp`
- **THEN** each glyph is the capital's glyph at 70% scale

#### Scenario: Superscript is smaller and raised

- **WHEN** text is rendered with Superscript on
- **THEN** its ink is shorter than the plain render and its ink bottom sits above the plain baseline

#### Scenario: Subscript is smaller and lowered

- **WHEN** text is rendered with Subscript on
- **THEN** its ink bottom sits below the plain render's ink bottom

#### Scenario: Faux Bold adds ink

- **WHEN** text is rendered with Faux Bold on
- **THEN** the render covers more pixels than the plain render and its advance is wider

#### Scenario: Faux Italic slants right

- **WHEN** a single upright glyph is rendered with Faux Italic on
- **THEN** its ink's top rows sit further right relative to its bottom rows than in the plain render

### Requirement: Underline and strikethrough render

The renderer SHALL draw an underline below each horizontal line's baseline and a
strikethrough across its x-height band. Each SHALL span the line from its pen
start to its last advance, at the position and thickness the face's `post` and
OS/2 metrics give for the effective size. A face without the metrics SHALL use
0.05 em below the baseline and 0.3 em above it, each 0.05 em thick. Vertical
type SHALL draw the underline down the column's right side and the
strikethrough down its centre. The layer rect SHALL include the decoration ink.

#### Scenario: Underline paints below the baseline

- **WHEN** text is rendered with Underline on
- **THEN** a row below the baseline is painted across the line's width that the plain render leaves empty

#### Scenario: Strikethrough paints through the letters

- **WHEN** text without descenders is rendered with Strikethrough on
- **THEN** a band between the baseline and the x-height is fully covered across the line's width

### Requirement: Kerning modes render

Metrics kerning SHALL apply the face's `kern`/GPOS pair kerning. Manual kerning
SHALL disable the face's pair kerning and add its value in 1/1000 em after each
glyph, so a manual value of 0 renders unkerned. Optical kerning SHALL disable
the face's pair kerning and space each pair from the glyph outlines toward the
face's `n n` gap, with each adjustment clamped to ±0.12 em. This is an
approximation of CS6's closed algorithm. A larger manual value SHALL widen the
line.

#### Scenario: Manual zero turns metrics kerning off

- **WHEN** a pair the face kerns is shaped with Metrics and with Manual 0
- **THEN** the Manual 0 advance is larger than the Metrics advance and equals the unkerned sum

#### Scenario: Manual kerning widens the line

- **WHEN** a line is measured with Manual 0 and with Manual 100
- **THEN** the Manual 100 line is wider by 0.1 em per glyph

#### Scenario: Optical tightens an open pair

- **WHEN** an open diagonal pair such as `AV` is shaped with Optical and with Manual 0
- **THEN** the Optical advance is smaller

### Requirement: Measurement matches the render

The caret stops, the paragraph wrap, and the point-type layer rect SHALL
measure the same styled glyph run the renderer paints, including caps, scale,
super/subscript size, Faux Bold widening, and kerning mode.

#### Scenario: All Caps widens the measured line

- **WHEN** lower-case text's line advance is measured with All Caps off and on
- **THEN** the All Caps advance equals the advance of the upper-case text and is larger

#### Scenario: The caret's last stop meets the styled advance

- **WHEN** the caret stops of a styled point-type line are computed
- **THEN** the distance from its first to its last stop equals the styled line advance
