# ui/type-panels Specification

## Purpose
The Glyphs panel wired to the Type tools' shared options and the open type edit.
The Character and Paragraph panels live in `ui/character-paragraph-panels`.

## Requirements

### Requirement: Glyphs panel

The Glyphs panel (a post-CS6 extra) SHALL list the glyphs the chosen font has
in its Unicode blocks, filterable by block, and SHALL insert a chosen glyph at
the open type edit's caret, or explain that a type edit must be open.

#### Scenario: A glyph lands at the caret

- **WHEN** the `tst_type_panels` test filters to Basic Latin, chooses "Z" with no type edit open, then opens an edit, types "a", chooses "Z", and commits
- **THEN** Basic Latin shows at most 95 glyphs and the entire font more, the first choice shows the hint and inserts nothing, and the committed layer's text is "aZ"
