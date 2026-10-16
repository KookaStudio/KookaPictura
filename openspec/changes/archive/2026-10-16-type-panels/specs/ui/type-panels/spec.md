## ADDED Requirements

### Requirement: Character panel

The Character panel SHALL edit the Type tools' shared font family, size,
text colour, and anti-aliasing as the Type options bar does, SHALL follow
them when they change elsewhere, and SHALL show the attributes the type model
lacks (font style, leading, kerning, tracking, scale, baseline shift) disabled.
Window > Character SHALL toggle it, Type > Panels > Character SHALL open it,
and the Type options bar's panel button SHALL toggle it.

#### Scenario: The panel edits and follows the type options

- **WHEN** the `tst_type_panels` test opens the panel from the Type menu, sets the size to 30 and then to an out-of-range 5000, turns anti-aliasing off, picks another family, changes the options from outside, and clicks the Type bar's panel button twice
- **THEN** the options take 30, keep 30 for 5000, take None and the family, the panel shows the outside change, the unmodelled fields are disabled, and the button hides then shows the panel

### Requirement: Paragraph panel

The Paragraph panel SHALL set and follow the Type tools' paragraph alignment
(left / centre / right; top / centre / bottom while a vertical Type tool is
active) and SHALL show justification, indents, paragraph spacing, and
hyphenation disabled.

#### Scenario: Alignment round-trips and turns for vertical type

- **WHEN** the `tst_type_panels` test clicks Center, sets right alignment from outside, and switches between the vertical and horizontal Type tools
- **THEN** the justification becomes centre, the Right button checks, the first button reads "Top align text" for vertical type and "Left align text" for horizontal, and the indent fields are disabled

### Requirement: Glyphs panel

The Glyphs panel (a post-CS6 extra) SHALL list the glyphs the chosen font has
in its Unicode blocks, filterable by block, and SHALL insert a chosen glyph at
the open type edit's caret, or explain that a type edit must be open.

#### Scenario: A glyph lands at the caret

- **WHEN** the `tst_type_panels` test filters to Basic Latin, chooses "Z" with no type edit open, then opens an edit, types "a", chooses "Z", and commits
- **THEN** Basic Latin shows at most 95 glyphs and the entire font more, the first choice shows the hint and inserts nothing, and the committed layer's text is "aZ"
