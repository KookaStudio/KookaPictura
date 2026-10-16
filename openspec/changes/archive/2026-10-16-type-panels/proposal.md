# Proposal: type-panels

## Why

Issue #65: import the Character, Paragraph, and Glyphs panels from photorust
(`shell/src/panels/CharacterPanel.cpp`, `ParagraphPanel.cpp`,
`GlyphsPanel.cpp`). Window > Character / Paragraph and Type > Panels >
Character / Paragraph were inert placeholders, and the Type options bar had no
panel toggle.

## What Changes

- `panels/character_panel.*` (new): font family, size (px, 1–1296, applied on
  a pick or Enter), text colour (sets the foreground, as the bar's swatch
  does), and anti-aliasing (None / Sharp) over the Type tools' shared
  `TypeOptions`, following them as they change. Font style, leading, kerning,
  tracking, vertical / horizontal scale, and baseline shift are shown
  disabled: the type model has none of them.
- `panels/paragraph_panel.*` (new): the three paragraph alignments over
  `TypeOptions::justification`, turned to top / centre / bottom while a
  vertical Type tool is active. Justify All, the indents, paragraph spacing,
  and Hyphenate are shown disabled.
- `panels/glyphs_panel.*` (new): family / style / subset pickers over a grid of
  the code points the font has in sixteen Unicode blocks, with a zoom slider;
  a double-click inserts the glyph at the open type edit's caret (switching the
  type's family only when it lacks the glyph) or explains that a type edit must
  be open. **Not a CS6 panel** — Photoshop added it in CC 2015; ported because
  #65 asks for it, as a post-CS6 extra, and `docs/` is unchanged.
- `ToolHandler::insertText` / `ToolController::insertText` (new): the Type
  tool inserts text at its caret.
- Character, Paragraph, and Glyphs dock as one group, hidden by default. Window > Character / Paragraph / Glyphs toggle them, Type >
  Panels > Character / Paragraph open them, and the Type options bar gains
  CS6's Toggle the Character and Paragraph panels button.
- The bar's size list, alignment glyph, and colour swatch are shared with the
  panels (`typeSizes`, `typeAlignIcon`, `typeSwatchIcon`).
- Three Lucide 1.50.0 tab icons (`window.panels.character|paragraph|glyphs`).
- Tests: Qt Test `tst_type_panels`.

## Capabilities

### New Capabilities

- `ui/type-panels`: the Character, Paragraph, and Glyphs panels.

## Impact

- `pictura-app` (C++ only), `assets/icons`. No new dependency.

## Provenance

Panel layout, the Unicode block list, and the glyph-insertion rule from
photorust's panels; the CS6 contract in
`docs/02-ui-ux/panels/character-and-paragraph.md`. Ceiling (`ponytail:`): only
the attributes `TypeOptions` models are live, Character Styles / Paragraph
Styles stay placeholders, and the Glyphs grid sees only the listed blocks.
