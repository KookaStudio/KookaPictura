# Proposal

## Why

The character model already carries the OpenType toggles (ligatures, swash,
fractions, …), the caps and super/subscript toggles, Faux Bold/Italic,
Underline, and Strikethrough. The Character panel and the Paragraph Style
Options dialog edit them, and the PSD codec round-trips them. The bundled
renderer reads none of them, though: it shapes with the font's default features
and paints plain glyphs. Kerning also looks broken. The panel offers only
Metrics and Manual, its value box stays disabled until Manual is picked, and
Optical does not exist. The panel is a plain form of spin boxes and checkboxes,
not CS6's compact icon grid with scrubbable fields (#780).

## What Changes

- The renderer honours the character toggles across the whole layer, as it
  already does for every other character attribute (first style run):
  - OpenType features are passed to the shaper.
  - All Caps and Small Caps change the text; Small Caps falls back to capitals
    at 70% when the face has no `smcp`.
  - Superscript and subscript render smaller and raised or lowered.
  - Faux Bold thickens the glyph outlines and Faux Italic slants them.
  - Underline and strikethrough draw lines from the font's metrics.
- Kerning gains Optical: an outline-based pair spacing approximation. A manual
  value replaces the font's metrics kerning, so 0 turns kerning off.
  Optical saves as metrics kerning, because CS6 EngineData has no known key
  for it.
- Caret stops, paragraph wrapping, and the layer rect measure the same styled
  text the renderer paints.
- A new bridge call reports which of the Character panel's OpenType features
  the current font supports.
- The Character panel is rebuilt to the CS6 layout:
  - icon-labelled fields: size, leading, kerning, tracking, the vertical and
    horizontal scales, and baseline shift;
  - the Color swatch;
  - a row of eight style toggle buttons: Faux Bold, Faux Italic, All Caps,
    Small Caps, Superscript, Subscript, Underline, Strikethrough;
  - a row of eight OpenType buttons that grey out when the font lacks the
    feature: Standard Ligatures, Contextual Alternates, Discretionary
    Ligatures, Swash, Stylistic Alternates, Titling Alternates, Ordinals,
    Fractions;
  - the anti-aliasing combo.

  Size, leading, kerning, and tracking are editable combos with CS6's preset
  lists. Dragging horizontally on any numeric field's icon or value changes it.

Out of scope, still tracked in #517: language, tsume, and fractional widths.
Font style is not modelled (the options bar has the same ceiling), so the
style combo is not built.

## Capabilities

### New Capabilities

- `tools/type-character-rendering`: how the bundled renderer renders the
  OpenType features, caps, super/subscript, Faux Bold/Italic,
  underline/strikethrough, and the three kerning modes; that measurement
  matches rendering; and the per-font OpenType support query.

### Modified Capabilities

- `ui/character-paragraph-panels`: the Character panel's layout, its toggle
  buttons and OpenType buttons, the CS6 kerning field, and scrubbable numeric
  fields.
- `document/type-style-model`: the character attribute set gains Optical
  kerning and makes Superscript and Subscript mutually exclusive.

## Impact

- `crates/pictura-core`: `KerningMode::Optical`; `PlacedGlyph` and
  `ShapedGlyph` gain a per-glyph `scale` (synthesized small caps).
- `crates/pictura-render`: a new `type_style` module (feature list, styled
  shaping, optical kerning, decorations, feature support). It is used by
  `text_render`, `type_layer`, `type_paragraph`, and `type_caret`.
- `crates/pictura-codec`: Optical writes as `AutoKerning true`.
- `crates/pictura-app`: bridge kerning mode 1 = Optical and a new
  `type_font_opentype_features` call. The Character panel is rewritten, and
  its icons are drawn in a new `character_panel_icons` unit.
- No new dependencies. Rustybuzz feature toggles, ttf-parser tables, and swash
  emboldening are all in crates already present.
