# Type Tools

- **Spec ID:** `TOOL-050` (Horizontal/Vertical Type, point/paragraph/on-path type), `TOOL-051` (Type Mask tools, text engine, anti-aliasing)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds a dedicated **Type** menu, **Character Styles** and **Paragraph Styles** panels, `Type > Paste Lorem Ipsum`, `Type > Font Preview Size` (moved out of Preferences), `Type > Replace All Missing Fonts`, and a global **Blend Text Colors Using Gamma** text-compositing option. Core type entry (point, paragraph, on-path, mask) is unchanged from CS5.
- **Depends on:** `ARCH-008` document-model (`Type` node, `pictura_core::text`), `ARCH-002` rust-core-design, `ARCH-003` qt6-ui-design, `02-ui-ux/panels/character-and-paragraph.md`, `02-ui-ux/toolbox-and-options-bar.md`, `05-layers/layers-overview.md`, `05-layers/vector-masks-and-clipping-masks.md`, `03-tools/pen-and-path-tools.md`.

## CS6 behavior

Type in Photoshop CS6 is **vector-based text** stored on an editable **type layer**. The outlines are mathematically defined (Type 1/PostScript, TrueType, OpenType, New CID, CID nonprotected), so type stays crisp when scaled, saved to PDF/EPS, or printed to a PostScript device. A change that requires rasterization converts the type layer to pixels and ends text editability.

Three ways to create type (all with the Horizontal Type `T` or Vertical Type tool):

| Mode | How entered | Behavior |
|---|---|---|
| **Point type** | Click in the image | Independent horizontal/vertical line; expands/shrinks as edited, no wrapping. The I-beam's cross-line marks the baseline (for vertical type, the center axis). |
| **Paragraph type** | Drag a bounding box; or Alt/Option-click/drag to open the **Paragraph Text Size** dialog (Width/Height) | Lines wrap to the box; resize, rotate, skew the box and the type reflows. Overflow shows a `+` in the box handle. |
| **Type on a path** | Put the type baseline indicator on an open or closed path and click | Flows along the path in the direction anchor points were added. Horizontal type is perpendicular to the baseline; vertical type is parallel to it. Inside a closed path, type is always horizontal and breaks at path boundaries. Overflow shows a `+` at the path end. |

A third entry mode, **Type Mask** (Horizontal/Vertical Type Mask), creates a *selection* in the shape of the type instead of a type layer. A red mask overlays the active layer while typing; after Commit the type selection border appears on the active layer and behaves like any selection (move, copy, fill, stroke).

Type-layer editing rules (documented):

- Still editable type: change orientation, apply anti-aliasing, convert point↔paragraph, create a work path, apply Edit-menu transforms except Perspective/Distort, use layer styles, warp text.
- The tool enters **edit mode** on canvas; the options bar shows **Commit** and **Cancel** (Esc). `Ctrl+Enter` (Windows) / `Cmd+Return` commits. Clicking most other tools/panels also commits.
- Point type can be transformed while in edit mode by holding `Ctrl`/`Cmd` (a bounding box appears; scale, skew, rotate handles).
- `Type > Convert To Point Text` / `Type > Convert To Paragraph Text` (moved from `Layer > Type` in CS5). Converting paragraph→point deletes overflow characters — the Help warns to fit all text first.
- `Type > Create Work Path` and `Type > Convert To Shape` require outline font data; bitmap-only fonts cannot be converted.
- `Type > Warp Text` (CS6 location; `Layer > Type > Warp Text` in CS5) offers style, Horizontal/Vertical orientation, Bend, Horizontal/Vertical Distortion. Faux Bold and outline-less fonts cannot be warped.
- Rasterize via `Layer > Rasterize > Type` for filters/painting.
- Type layers are **not created** in Multichannel, Bitmap, or Indexed Color modes; type is rasterized onto the background.

### CS6 character/paragraph improvements

| Capability | CS6 behavior |
|---|---|
| **Type menu** | New top-level `Type` menu gathers text/type functionality; `Type > Panels` opens Character/Paragraph. |
| **Character Styles panel** | `Window > Character Styles`. Create/store/reapply character-level attributes. |
| **Paragraph Styles panel** | `Window > Paragraph Styles`. New documents start with a **Basic Paragraph** style. |
| **Style hierarchy** | Manual overrides supersede applied character styles, which supersede applied paragraph styles. |
| **Default type styles** | `Type > Save/Load Default Type Styles` is documented in the CS6 Help as **Creative Cloud only**, not base CS6; the CC-shaped build implements both from the Type menu and saves the defaults in the Type preferences. |
| **Font preview size** | `Type > Font Preview Size` (CS5 had this in the Type preferences). Font menus show live previews; font-kind icons distinguish OpenType, Type 1, TrueType, Multiple Master. |
| **Font selection** | Type-ahead: typing a name in the Font Family/Style field jumps to the first font/style beginning with those letters. Duplicate installed fonts are suffixed `(T1)`, `(TT)`, `(OT)`. |
| **Missing fonts / glyph protection** | An alert on open when fonts are missing; `Type > Replace All Missing Fonts` (in `Layer > Type` in CS5). **Enable Missing Glyph Protection** (Advanced Type preferences, on by default) auto-substitutes a font for non-Roman input. |
| **Language/script engine** | Type preferences expose **Middle Eastern**, **East Asian**, and **Show Font Names In English**. CS6 Help notes that Middle Eastern is preferred in CS6 over the older Show Asian Text Options for non-CJK Asian languages. |
| **Asian type** | Show Asian Text Options exposes Tsume (0–100%, character-side spacing compression), leading measurement options, and vertical underline left/right. |
| **Spelling / find-replace** | `Edit > Check Spelling`, `Edit > Find And Replace Text` (Search All Layers, Forward, Case Sensitive, Whole Word Only). |
| **Composition** | `Ctrl+Shift+Alt+T` toggles single-line / every-line composer; hyphenation and justification controls live in the Paragraph panel menu. |

The task brief lists "font search & filtering" as a CS6 type improvement. The CS6 Help documents **type-ahead** in the font field and **layer** filtering (by name/kind/effect/mode/attribute/color, including the `[T]` type-layer filter), but a dedicated *font* search/filter UI is not described in the CS6 Help. That gap is recorded under Open questions and must not be implemented as if sourced.

### Text engine and anti-aliasing

- Glyph edges are anti-aliased with one of **None / Sharp / Crisp / Strong / Smooth**, chosen in the options bar, Character panel, or `Layer > Type`.
- CS6 adds the global **Blend Text Colors Using Gamma** option (`Edit > Color Settings > More Options`, default **1.45**). It applies a gamma curve to the text layer's anti-aliased mask when compositing, so text looks thinner/fatter and text layers from CS6 look different in earlier Photoshop versions. It also changes the effective opacity of non-opaque text layers and can make text and shape layers of the same color/opacity mismatch unless disabled. This is a major, easily-missed CS6 text-rendering change.
- The Help notes that anti-aliased type may render inconsistently at small sizes/low resolutions and suggests deselecting **Fractional Width** in the Character panel menu to reduce it.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel: Horizontal Type / Vertical Type | Tool | `T` (Shift+`T` cycles) | Shares slot with Type Mask tools |
| Tools panel: Horizontal Type Mask / Vertical Type Mask | Tool | `T`, Shift+`T` | Creates a selection, not a layer |
| Type options bar | Tool bar | n/a | Orientation toggle, font family/style, size, anti-aliasing, alignment, color, warp, panels button, Create/Commit/Cancel |
| Character panel | Dock | n/a; `Type > Panels` | Character attributes |
| Paragraph panel | Dock | n/a; `Type > Panels` | Paragraph attributes |
| Character Styles panel | Dock | n/a | `Window > Character Styles` (CS6) |
| Paragraph Styles panel | Dock | n/a | `Window > Paragraph Styles` (CS6) |
| `Type` menu | Menu | n/a | New in CS6 |
| `Type > Panels` | Menu | n/a | Character / Paragraph / Character Styles / Paragraph Styles |
| `Type > Convert To Point Text` / `Convert To Paragraph Text` | Menu | n/a | CS6 location |
| `Type > Warp Text` | Dialog | n/a | CS5: `Layer > Type > Warp Text` |
| `Type > Create Work Path` | Menu | n/a | Outline fonts only |
| `Type > Convert To Shape` | Menu | n/a | Replaces layer with vector mask |
| `Type > Paste Lorem Ipsum` | Menu | n/a | New in CS6 |
| `Type > Font Preview Size` | Menu | n/a | Moved from Preferences (CS6) |
| `Type > Replace All Missing Fonts` | Menu | n/a | CS5: `Layer > Type` |
| `Type > Update All Text Layers` | Menu | n/a | Re-renders every type layer |
| `Layer > Rasterize > Type` | Menu | n/a | Destructive |
| `Edit > Check Spelling` | Dialog | n/a | |
| `Edit > Find And Replace Text` | Dialog | n/a | |
| `Edit > Color Settings > More Options` | Dialog | `Ctrl+Shift+K` | Blend Text Colors Using Gamma |
| `Edit > Preferences > Type` | Dialog | n/a | Middle Eastern / East Asian / missing-glyph / font names |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Font Family | string/combo | app default | installed families | Live previews; type-ahead |
| Font Style | string/combo | Regular | family-dependent | Faux Bold/Italic when absent |
| Font Size | number | last used (pt) | > 0; units pt/px/in/cm/mm/pica | PostScript 72 pt/in vs Traditional 72.27 pt/in preference |
| Leading | number | Auto | ≥ 0 (auto or fixed) | Distance baseline-to-baseline |
| Kerning | number/`Metrics`/`Optical`/`0` | Metrics | signed | Pair spacing |
| Tracking | number | 0 | signed | Uniform run spacing |
| Vertical Scale / Horizontal Scale | percent | 100 | > 0 | Distorts glyphs |
| Baseline Shift | number | 0 | signed | Also moves type on/off a path |
| Anti-aliasing | enum | Sharp? (Help does not state default) | None, Sharp, Crisp, Strong, Smooth | Options bar / Character panel |
| Color | color | foreground | document color mode | Applies to selected characters |
| Tsume | percent | 0 | 0–100 (100 = no em gap) | Asian type; requires Show Asian Text Options |
| All Caps / Small Caps / Superscript / Subscript / Underline / Strikethrough / Faux Bold / Faux Italic | toggle | off | on/off | Dynamic shortcuts in edit mode |
| Leading measurement (Asian) | enum | — | e.g. top/bottom | Paragraph panel menu |
| Alignment | enum | Left | Left/Center/Right (horizontal); Top/Center/Bottom (vertical) | Paragraph type only |
| Justification | enum | Left | Left/Center/Right/Justify last left/center/right/fully justify | Paragraph panel |
| Word/Letters/Glyph Scaling (justified) | percent | Auto | Min/Desired/Max | Paragraph panel |
| Indent Left / Right / First Line | number | 0 | signed (negative = hanging) | Paragraph panel |
| Space Before / After | number | 0 | ≥ 0 | Paragraph panel |
| Hyphenation | toggle + dict | off | on/off + zone/slider/limit | Paragraph panel menu |
| OpenType features | toggles | off | standard/discretionary ligatures, contextual alternates, swash, old style, stylistic/titling alternates, ornaments, ordinals, fractions | Character panel menu |
| Warp style | enum | None | Arc, Wave, Fish, etc. + Bend / H-Distortion / V-Distortion | `Type > Warp Text` |
| Blend Text Colors Using Gamma | number (toggle) | 1.45, on | gamma value | Color Settings; global |
| Font Preview Size | enum | Medium | None / Small / Medium / Large / Extra Large / Huge | Type menu (CS6) |

## Algorithms & pipeline

### Text shaping and layout (proposed)

Behavioral parity target: given the same string, font, size, tracking/kerning/leading/baseline settings, the laid-out glyph positions and the rasterized type layer must match CS6 within the screenshot tolerance of `11-cross-cutting/testing-strategy.md`. Adobe's exact outline rasterizer and gamma handling are closed; CS6 documents only the observable output, so this is **behavioral parity only, algorithm TBD** for the AA curve itself. The concrete engine design below is a proposal.

Pipeline:

1. **Resolve fonts** — enumerate system + bundled fonts; match family/style; detect missing glyph coverage; run fallback per script.
2. **Itemize / segment** — split text into runs by script, direction (bidi), font, and style.
3. **Shape** — map Unicode → glyph IDs and advance/offset positions, applying OpenType features (ligatures, alternates, kerning), and (where applicable) cursive joining and mark positioning. This is the HarfBuzz step.
4. **Line break** — apply Unicode line-breaking; for paragraph type, wrap to the bounding box; for point type, only explicit returns.
5. **Position lines** — apply leading, alignment/justification, indents, space before/after.
6. **Place on path** — map each glyph's advance-along-line distance to the path parameter, using arc length (CS6 flows in anchor-point order; glyphs are normal/perpendicular to the baseline depending on orientation).
7. **Rasterize / outline** — produce a coverage mask per glyph (this is where the AA presets and the 1.45 gamma blend apply), or emit outlines for `Create Work Path` / `Convert To Shape`.
8. **Composite** — blend the type layer mask into the layer stack per the document model; apply the global gamma option.

Notes:

- CS6's Type-on-path and work-path conversion imply the layout must be reversible: layout → glyph outline → path is required for `Create Work Path`, and path → per-glyph x → glyph placement is required for type-on-path.
- Anti-aliasing presets are behavioral (blur/coverage shaping), not a documented formula. Treat "None/Sharp/Crisp/Strong/Smooth" as named output profiles to be calibrated against references, not hard-coded algorithms.
- `Blend Text Colors Using Gamma` is a compositing-time curve on the text mask (dark text stays dark, white stays white, gray AA pixels change contrast); default 1.45. Implement as a configurable mask LUT, defaulting to 1.45 but off-switchable for cross-app match.

### Relationship to existing modules

`ARCH-008` already proposes `pictura_core::text` for "text engine data, glyph runs, paragraph settings." This spec refines that into a shaping-focused crate (see Rust module mapping) and defines the CS6 attributes the node must carry.

## Rust module mapping

Proposed crate `pictura-text` (new; depends on `pictura-core`):

- `pictura_text::font` — font discovery/matching over `fontdb` (system font scan, CSS-like family/style/weight/stretch queries, fallback per script). Key types: `FontDb`, `FaceId`, `FontQuery`, `ResolvedFont`.
- `pictura_text::shape` — HarfBuzz-compatible shaping via `rustybuzz` (a pure-Rust port of HarfBuzz) or a `harfbuzz` C binding if a script requires the reference engine. Key types: `ShapedRun { glyphs: Vec<ShapedGlyph>, font: FaceId }`, `ShapedGlyph { gid, cluster, advance, offset, flags }`.
- `pictura_text::layout` — itemization, bidi (`unicode-bidi`), line breaking (`unicode-linebreak`, `unicode-segmentation`), paragraph/point boxes, alignment/justification/indents. Key type: `TextLayout { lines: Vec<LaidLine>, runs: Vec<PlacedRun>, box_size, overflow }`.
- `pictura_text::path` — type-on-path mapping (glyph × arc-length → position + tangent), and outline extraction (`ttf-parser`/`skrifa` glyph outlines → `kurbo::BezPath`).
- `pictura_text::render` — glyph rasterization/coverage masks via `swash` (hinting, subpixel offsets, synthetic bold/oblique); named AA profiles; gamma LUT. Key type: `GlyphMask`.
- `pictura_text::style` — `CharacterStyle`, `ParagraphStyle`, `TextStyleSheet`, and the manual > character > paragraph override resolution.
- `pictura_text::state` — `TextEditState` for editing sessions (selection, caret, composition/preedit, commit/cancel).

Crossing types: `pictura_core::text::TextRun` (glyphs + placement) and `pictura_core::text::TextSettings` (character/paragraph attributes) cross into the document model; `kurbo::BezPath` crosses into `pictura-vector` for work paths and converted shapes. No Qt types cross into this crate.

Crates verified to exist: `rustybuzz` 0.20 (pure-Rust HarfBuzz port), `fontdb` 0.24, `swash` 0.2, `kurbo` 0.13. `harfbuzz_rs`/`harfrust` are candidate alternative bindings but were not verified in this pass.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `TextToolHandler` | `QObject` | Pointer/keyboard state machine for point/paragraph/on-path/mask entry; edit mode; Commit/Cancel |
| `TypeOptionsBar` | `QWidget` | Orientation toggle, font combo, size, AA menu, alignment, color, warp button, panels button |
| `FontComboBox` | `QFontComboBox`/`QComboBox` | Family with previews, type-ahead, OpenType/T1/TT icons over `pictura-text` font data |
| `CharacterPanel` | `QWidget` | Character attributes; edits commit through commands |
| `ParagraphPanel` | `QWidget` | Paragraph attributes and justification |
| `TypeStylesModel` | `QAbstractItemModel` | Character/Paragraph Style entries; create/rename/reapply |
| `ParagraphStylesModel` | `QAbstractItemModel` | Paragraph Style list with Basic Paragraph default |
| `TextEditOverlay` | `QGraphicsObject` | On-canvas caret, selection highlight, bounding box, overflow markers; drawn as vector overlay |
| `TextOnPathOverlay` | `QGraphicsPathItem` | Baseline indicator and glyph placement preview over a path |
| `WarpTextDialog` | `QDialog` | Warp style, orientation, Bend/H/V Distortion |
| `BlendTextGammaControl` | `QWidget` | Color Settings "More Options" gamma control |

Widgets (not QML) is consistent with `ARCH-003`. The interactive text editor is a custom canvas overlay widget, not a `QTextEdit`: the document text model lives in Rust (`pictura-text`) and Qt supplies hit-testing/caret painting through `QTextLayout` results or a mapping shim. `QFontDatabase`/`QFont` are used only to enumerate and select application fonts, not as the authoritative shaping engine.

## Data-model impact

- `ARCH-008`'s `NodeKind::Text` carries a `pictura_core::text` payload. This spec requires it to hold: raw string, run list with per-run character attributes, paragraph list with paragraph attributes, point-vs-paragraph mode, bounding box, type-on-path reference (`PathId` + start offset/baseline shift/side), warp parameters, and orientation.
- Type layers round-trip to PSD text-engine data (the `TySh`-style additional-layer block) — exact binary layout is not sourced here and is a noted open question in `ARCH-008`.
- Character/Paragraph styles are document-level (and optionally saved as defaults). Where CS6 serializes them (PSD image resource, additional-layer info, or XMP) is not sourced; must be preserved as opaque data until resolved.
- `Blend Text Colors Using Gamma` is document-color-setting state; store as a document/app setting, not per-layer.
- Undo granularity: character/paragraph attribute edits and text edits should be one command per committed edit session (keystroke coalescing), matching CS6's type-layer undo behavior. Style application is one command.
- `Create Work Path` / `Convert To Shape` are structural commands (produce a path node / replace the type node with a shape node) and need the pre-conversion type payload for undo.

## Edge cases

- **Unsupported modes.** No type layers in Multichannel/Bitmap/Indexed; type rasterizes to the background.
- **Font availability.** Missing fonts alert on open; missing-glyph protection substitutes automatically; outline-less (bitmap) fonts cannot become work paths/shapes and cannot be warped.
- **Overflow.** Paragraph `+` handle and on-path end `+` marker; converting paragraph→point deletes overflow — must warn/preserve for undo.
- **Empty type layer.** Zero-length text should not crash layout or path conversion; a Work Path from empty type is empty.
- **Type menu implementation ceilings.** Outlines use the engine's single-run type model; area-type outlines are not clipped to the area path; warped outlines are polylines; find/replace folds ASCII case only and treats alphanumerics/underscore as word characters; the solid-fill writer stores an opaque fill.
- **Type on a closed path.** Always horizontal, breaks at boundaries; flipping across the path changes side and reverses reading direction.
- **RTL / bidi / complex scripts.** Arabic/Hebrew/Indic shaping, contextual forms, Kashida justification, ligatures, diacritics; CS6 documents Middle Eastern and East Asian engines separately. Shaping must not assume LTR.
- **32-bit / HDR.** The Help notes the Text tool can add 32-bpc type layers to an HDR image.
- **CMYK/Lab.** Color must resolve through the document color space; AA mask is independent of mode.
- **Huge documents / PSB.** Bounding boxes up to 300,000 px; do not enumerate per-glyph geometry for off-canvas runs.
- **GPU unavailable.** Text rasterization is CPU (coverage masks) and composes through the CPU fallback path.
- **CUDA/gamma.** With Blend Text Colors Using Gamma off, CS6 compatibility with CS5 documents is expected; with it on, output deliberately differs. This must be a documented, testable divergence.
- **Undo/redo.** Uncommitted edit sessions must be cancellable without mutating history (Cancel button).

## Parity acceptance criteria

1. Given a font, size, and string, `pictura_text::shape` produces glyph IDs and advances that match HarfBuzz for the same font/features within one font unit (HarfBuzz is the reference the port claims to implement).
2. Given point type, the rendered line does not wrap; given paragraph type, text wraps to the box and an overflow marker appears exactly when the text exceeds the box.
3. Given a path drawn left-to-right (anchor order), horizontal type flows along it with glyphs perpendicular to the baseline, and flipping the type across the path reverses the side; vertical type is parallel to the baseline.
4. Given `Type > Create Work Path` on a text layer, the resulting path's outline bounds match the rendered type bounds within one pixel at a reference size.
5. Given an OpenType font with standard ligatures, enabling the feature replaces the input clusters with the ligature glyph while keeping clusters editable and not flagged by the spell checker.
6. Given AA setting `None`, glyph edges are hard (no semi-transparent edge pixels within the fill); `Smooth` produces the widest coverage falloff of the five presets, ordered as documented.
7. Given Blend Text Colors Using Gamma at default 1.45, text-layer AA gray values are remapped by the configured gamma; disabling it reproduces the pre-CS6 (CS5-style) blend for the same document.
8. Given a type layer, re-saving to PSD and re-opening preserves editability and all character/paragraph/style attributes for the fonts present.
9. Given a document with missing fonts, opening reports them and `Replace All Missing Fonts` remaps affected layers without dropping text.
10. Given a paragraph/character style hierarchy, a manual override beats an applied character style, which beats an applied paragraph style, for every attribute.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official Photoshop CS6 Help (757 pp; text extracted via `pdftotext`). Established: tool set and shortcuts; point/paragraph/on-path/mask entry; commit/cancel and edit-mode rules; unavailable modes; anti-aliasing presets; `Type` menu and CS6 menu locations (`Convert To Point/Paragraph Text`, `Warp Text`, `Create Work Path`, `Convert To Shape`, `Replace All Missing Fonts`, `Update All Text Layers`); `Paste Lorem Ipsum`; `Font Preview Size`; Character/Paragraph panel contents and options; character/paragraph styles and the manual>character>paragraph hierarchy; fonts/type-ahead/preview icons; missing-fonts and glyph protection; OpenType features; `Blend Text Colors Using Gamma` default 1.45 (JDI list, p.7) and its format-compatibility note; Type preferences (Middle Eastern / East Asian / Show Font Names In English); Tsume and Asian leading; spelling and find/replace.
- `https://bjango.com/articles/photoshopcs6text` — independent account of CS6's **Blend Text Colors Using Gamma** (location under Color Settings > More Options, effect on AA and layer opacity, CS5/CS6 document divergence, advice to disable).
- `https://docs.rs/rustybuzz` — `rustybuzz` 0.20, "a complete harfbuzz shaping algorithm port to Rust"; API surface (`shape`, `Face`, `UnicodeBuffer`, `GlyphBuffer`, `Feature`, `Direction`, `Script`).
- `https://docs.rs/fontdb` — `fontdb` 0.24 font database, CSS-like queries, system-font scanning, no built-in fallback.
- `https://docs.rs/swash` — `swash` 0.2 shaping (`shape`) and glyph scaling/rasterization (`scale`), `FontRef`, `Features`.
- `https://docs.rs/kurbo` — `kurbo` 0.13 2D curves, `BezPath`, `Affine`, path segmentation.
- `https://doc.qt.io/qt-6/qtextlayout.html` — Qt 6 text layout, line breaking, cursor positions, `glyphRuns`, `FormatRange`.
- `https://doc.qt.io/qt-6/qfontdatabase.html` — font enumeration, family/style matching, writing systems, application fonts, fallback families.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+type+tool+Horizontal+Vertical+Type+Mask+type+on+path+options+bar` and `https://search.brave.com/search?q=Photoshop+CS6+font+search+filter+Character+panel+Type+menu+font+preview+size` — discovery queries locating the secondary sources above; search-result snippets were used only to find URLs, not as assertions.

## Open questions

- **True font search/filtering in CS6.** The task brief names it; the CS6 Help documents only type-ahead and layer filtering. *Resolves with:* a CS6 screenshot/Help page for the font menu, or a correction to the feature list in `OVR-002`.
- **Default anti-aliasing preset.** The Help lists the options but not the default (widely reported as Sharp). *Resolves with:* the Type preferences default or a clean CS6 install reference.
- **Exact PSD serialization of type-engine data and type styles.** The `TySh`/additional-layer layout and where Character/Paragraph styles are stored are not sourced. *Resolves with:* the Adobe File Formats Specification's type keys and a CS6-authored PSD corpus.
- **Blend Text Colors Using Gamma algorithm.** The curve is described qualitatively, not as a formula. *Resolves with:* calibrated reference renders across gamma values and mask levels.
- **Anti-aliasing preset implementation.** No public coverage/hinting algorithm is documented. *Resolves with:* the screenshot-diff harness in `11-cross-cutting/testing-strategy.md`.
- **Middle Eastern vs East Asian engine selection.** CS6 exposes a choice; how it changes shaping (bidi, joining, Kashida) is only partially documented. *Resolves with:* the CS6 Help Middle Eastern/Hebrew/Arabic sections and reference text.
- **Font fallback order across scripts** is not specified by CS6 docs. *Resolves with:* `fontconfig` behavior comparison plus reference renders.
- **`Type > Save/Load Default Type Styles` is Creative Cloud only in the Help; the CC-shaped build implements it as an owner decision.**
