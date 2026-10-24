# Proposal

## Why

Four type gaps remain visible at once: the Character panel still omits font
style, language, tsume, and fractional-widths controls (#517); the Paragraph
panel has no Justification dialog or paragraph-direction control (#518); there
is no Character Styles panel even though the style model and bridge write path
exist (#519); and a type layer still renders every character in one face, so
non-Roman input can show tofu where CS6 substitutes a covering font (#887).
They share the same model, bridge, preferences page, and layout seams, so they
ship as one bundle rather than four overlapping changes.

## What Changes

- **Character panel (#517).** Add a font-style picker backed by style-specific
  face registration, a Language combo (stored; no spelling engine), a Set Tsume
  field shown while the East Asian preference is on, and a Fractional Widths
  panel-menu toggle that rounds advances to whole pixels. Faux Bold, Faux Italic,
  Underline, and the OpenType buttons already ship and are verified rather than
  rebuilt.
- **Paragraph panel (#518).** Add the Justification dialog (word, letter, and
  glyph spacing min/desired/max plus auto-leading) and make justified layout
  honour those bounds instead of dividing leftover width evenly. Add paragraph
  direction LTR/RTL with bidi run ordering, RTL line placement, and RTL caret
  movement; the direction control appears only while the Middle Eastern
  preference is on.
- **Character Styles panel (#519).** Add the panel, its options dialog, the
  Window and Type menu toggles, and a provenance-recorded icon. The type bridge
  gains character-style read access (count, name, family, attributes), a live
  preview call, and a rename-capable edit; apply, create, and delete already
  exist. Renaming a style updates every layer that applies it.
- **Type preferences.** Add the Type pane's East Asian / Show Asian Text
  Options, Middle Eastern, and Enable Missing Glyph Protection (default on)
  settings as process-wide state, saved with the session and restored on the
  next start.
- **Missing Glyph Protection (#887).** When the active face lacks a glyph and
  the preference is on, shape and render that character in a registered face
  that covers it; wrapping, caret stops, and bounds use the same mixed-face
  layout. Selection follows a deterministic fallback list built for the active
  family. With the preference off, the current face is used unchanged.
- **Model and codec.** Expose the existing character `language` and add character
  `tsume` and paragraph `direction` to the model, then author/decode them
  (`/Tsume`, `/Language`, `WritingDirection`). Keys that cannot be verified
  against a CS6 file are written as documented approximations and recorded as a
  parity ceiling.
- **Corpus cleanup.** Remove the stale Character and Paragraph panel
  requirements from `ui/type-panels`; `ui/character-paragraph-panels` is the
  live contract for those panels, and `ui/type-panels` keeps the Glyphs panel.

## Capabilities

### New Capabilities

- `ui/character-styles-panel`: the Character Styles panel that lists, applies,
  creates, edits, and deletes named character styles, and its menu wiring.
- `ui/type-preferences`: the Type preferences pane controls for the East Asian
  and Middle Eastern options and Missing Glyph Protection.
- `tools/missing-glyph-protection`: per-character font fallback during shaping,
  layout, caret placement, and rendering, gated by the preference.

### Modified Capabilities

- `ui/character-paragraph-panels`: Character panel gains font style, language,
  tsume, and fractional widths; Paragraph panel gains the Justification dialog
  and direction control.
- `ui/type-panels`: remove the superseded Character and Paragraph requirements.
- `document/type-style-model`: document the existing language and add character
  tsume and paragraph direction.
- `codec/psd-type-tool`: author tsume, language, and direction, and decode them
  back.
- `tools/type-engine-data`: decode tsume, language, and direction from
  EngineData into the modelled style.
- `tools/text-shaping`: shape by paragraph direction with bidi run ordering.
- `tools/type-character-rendering`: select the face for the requested font
  style and make the OpenType and outline queries follow that face.
- `tools/paragraph-type`: wrap and place paragraph text by direction (alignment
  keeps its named edge).
- `tools/type-caret`: place and move the caret in reading order for RTL text.

## Impact

- `crates/pictura-core`: `CharacterAttrs.tsume`, `ParagraphAttrs.direction`, a
  direction enum, and the style-resolution merge for the new fields.
- `crates/pictura-codec`: `type_write.rs` authors `/Tsume`, `/Language`, and
  `WritingDirection`; `engine_data.rs` extracts them. The style and paragraph
  merges keep their unknown keys; the rebuilt `/Rendered` shape does not.
- `crates/pictura-render`: `fonts.rs` gains style- and coverage-aware face
  lookup, an insertion-ordered fallback list, and style-aware OpenType/outline
  queries; `type_style`, `type_paragraph`, `text_render`, `type_layer`,
  `type_justify`, `type_caret`, `type_path`, `type_warp`, and the rasterizer
  carry a face per shaped glyph. New dependency `unicode-bidi` for UAX #9 run
  ordering (a hand-rolled bidi implementation is larger and easier to get
  wrong). PDF live text keeps its one-face path: a layer that needs fallback
  rasterizes in PDF rather than emitting mixed-face live text.
- `crates/pictura-app`: the type bridge gains character-style read and preview
  calls, a rename-capable edit, tsume/direction/font-style mapping, and a helper
  that registers host faces for requested families/styles and script fallbacks;
  `type_fonts.cpp` resolves and registers style-specific faces. Character,
  Paragraph, and new Character Styles panels plus the Type preferences page;
  menus, commands, icons, session persistence, and CMake registration.
- A document whose text the active face fully covers keeps its current rendered
  pixels. A document with an uncovered character now renders the fallback by
  default; turning Enable Missing Glyph Protection off restores the old output.
  Re-authored EngineData gains the new keys at their documented defaults,
  because the author writes every modelled attribute.
