# Design

## Context

The bundled renderer sets a whole type layer in its first style run's
`CharacterAttrs` (`TypeTool::style`). Two paint paths exist:
`render_text_buffer` for untransformed type and `render_mapped` for type under
a matrix. Four measurers must agree with them: `type_paragraph::line_advance`
(wrapping and caret stops), `text_box_any` (the layer rect), the vertical
cell, and `type_pens`. Today each calls `BundledText::shape_line(text, size)`
directly.

## Decisions

### D1. One styled shaper

`type_style::shape_styled(face, text, c) -> Vec<ShapedGlyph>` is the only
place character styling turns into glyphs. Every measurer and painter calls it,
so caret, wrap, rect, and ink cannot drift. It:

1. Transforms the text: `to_uppercase` for All Caps. For Small Caps on a face
   without `smcp`, it splits the text into runs of lowercase and other
   characters, uppercases the lowercase runs, and marks them with glyph scale
   0.7.
2. Builds the rustybuzz feature list. Each OpenType toggle maps to a tag with
   value 0 or 1, so a default-on feature (liga, clig, calt) can be turned off:

   | Toggle | Tags |
   |---|---|
   | Standard Ligatures | `liga`, `clig` |
   | Contextual Alternates | `calt` |
   | Discretionary Ligatures | `dlig` |
   | Swash | `swsh` |
   | Oldstyle | `onum` |
   | Stylistic Alternates | `salt` |
   | Titling Alternates | `titl` |
   | Ornaments | `ornm` |
   | Ordinals | `ordn` |
   | Fractions | `frac` |
   | Small Caps (only when the face has it) | `smcp` |

   Kerning maps to `kern`: 1 for Metrics, 0 for Manual and Optical.
3. Shapes at the effective size: `size × 0.583` for super/subscript, else
   `size`. Then it applies the horizontal and vertical scale and the per-glyph
   scale, adds the Faux Bold widening (`strength`) to each advance, and adds
   the Optical pair adjustments.

`ShapedGlyph` and `PlacedGlyph` gain `scale: f32` (1.0 normally). The
`layout_lines` call copies it through, and the painter multiplies the outline
transform by it. Without per-glyph scale, synthesized small caps would need
two layouts.

### D2. Effective run geometry

`type_style::RunStyle::of(face, c)` carries what the painters need:

- the effective pixel size;
- `rise`: +0.333 × size for superscript, −0.333 × size for subscript, plus the
  baseline shift;
- `skew`: tan 12° for Faux Italic;
- `embolden`: size / 24 for Faux Bold;
- the underline and strikethrough rectangles' offsets and thickness, from the
  font's `post` and OS/2 metrics at the effective size, with a fallback of
  0.05 em below the baseline and 0.3 em above it, each 0.05 em thick.

Leading still uses the authored size, so a superscript line keeps its line
spacing. CS6's Type preferences default super/subscript to 58.3% size and
±33.3% position. Those values, the 12° slant, the 0.7 small caps, and the
bold strength are **approximated**, not oracle-verified.

### D3. Kerning modes

- **Metrics:** `kern` on, as today.
- **Manual(v):** `kern` off, plus `v`/1000 em per glyph, as today's
  tracking-style addition. `Manual(0)` is kerning off, matching the docs ("set
  Kerning to 0 to turn it off").
- **Optical:** `kern` off. For each adjacent glyph pair, it takes each glyph's
  outline profile: the extreme ink x over 16 horizontal bands from the
  baseline to cap height, with curves flattened at their control points. It
  measures the mean gap over the bands where both glyphs have ink. It then
  moves the pair toward the face's reference gap, which is the same
  measurement for `n n`. The adjustment is clamped to ±0.12 em. This is an
  **approximation** of a closed Adobe algorithm.

`KerningMode::Optical` writes `AutoKerning true` / `Kerning 0`, because CS6
EngineData has no groundable key for it. It reopens as Metrics, marked
`ponytail:` in the writer.

### D4. Decorations and the layer rect

Underline and strikethrough are painted after the glyphs, one rectangle per
line, from the line's pen start to its last advance. Rows are filled at full
coverage, and partial top and bottom rows at fractional coverage. Vertical
type draws them as columns: underline on the column's right (CS6's default
Underline Right), strikethrough through the column's centre.

`text_box_any` grows by the styled extents:

- the width by Faux Italic's overhang (`ascent × skew`) plus the bold
  strength;
- the descent by a subscript's drop when it passes the font's descent;
- the descent to the underline when the underline sits lower.

The rect therefore stays a superset of the ink.

### D5. Feature support query

`type_style::opentype_support(face) -> u32` sets one bit for each of the
panel's eight OpenType buttons whose tags appear in the face's GSUB or GPOS
feature list. Standard Ligatures counts `liga` or `clig`. The bits are in
panel order:

| Bit | Feature |
|---|---|
| 0 | Standard Ligatures |
| 1 | Contextual Alternates |
| 2 | Discretionary Ligatures |
| 3 | Swash |
| 4 | Stylistic Alternates |
| 5 | Titling Alternates |
| 6 | Ordinals |
| 7 | Fractions |

`pictura_render::opentype_features(family)` resolves the family through
`fonts::face_for`, and the bridge exposes it as
`type_api::openTypeFeatures(family)`. The panel calls `registerTypeFont`
first, so the query sees the host face rather than the fallback. A button is
also greyed out with no type layer active. A greyed button keeps its stored
state.

### D6. Panel widgets

- Fields with a preset list (size, leading, kerning, tracking) are an editable
  `QComboBox` behind an icon `QLabel`. The panel filters mouse events on the
  icon and on the combo's line edit. A horizontal drag changes the value one
  step per pixel, with the step scaled 10× by Shift and 1/10 by Ctrl, as
  `NumericField` does. The field's text follows the drag, and the layer
  commits once, on release. A live per-step layer preview would need a
  non-recording type update the bridge does not have.
- Fields without presets (the vertical and horizontal scales and baseline
  shift) use the existing `NumericField` with an icon on its label.
- The kerning combo lists `Metrics`, `Optical`, then the CS6 presets −100, −75,
  −50, −25, −10, −5, 0, 5, 10, 25, 50, 75, 100, 200. Typing a number sets
  Manual. Scrubbing from Metrics or Optical starts at 0.
- The leading combo lists `(Auto)` and the size presets. Typing a number sets
  Fixed.
- Icons are drawn at runtime with `QPainter` from text glyphs and simple
  strokes. No Adobe artwork is copied (see `CONTRIBUTING.md`).
- Toggle buttons are checkable `QToolButton`s. All Caps and Small Caps are
  exclusive (checking one clears the other), and so are Superscript and
  Subscript, as in CS6's FontCaps and FontBaseline enums.

Size, leading, and baseline shift show `px`, the model's and the options bar's
unit. CS6's `pt` display depends on the Units preference, which is not
modelled.

## Risks

- Re-shaping with optical kerning profiles each glyph's outline once per
  shape. The profiles are cached per face and glyph in a process-wide map.
- The combo scrub repeats about 20 lines of `NumericField`'s drag maths.
  Extracting a shared helper would touch every `NumericField` site, so it is
  left for later.
