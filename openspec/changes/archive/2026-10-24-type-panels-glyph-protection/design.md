# Design

## Context

See `proposal.md` for motivation and the delta specs for behavior. The current
code already has the pieces the bundle extends:

- `CharacterAttrs` already carries `language` (the bridge and reader use it);
  `CharacterAttrs` lacks tsume, `ParagraphAttrs` lacks direction, and the codec
  never authors `/Language` or `/Tsume`. The bridge's
  `ParagraphSetting.direction` is inert.
- `pictura-render::fonts` resolves one face per family; `pictura-app`'s
  `registerTypeFont` registers `QFont(family)`'s default face only.
- `shape_styled` shapes a line in one face and `ShapedGlyph` carries no face or
  source-cluster identity.
- `type_justify` spreads leftover width evenly; spacing bounds, glyph scaling,
  and auto leading are stored but unused.
- `TextStyleSheet` and the character-style write path exist. The bridge has
  apply/create/edit/delete but no read calls, no preview call, and no rename;
  no panel or dialog exists.
- `type_shape::rendered` already writes `WritingDirection` 0 in `/Rendered
  /Shapes` and its `Lines` cache; the reader does not extract it, so re-authoring
  discards a non-zero value today.
- The Type preferences page exists with a single placeholder checkbox.

## Goals / Non-Goals

**Goals**

- Keep the type model as the single source of attributes; the panel, codec,
  renderer, and styles read and write the same structs.
- Make face selection explicit: a glyph knows which registered face produced it,
  so mixed-face layout measures and paints the same thing.
- Keep every default path's rendered output identical: LTR, regular style,
  fractional widths on, tsume 0, protection on but no fallback needed.
- Ship as one OpenSpec change with phased commits so each issue can be reviewed
  and reverted independently.

**Non-Goals**

- A spelling engine. Language is a stored attribute only.
- Character ranges or selections: the type model is single-run, so character
  attributes and character styles apply to the whole type layer. This is an
  existing ceiling, not a new one.
- Full CS6 paragraph layout: indents, space before/after, hanging punctuation,
  hyphenation, and composer remain as they are.
- Exact Adobe tsume math or PSD key names without an oracle; these are
  documented approximations.
- A general font-management UI; the font registry stays an internal cache.
- Mixed-face PDF live text: a layer that needs per-character fallback
  rasterizes in PDF instead of emitting one font per run.

## Decisions

### 1. Model and bridge additions

Add `CharacterAttrs.tsume: f64` (0–100), `ParagraphAttrs.direction: TextDirection`,
and carry the existing `language` field end to end (the panel exposes it and the
author writes it; the model and reader already have it). `TextDirection` is a
small enum (`Ltr`, `Rtl`) with `Default = Ltr`; the bridge maps it to `i32` like
the other enums. `style_character_overrides`, `character_setting`,
`paragraph_attrs`, and `paragraph_setting` gain the tsume/direction fields (the
language field is already mapped), and `TextStyleSheet::resolve` merges them
through the existing sparse `CharacterOverrides` / `ParagraphOverrides`.

Alternative: keep direction in the bridge only. Rejected because the codec and
layout need it on the document model, and "inert bridge field" is the bug being
fixed.

### 2. Face registry keyed by family and style

`pictura-render::fonts` changes `Registered` to carry `family` and `style` and
the registry to key on `family|style` (normalized) plus each face's PostScript
name, backed by an insertion-ordered list so fallback lookup is deterministic.
`register_font(family, style, bytes)` and
`face_for(family, style) -> Option<&'static BundledText>` resolve:

1. the exact family+style entry,
2. the family's regular entry,
3. the bundled face.

An empty style means the family's regular entry. A separate ordered fallback
list is replaced when the active family changes, so fallback never accumulates
document history; it starts with the family's regular face when that differs
from the active style face. `opentype_features`,
`font_has_outlines`, `font_postscript_name`, and `font_family` take the style
too (or an explicit regular default) so the panel's OpenType greying, Warp Text,
and PDF embedding follow the selected face, not the family's regular face.

The app's `typeFontBytes(family, style)` builds a `QFont` with
`setStyleName(style)` (falling back to weight/italic mapping when the style name
is not found), and `registerTypeFont(family, style)` caches on the family+style
key. The Character panel's family change registers the regular face and
populates the style list from `QFontDatabase::styles(family)`; a style change
registers that face. Opening a type layer or a new-type session registers the
layer's family+style before its first render, so a document opened with Bold
text does not silently fall back to Regular.

Alternative: register every style of a family up front. Rejected: it loads and
keeps every face for the process even when unused.

### 3. Per-glyph face identity and fallback

`ShapedGlyph` gains `face: u16`, `cluster: u32` (the logical byte offset of the
glyph's source character), and `x_scale: f32`. `PlacedGlyph` carries the same
`face`, `cluster`, and `x_scale` so layout does not discard them. `TextLayout`
carries the palette as opaque `face` ids; the renderer owns the corresponding
`Vec<&'static BundledText>`, keeping `pictura-core` font-agnostic.
`fonts::faces_for(family, style)` resolves the active face plus its ordered
fallback faces once per layout, and `shape_styled` takes that face set (not one
face) and stamps each glyph with its palette index. `layout_type` returns the
layout together with its palette, or a render-side wrapper that carries both, so
`paint_run` and the path/warp rasterizers can resolve each glyph's face.
`PathGlyph` carries the face index too. `shape_styled` becomes a run splitter:

1. Order the line by bidi (decision 4), producing directional runs.
2. Within each directional run, split by coverage of the active face when
   Missing Glyph Protection is on; each uncovered character goes to the first
   face in the active family's ordered fallback list whose `cmap` covers it; if
   none, the active face. A run's `RunStyle` is computed from the face that will
   shape it, so `smcp` and other face-dependent features stay correct.
3. Shape each run with its face and direction; offset each glyph's cluster by
   the run's start and by any case-piece start, then concatenate in visual
   order. A case piece keeps the source byte range; where a transformation
   rewrites a character's length (upper-casing expanding one character), the
   piece's source start is the cluster ceiling rather than a guessed offset.

The fallback list is rebuilt deterministically for the active family instead of
accumulating process-wide document history: the app sorts candidate families
before registering them, and the renderer consults only that list for fallback.
The renderer never registers fallbacks itself; only the app and tests call
`register_font`, tests in a fixed order. Fallback faces are registered by the
app, never by the renderer: a bridge call `type_register_type_fallbacks(view)`
(called on document open and when a type session opens) scans the type text,
asks the renderer's active face which characters it lacks, groups them by script,
and asks Qt for one sorted-first installed family per script, iterating the
scripts in sorted order, through the existing registration path. A per-keystroke check registers the covering family
for a newly typed character before the preview renders. The renderer's policy is
purely registry-based and deterministic, so engine tests can register the bundled
Noto Sans as a fallback for a codepoint Liberation Sans lacks (for example
U+0370) without Qt.

Every laid-out consumer uses the glyph's face: `paint_run`, the warped-outline
path in `type_warp`, and the on-path rasterizer in `type_path`. A path or warp
that mixes faces therefore measures and paints the same face per glyph.

Alternative: the renderer calls back into Qt for each uncovered character.
Rejected: it crosses the crate boundary, is slow per glyph, and makes the
render crates untestable without a GUI.

### 4. Bidi and paragraph direction

Add `unicode-bidi` (UAX #9). `layout_type` runs `BidiInfo::new(line, base)` and
`visual_runs`; each visual run shapes with `rustybuzz::Direction::RightToLeft`
when its level is odd, otherwise left-to-right, through `shape_with`, which
gains a direction argument (today it hardcodes LTR). The paragraph's direction
sets the base level. Line order, alignment, and advance are computed from the
visual runs. In an RTL line, logical line start maps to the visual right edge
and line end to the visual left edge; alignment still places the line by its
named edge, so a left-aligned RTL line starts at the box's left and its first
logical character sits at that line's right end. The caret uses the glyph
`cluster` values to map visual positions back to logical boundaries (decision 3
gives it the data), and the baseline guides run from the line's visual start to
its visual end.

Alternative: reverse glyph order heuristically for RTL. Rejected: it breaks
mixed Latin/Arabic text and caret indexing, and the crate is small and pure Rust.

### 5. Justification and auto leading

`type_justify` distributes the leftover width in CS6 order:

1. word spaces: each space's normal advance times the Desired factor (the
   model's `1.0` is normal), clamped to Minimum/Maximum; apply as much of the
   leftover as the clamp allows;
2. letter spacing: `size × value` between glyphs, where the model value is the
   fraction (`0.05` is 5%), clamped to Minimum/Maximum;
3. glyph scaling: a horizontal `x_scale` starting at Desired, solved to consume
   the remainder and clamped to Minimum/Maximum (defaults 50–200%).

Desired values apply to every line; Minimum and Maximum only bound a justified
line that has leftover width to spread. The algorithm records how much each
step consumed and leaves the remainder ragged when all three are clamped.
`ShapedGlyph` gains `x_scale` (separate from the uniform `scale` used by
synthesized small caps); `PlacedGlyph` carries it and the rasterizer applies it
horizontally to the advance and outline. Wrapping uses the post-spacing
advances, so the break points and the painted line agree. `leading_px` takes the
paragraph's `auto_leading` instead of the hardcoded 120%; every caller that
measures a line (`layout_type`, `text_box_any`, `type_caret`, `type_paragraph`)
passes it through. The bridge already carries it.

The Paragraph panel's Justification dialog reuses the Paragraph Style Options
justification page's three-column Min/Desired/Max layout and Auto Leading field
(factored into one widget); it writes the same `ParagraphSetting` fields.

Alternative: scale the whole line uniformly. Rejected: it violates the bounds and
CS6's order, and glyph scaling is a distinct control.

### 6. Fractional widths and tsume

Fractional Widths off rounds each shaped advance, the per-glyph tracking step,
and offsets to whole pixels before wrapping and caret measurement. The flag is
layer-wide, so it is read from the character attributes in `shape_styled`; the
Character panel menu carries a checkable `Fractional Widths` item bound to the
active layer's value.

Tsume is applied to CJK and full-width punctuation glyphs as
`advance -= (tsume / 100) × 0.5 × size`; this is an approximation of CS6's
character-side compression and is recorded as such. The CJK/full-width predicate
is a Unicode East Asian Width query, so it is testable without a CJK face: the
unit test feeds a synthetic full-width advance through the same predicate and
adjustment. It is applied after shaping and before justification. At 100% the
advance shrinks by half an em; the renderer does not reorder glyphs.

Alternative: apply tsume as tracking. Rejected: it would move every glyph,
including Latin, and tsume is character-side spacing.

### 7. Codec keys

The author writes `/Tsume` (double), `/Language` (string), and the paragraph
direction as `WritingDirection` (0 LTR, 1 RTL) in `/Rendered /Shapes` and its
`Lines` cache. The existing `type_shape::rendered` hardcodes 0, so the change
parameterizes it. The reader extracts tsume, language, and the shape's
`WritingDirection` with the documented defaults when absent; language resolves
through the style-sheet default like the other character keys, not only the
first run.

The style and paragraph merges preserve unknown keys because they overwrite only
modelled keys in the parsed tree. `/Rendered` is rebuilt from the model, not
merged, so unmodelled keys inside that shape do not survive a re-author; this is
a documented ceiling, not a claim of full-tree preservation. Because the reader
now decodes direction, re-authoring an unrelated attribute writes the same
direction back.

`/Tsume` and the string form of `/Language` are best-effort: CS6's exact tsume
key and language encoding are not grounded in an oracle, so the round-trip test
is our codec's, and the limitation is recorded alongside the existing
EngineData ceilings.

Alternative: store tsume/direction only in memory. Rejected: saving would
silently drop a user's setting.

### 8. Character Styles panel

Add `type_character_style_count`, `_name`, `_font`, and `_character` bridge
calls, all resolving through `TextStyleSheet::resolve(..., Some(name), None)` so
a sparse style reports document defaults for unset fields. Add
`type_preview_character_style` (edit without history, re-resolve every applying
layer, recomposite) so the dialog's Preview works like the paragraph dialog's.
`type_edit_character_style` gains a `new_name` argument: it renames through
`TextStyleSheet::rename_character_style` and rewrites every layer's
`applied_character_style` reference from the old name to the new one, then
re-resolves those layers, all under one history state. The panel mirrors
`ParagraphStylesPanel` (list, click apply, double-click edit, footer
create/delete, delete enabled only with a row selected). The options dialog
reuses `ParagraphStyleDialog` in a character-only mode: title "Character Style
Options", the three character pages only, and the paragraph setting it produces
is the default and is ignored. A new icon is vendored from Lucide (or drawn in
the Lucide style) under the command id `window.panels.characterStyles` and
registered in the qrc/provenance mapping.

Alternative: a separate `CharacterStyleDialog`. Rejected: it duplicates three
pages of widget code for no behavior difference.

### 9. Preferences are process-wide

`Enable Missing Glyph Protection` lives in `pictura-render` as a process-wide
setting (default on) alongside the text-gamma setting, with a bridge setter/getter; the setter
triggers a re-render. `Show Asian Text Options` and `Middle Eastern` live
in the app's session/frame state because they only govern panel visibility; the
Preferences signals push them to the Character and Paragraph panels. All three
are saved in the session store and restored at startup before the first document
renders; the glyph-protection value is pushed to the renderer at startup. None
is stored on the document and none records history.

Alternative: store the settings per document. Rejected: CS6 treats them as
application preferences, and the panel visibility is global.

## Risks / Trade-offs

- **Ungrounded PSD keys** → `/Tsume` and `/Language` may not be readable by
  Photoshop even though our codec round-trips them. Mitigation: mark the
  approximation in the parity register, keep the style/paragraph unknown-key
  merge, and do not claim oracle parity.
- **Mixed-face layout drift** → wrapping, caret, and ink can disagree if any
  path still measures a single face. Mitigation: all measurement goes through
  the shaped layout with face-aware advances; add unit tests that wrap a mixed
  line and compare the caret's line end with the painted ink.
- **Bidi dependency** → `unicode-bidi` is a new dependency and mixed-direction
  tests need a font with the script. Mitigation: Liberation Sans covers Hebrew,
  so RTL and mixed-run tests use the bundled faces; no C dependency is added.
- **Fallback font availability** → CI may lack CJK fonts. Mitigation: engine
  tests use the two bundled faces for fallback (Noto Sans covers codepoints
  Liberation lacks, e.g. U+0370); tsume is unit-tested on a synthetic full-width
  advance because neither bundled face carries CJK, and the Qt-dependent fallback
  registration is tested with whatever the host provides or marked manual.
- **Mixed-face PDF live text** → PDF export has one face per live-text run.
  Mitigation: a layer that needs fallback rasterizes in PDF instead of emitting
  live text; this is listed as a non-goal, not silently broken.
- **Bundle size** → four issues in one change. Mitigation: phased commits in the
  draft's order (model/codec, #517, #519, #518, #887); each phase keeps the tree
  green, so a phase can be reverted without touching the others.
- **Tsume parity** → the half-em approximation may look wrong for punctuation.
  Mitigation: keep the value model-level, document the approximation, and leave
  the exact rule for a later oracle-backed change.

## Migration Plan

No document migration: every new attribute defaults to the current behavior
(tsume 0, LTR, Regular style, fractional widths on, language `English: USA`).
Existing text whose active face lacks a glyph renders fallback faces under the
new default-on preference; turning the preference off restores the old output.
Opening and saving an existing PSD preserves unknown keys on the unchanged
open-save path; a re-author adds the new keys at their defaults. Rolling back the
change leaves documents readable because the new keys are additive and unknown
keys are preserved by the style/paragraph merge.
