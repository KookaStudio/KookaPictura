# Character and Paragraph Panels (and Type Styles)

- **Spec ID:** `PAN-022`
- **Status:** `Draft`
- **Parity tier:** `Core` — Character, Paragraph, Character Styles, and Paragraph Styles panels and the type options bar are in both CS6 editions.
- **New in CS6:** `Changed` — the Character and Paragraph panels are carried from CS5, but CS6 adds the **Character Styles** and **Paragraph Styles** panels (`Window > Character Styles`, `Window > Paragraph Styles`), gathers type commands into the new **Type** menu, and moves **Font Preview Size** out of Preferences. The `Type > Panels` submenu opens all four panels. The panels themselves add no new attribute controls.
- **Depends on:** `03-tools/type-tools.md` (`TOOL-050`/`TOOL-051` — text engine, shaping, anti-aliasing, styles semantics; this panel spec is the **UI contract only**), `02-ui-ux/toolbox-and-options-bar.md` (options bar host), `01-architecture/qt6-ui-design.md` (`ARCH-003`), `01-architecture/document-model.md` (`ARCH-008`, `pictura_core::text`), `01-architecture/undo-history.md` (`ARCH-009`).

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Panel controls and ranges are from the fetched CS6
> Help PDF unless marked *(secondary)* or *(inferred)*. The text engine/algorithm
> details are deliberately **not** duplicated here; see `03-tools/type-tools.md`.

## CS6 behavior

### Character panel

The Character panel formats characters, and some of the same formatting options are also available from the options bar. Open it with `Window > Character`, the Character panel tab, or (with a type tool active) the options-bar **Panel** button.

The CS6 figure labels eleven controls:

| Key in CS6 figure | Control |
|---|---|
| A | Font Family |
| B | Font Size |
| C | Vertical Scale |
| D | Set Tsume *(only with Show Asian Text Options)* |
| E | Tracking |
| F | Baseline Shift |
| G | Language |
| H | Font Style |
| I | Leading |
| J | Horizontal Scale |
| K | Kerning |

Numeric fields accept typed values with: `Enter`/`Return` to apply; `Shift+Enter`/`Shift+Return` to apply then highlight the value; `Tab` to apply and move to the next field. The panel menu (triangle, top-right) holds additional commands, including display and **Dynamic Shortcuts**.

**Dynamic Shortcuts** are keyboard shortcuts available only while entering/editing point or paragraph type and are listed in the Character panel menu: **Faux Bold, Faux Italic, All Caps, Small Caps, Superscript, Subscript, Underline, Strikethrough**.

Behavioural notes sourced from the CS6 text:

- **Size** defaults to points; PostScript (72 pt/in) vs. traditional (72.27 pt/in) point definitions are switchable in `Edit > Preferences > Units & Rulers`. Alternate units (`in, cm, mm, pt, px, pica`) can be typed into the size box.
- **Leading** is baseline-to-baseline; the largest leading value on a line determines that line's leading. Auto-leading percentage is set via `Justification` in the Paragraph panel menu.
- **Kerning** options are **Metrics** (font kern pairs; default), **Optical** (shape-based), and a manual value. `Alt`/`Option`+`←`/`→` decreases/increases the pair kerning; setting Kerning to `0` turns it off.
- **Tracking** is the uniform run spacing; tracking and kerning are both in **1/1000 em** and are cumulative.
- **Baseline Shift** moves characters up (positive) or down (negative) relative to the surrounding baseline.
- **Fractional Widths** is on by default; `System Layout` / `Fractional Widths` in the Character panel menu toggles whole-pixel spacing, and the setting applies to the **whole type layer**, not selected characters.
- **Underline** has a button for horizontal type and **Underline Left**/**Underline Right** menu entries for vertical type; **Strikethrough** is also in the panel menu.
- **Color** is set from the options bar or the Character panel color box (Adobe Color Picker) and applies to selected characters.
- **Language** selects the spelling dictionary.

### Paragraph panel

The Paragraph panel changes the formatting of columns and paragraphs. Open with `Window > Paragraph`, the Paragraph tab, or the type-tool options-bar Panel button. The CS6 figure labels: **A** Alignment and justification, **B** Left indent, **C** First-line left indent, **D** Space before paragraph, **E** Hyphenation, **F** Right indent, **G** Space after paragraph.

- **Alignment** (paragraph type only): horizontal **Left / Center / Right**; vertical **Top / Center / Bottom**.
- **Justification**: horizontal **Justify Last Left / Justify Last Centered / Justify Last Right / Justify All**; vertical **Justify Last Top / Justify Last Centered / Justify Last Bottom / Justify All**. Justification affects Roman characters only (not double-byte CJK).
- **Justification dialog** (Paragraph panel menu): **Word Spacing** (0–1000%; 100% = no added space), **Letter Spacing** (−100–500%; 0% = no added space), **Glyph Scaling** (50–200%; 100% = no scaling). Min/Max apply to justified paragraphs; Desired applies to both.
- **Indents**: **Indent Left Margin**, **Indent Right Margin**, **Indent First Line** (negative = hanging first line); vertical type maps left/top and right/bottom respectively.
- **Space Before/After Paragraph**.
- **Roman Hanging Punctuation** (panel menu) lets listed punctuation (quotes, apostrophes, commas, periods, hyphens, dashes, colons, semicolons) hang outside the margin; the hanging margin follows the paragraph alignment.
- **Hyphenation** (panel menu) and avoid-breaking-word options.
- **Composition methods**: single-line vs. every-line composer, toggled with `Ctrl+Shift+Alt+T` (`03-tools/type-tools.md`).
- **Paragraph direction** (LTR/RTL) is selected from the Paragraph panel when a Middle Eastern engine is enabled.

### Character Styles and Paragraph Styles panels (new in CS6)

- **Character Styles** (`Window > Character Styles`) stores character-level attributes and applies them to characters, a paragraph, or a range of paragraphs.
- **Paragraph Styles** (`Window > Paragraph Styles`) stores character *and* paragraph attributes. Each new document contains a **Basic Paragraph** style that is applied to new text; it can be edited but **not renamed or deleted**; user-created styles can be renamed and deleted, and a different default style can be chosen.
- **Hierarchy (sourced):** manual overrides take precedence over applied character styles, which in turn override applied paragraph styles.
- Create a style from selected text via **New Character Style** / **New Paragraph Style**; create without selecting text via the panel's **Create New Style** icon (select an image layer such as Background to edit a style without applying it). Double-click a style to edit it; editing updates all text using that style.
- Styles can be saved/loaded as **default type styles**, but the fetched CS6 PDF labels **`Type > Save Default Type Styles` / `Load Default Type Styles` as Creative Cloud only**, not base CS6 (see `03-tools/type-tools.md` Open questions).

### Type options bar

*(secondary, PSforPhotographers CS6)* With a type tool active, the options bar shows, left to right: a saved **type tool preset**, an **orientation** toggle (horizontal/vertical), **font family** (WYSIWYG list with type-ahead), **font style**, **font size**, **anti-aliasing** (None/Sharp/Crisp/Strong/Smooth), **paragraph alignment**, a **Warp** button, and a **Panel** button that opens the Character and Paragraph panels. In edit mode the options bar also shows **Commit** and **Cancel** (see `03-tools/type-tools.md`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Character` | Menu → dock | — | Character panel |
| `Window > Paragraph` | Menu → dock | — | Paragraph panel |
| `Window > Character Styles` | Menu → dock | — | CS6 |
| `Window > Paragraph Styles` | Menu → dock | — | CS6; Basic Paragraph default |
| `Type > Panels` | Menu | — | Opens the four type panels (CS6) |
| Type options bar — Panel button | Button | — | Opens Character + Paragraph |
| Character panel menu | Menu | — | Dynamic Shortcuts, Fractional Widths/System Layout, Underline Left/Right, OpenType, etc. |
| Paragraph panel menu | Menu | — | Justification, Hyphenation, Hanging Punctuation, composition |
| Style panel menu / Create New Style icon | Menu / button | — | New/Rename/Delete/Edit style |
| Type options bar | Bar | — | Preset, orientation, font, size, AA, alignment, warp, panels |
| Canvas | Type edit overlay | `Ctrl+Shift+Alt+T` (composer) | Caret/selection/bounding box (`type-tools.md`) |

## Parameters & ranges

### Character

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Font Family | combo | app default | installed families | WYSIWYG, type-ahead |
| Font Style | combo | Regular | family-dependent | Faux Bold/Italic when absent |
| Font Size | number | last used | > 0; `pt/px/in/cm/mm/pica` | PostScript vs. traditional point pref |
| Leading | number/combo | Auto | auto or fixed ≥ 0 | Baseline-to-baseline; largest value wins |
| Kerning | combo/number | Metrics | Metrics / Optical / 0 / signed | 1/1000 em; `Alt`+arrow |
| Tracking | number | 0 | signed | 1/1000 em; cumulative with kerning |
| Vertical Scale | percent | 100 | > 0 | Glyph distortion |
| Horizontal Scale | percent | 100 | > 0 | Glyph distortion |
| Baseline Shift | number | 0 | signed | Up/down; also type-on-path offset |
| Anti-aliasing | enum | not stated by Help (widely Sharp) | None / Sharp / Crisp / Strong / Smooth | Options bar and Character panel |
| Color | color | foreground | document color mode | Selected characters |
| Set Tsume | percent | 0 | 0–100 | Asian type; needs Show Asian Text Options |
| Language | enum | not stated | installed dictionaries | Spelling |
| Faux Bold / Italic, All Caps, Small Caps, Super/Subscript, Underline, Strikethrough | toggles | off | on/off | Dynamic Shortcuts |
| Fractional Widths | bool | on | Fractional Widths / System Layout | Whole layer |
| Auto Leading % | number | 120% *(not stated in CS6 text)* | % | Paragraph panel menu → Justification |

### Paragraph

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Alignment | enum | Left | Left/Center/Right; Top/Center/Bottom (vertical) | Paragraph type only |
| Justification | enum | Left | Justify Last Left/Centered/Right/All (+ vertical) | Roman only |
| Word Spacing | percent | 100 (Desired) | 0–1000 | Min/Desired/Max |
| Letter Spacing | percent | 0 (Desired) | −100–500 | Min/Desired/Max |
| Glyph Scaling | percent | 100 (Desired) | 50–200 | Min/Desired/Max |
| Indent Left/Right Margin | number | 0 | signed | Vertical maps top/bottom |
| Indent First Line | number | 0 | signed (negative = hanging) | Relative to left/top indent |
| Space Before/After Paragraph | number | 0 | ≥ 0 | — |
| Hanging Punctuation | bool | off | on/off | Roman hanging punctuation |
| Hyphenation | bool + dict | off | on/off + zone/limit | Panel menu |
| Paragraph direction | enum | LTR *(engine-dependent)* | LTR / RTL | Middle Eastern engine |
| Composition | enum | not stated | single-line / every-line | `Ctrl+Shift+Alt+T` |

### Style panels

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Style name | string | "Style N" / Basic Paragraph | — | Basic Paragraph not renamable/deletable |
| Style attributes | attribute set | captured/current | character (+ paragraph for paragraph styles) | Edited via Style Options dialog |
| Default type styles | command | — | Save/Load | PDF marks Creative Cloud only (Open questions) |

## Algorithms & pipeline

The panels are **attribute editors over the text engine** (`03-tools/type-tools.md`); they do not lay out or rasterize text.

1. **Attribute resolution.** The UI shows the effective value of each attribute for the current selection using the documented hierarchy **manual override > applied character style > applied paragraph style**. Editing a field writes a **manual override** onto the selected run(s)/paragraph(s).
2. **Command emission.** Each committed edit is one command on the Rust side (keystroke-coalesced for live typed values; one command per field commit). Multiple selection shows a combined value or blank, per standard control behavior *(inferred)*.
3. **Style apply.** Applying a style stamps the style reference; per-attribute manual overrides already present are not cleared (hierarchy preserved). Editing a style updates all text using it.
4. **Paragraph scope.** Alignment/justification/spacing apply per paragraph; the panel targets the paragraph(s) containing the caret/selection, or all paragraphs when the type layer is selected.
5. **Live preview.** Font/size/tracking/kerning/scaling/leading edits must re-shape and re-layout through `pictura-text` and repaint the canvas overlay; the panels hold no geometry.

## Rust module mapping

The engine lives in `pictura-text` (`03-tools/type-tools.md`). The panel layer is:

- `pictura_text::style::CharacterStyle` / `ParagraphStyle` / `TextStyleSheet` — attribute sets, style references, and hierarchy resolution (already proposed in `type-tools.md`).
- `pictura_text::attrs::{CharacterAttrs, ParagraphAttrs}` — the field set the panels edit.
- `pictura_core::text::TextSettings` — the document-model payload the values commit into (`ARCH-008`).
- `pictura_core::command::EditTextAttrs` / `ApplyTextStyle` — undoable commands (`ARCH-009`).
- `pictura_ui_bridge::TextSelection` — current run/paragraph selection so the panels can display/combine effective values.

Crossing types: `TextSettings`, `CharacterAttrs`, `ParagraphAttrs`, style ids, and selection ranges. No Qt types cross into `pictura-text`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `CharacterPanel` | `QWidget` dock | Eleven controls, dynamic-shortcut menu actions, panel menu |
| `ParagraphPanel` | `QWidget` dock | Alignment/justification, indents, spacing, hanging punctuation, hyphens |
| `CharacterStylesPanel` / `ParagraphStylesPanel` | `QWidget` dock + list view | Style list; `QAbstractListModel`; create/rename/delete/edit |
| `TypeStyleOptionsDialog` | `QDialog` | Category tree + attribute controls for style creation/editing |
| `TypeOptionsBar` | `QWidget` (options bar) | Preset, orientation, font combo, size, AA, alignment, warp, panels |
| `FontComboBox` | `QFontComboBox`/`QComboBox` | WYSIWYG family list, type-ahead, font-kind icons (`type-tools.md`) |
| `StyleHierarchyIndicator` | `QLabel`/badge | Shows manual/character/paragraph provenance *(proposed)* |
| `NumericPanelField` | `QDoubleSpinBox` + menu | Implements Enter/Shift+Enter/Tab apply semantics |

Widgets over QML (dense docks, model/view lists, keyboard-centric; `ARCH-003`). `QFontDatabase`/`QFont` are used to *enumerate and select* application fonts only; shaping/layout remain in `pictura-text` (`type-tools.md`). Panels share one selection bridge so all four stay consistent.

## Data-model impact

- **Document data.** Character/paragraph attributes and style definitions are stored on the `NodeKind::Text` payload (`ARCH-008`) and must round-trip to PSD text-engine data; the exact PSD keys are unresolved in `ARCH-008`/`type-tools.md` (Open questions).
- **Style storage.** Where CS6 serializes Character/Paragraph styles (image resource, additional-layer block, or XMP) is not sourced; preserve them as opaque data until resolved (`type-tools.md`).
- **Undo.** Attribute edits are one command per committed edit (keystroke-coalesced); applying a style is one command. Manual overrides and style application must be reversible without losing the other.
- **Default type styles** are application/preference state (and per the PDF possibly Creative Cloud only), not per-document.
- **Selection is UI state**, never serialized.
- **CS5/CS6 text compatibility.** The global **Blend Text Colors Using Gamma** option (1.45) lives in Color Settings, not these panels, but it changes how these attributes render (`type-tools.md`).

## Edge cases

- **Mixed selections** — fields with differing values show the standard indeterminate state; committing applies to all selected runs/paragraphs.
- **No type tool active / no text selected** — panels are disabled or edit defaults for new text (CS6: size/leading/tracking "applies to new text you create").
- **Basic Paragraph** — editable but not renamable/deletable; the UI must disable those actions.
- **Editing a style with no text selected** — allowed by selecting an image layer; edits still update associated text.
- **Paragraph attributes in point type** — each line is its own paragraph; alignment options are paragraph-type-only and must disable for point type.
- **Vertical type** — left/right indent maps to top/bottom; underline becomes left/right; alignment uses top/center/bottom.
- **Asian type** — Set Tsume and leading measurement require **Show Asian Text Options**; hide them otherwise.
- **Middle Eastern engine** — paragraph direction and digit type appear only when enabled; the panel must not assume LTR.
- **Fractional widths** — applies to the whole layer, not the selection; UI should communicate that.
- **Missing fonts** — style application follows the missing-font/glyph-protection path (`type-tools.md`).
- **Unsupported modes** — no type layers in Multichannel/Bitmap/Indexed; panels are unavailable/rasterized.
- **Huge PSB / long text** — live re-shaping on every field change must be throttled/debounced; use proxy layout during drag (`01-architecture/performance-targets.md`).
- **Undo/redo** — uncommitted edit sessions cancel without touching history (`type-tools.md`).
- **Creative Cloud-only default styles** — if absent in shipped CS6, the menu commands must be gated, not broken.

## Parity acceptance criteria

1. Given selected horizontal type, changing Font Family/Size/Leading/Tracking/Kerning/Baseline Shift in the Character panel updates the canvas; the options bar and Character panel stay in sync.
2. Given numeric input, `Enter` applies, `Shift+Enter` applies and selects the value, and `Tab` applies and moves to the next field.
3. Given `Alt`/`Option`+`←`/`→` between two characters, the Kerning value changes by the documented step and is reflected in the panel.
4. Given Fractional Widths off (`System Layout`), spacing is quantized to whole pixels for the entire type layer.
5. Given a paragraph-type bounding box, Left/Center/Right alignment and justification options reflow text; point type disables alignment.
6. Given Justification values, Word Spacing (0–1000%), Letter Spacing (−100–500%), and Glyph Scaling (50–200%) affect only justified paragraphs within the Min/Desired/Max model.
7. Given a negative First-Line Indent, the first line hangs; Left/Right Margin indents the paragraph edges.
8. Given Roman Hanging Punctuation on, the listed punctuation hangs outside the margin appropriate to the alignment.
9. Given a Character Style and a Paragraph Style applied to the same text plus a manual override, the manual override wins, then the character style, then the paragraph style, for every attribute.
10. Given a new document, a Paragraph Styles panel exists with an editable but non-deletable **Basic Paragraph**; double-clicking a style and changing an attribute updates all text using it.
11. Given editing a style, all text using it updates; applying a style to text with manual overrides does not clear those overrides.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: Character panel purpose, open paths, the eleven labelled controls, numeric apply semantics (Enter/Shift+Enter/Tab), Dynamic Shortcuts (Faux Bold/Italic, All Caps, Small Caps, Superscript, Subscript, Underline, Strikethrough), type-size units and the PostScript/traditional preference, leading baseline-to-baseline and "largest leading wins", auto-leading via Paragraph panel menu, Metrics/Optical/manual kerning and `Alt`+arrow, tracking/kerning in 1/1000 em and cumulative, Baseline Shift direction, Fractional Widths/System Layout applying to the whole layer, Underline Left/Right and Strikethrough, color via options bar/Character panel; the Paragraph panel purpose and labelled controls, alignment/justification option lists (horizontal and vertical), Justification dialog ranges (word 0–1000%, letter −100–500%, glyph 50–200%), indents and hanging first line, space before/after, Roman hanging punctuation and its listed characters, hyphenation/composition, paragraph direction; Character Styles and Paragraph Styles (CS6), the "Basic Paragraph" default that cannot be renamed/deleted, style create/edit flows, and the manual > character > paragraph hierarchy; the PDF's **Creative Cloud only** label on Save/Load Default Type Styles; `Type > Panels` and `Window > …` panel paths; `Ctrl+Shift+Alt+T` composer toggle.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Character_palette.html` — Martin Evening, *Adobe Photoshop CS6 for Photographers*. **(secondary, CS6)** Established the type options bar left-to-right contents (saved type tool preset, orientation toggle, WYSIWYG font list with type-ahead, font style, size, anti-aliasing None→Smooth, paragraph alignment, Warp, Panel button opening Character + Paragraph), the Character/Character Styles panel relationship, and the Character Styles "New character style" button.
- `https://doc.qt.io/qt-6/qtreeview.html` — Qt 6 model/view and drag-reorder behavior informing the style-list/options proposals.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- SearXNG query "Photoshop CS6 Character Styles Paragraph Styles panel Type menu" — corroborating snippets (Adobe help, Textuts, Layers Magazine, PHOTOLESSONS) for the CS6 Type menu and type-styles novelty; later-version Help pages, not asserted as CS6.

Not used in this pass:

- `helpx.adobe.com` character/paragraph pages (HTTP 403 / current-version only).

## Open questions

- **Default anti-aliasing preset** is not stated by the CS6 Help text (widely reported as Sharp) — shared with `03-tools/type-tools.md`. *Resolves with:* the Type preference default or a clean CS6 reference.
- **Default auto-leading percentage** is not stated in the fetched text. *Resolves with:* the Justification dialog on CS6.
- **Default composer** (single-line vs. every-line) is not stated. *Resolves with:* a CS6 Paragraph panel menu capture.
- **Exact ranges** for Baseline Shift, Tracking, and Kerning are given as "signed"/"1/1000 em" but the panel slider/spin bounds are not enumerated. *Resolves with:* a CS6 UI capture.
- **Where Character/Paragraph styles are serialized** in PSD/XMP is not sourced (`type-tools.md`). *Resolves with:* the Adobe File Formats Specification and CS6-authored PSDs.
- **Is `Save/Load Default Type Styles` base CS6 or Creative Cloud only?** The PDF says Creative Cloud; the CS6 feature brief lists styles. *Resolves with:* a shipped-CS6 build test and a decision recorded in `OVR-002`.
- **Which controls are duplicated on the type options bar vs. only in the panels** should be fixed against a CS6 capture; the fetched source gives the options-bar list only secondarily. *Resolves with:* a CS6 screenshot/measurement of the options bar.
