# Design: paragraph-styles-panel

## Context

The engine already models named paragraph styles: `pictura_core::ParagraphStyle`
holds sparse `CharacterOverrides` and `ParagraphOverrides`, `TextStyleSheet`
carries the styles (always including the protected `Basic Paragraph`), and the
`type_apply_style` / `type_create_/edit_/delete_paragraph_style` bridge
functions operate on a document's sheet and record one history state each. The
`type-style-model` change built that model; the Character and Paragraph panels
(`ui/character-paragraph-panels`) edit a type layer's attributes but there is no
UI over the style sheet itself. This change adds the panel and its dialog.

## Goals / Non-Goals

- Goal: list, apply, create, edit, and delete the document's paragraph styles.
- Goal: reuse the shipped style-sheet model, bridge, and history path unchanged.
- Goal: the full CS6 Paragraph Style Options dialog, with every page's options
  backed by the type model rather than shown dead.
- Non-goal: the Character Styles panel (a separate follow-up).
- Non-goal: persisting styles to `.psd`; they live on the document for the
  session (the model predates any persistence).
- Non-goal: CS6's style-override "+" marker, which needs the applied layer's
  formatting compared against the style on every change.

## Decisions

### Extended attribute model

The CS6 pages need attributes the model did not carry: faux bold/italic, the
ten OpenType features, the spelling language, vertical Roman alignment, auto
leading, and the hyphenation dictionary. They are added to
`CharacterAttrs`/`CharacterOverrides` and `ParagraphAttrs`/
`ParagraphOverrides` (defaults from the CS6 dialog: standard ligatures and
contextual alternates on, auto leading 120%, words longer than 5, after first 2,
before last 2, hyphen limit 2, zone 36 pt, hyphenate capitalized words on), the
codec reads and authors the EngineData keys, and the bridge structs carry them
so the panel round-trips every control. `language` and
`vertical_roman_alignment` stay model-only: their EngineData encoding is not
modelled.

### Live preview

Preview is checked by default. A control change emits `previewChanged` and the
panel calls `type_preview_paragraph_style`, which edits the style sheet and
re-resolves every applying type layer without recording a history state. Cancel
previews the opening values back; Accept runs the normal recording edit, so one
undo returns to the pre-dialog state. Creating a style has nothing to preview,
so the checkbox is disabled in that flow.

### Resolved read-back

A style's stored attributes are sparse overrides, so a naive read-back would
report zeros for fields the style leaves inherited. The panel and dialog read a
style's *effective* attributes through `TextStyleSheet::resolve` (via the new
`type_paragraph_style_character` / `_paragraph` / `_font` bridge functions),
which fills unset fields from `Basic Paragraph` and the model defaults. Editing
then writes the full effective set back, matching how the Character and
Paragraph panels edit a layer.

### Rename as delete-then-create

The bridge edits a style by its current name. The dialog's Style Name field can
change, so an accepted edit compares the new name to the old: a same-name edit
calls `type_edit_paragraph_style`; a changed name calls
`type_create_paragraph_style` with the new name and then
`type_delete_paragraph_style` with the old. This trades two history states for
one on a rename; a rename is rare and the outcome is correct.

### Alignment mapping

The engine's `Justify` index order is Left 0, Right 1, Center 2, while CS6 (and
the dialog) present Left, Center, Right. The panel maps between them at the
dialog boundary.

## Risks / Trade-offs

- The dialog is modal through `runDialog` (non-modal with a parent input
  blocker), consistent with every other app dialog.
- Ceiling (`ponytail:`): no "+" override marker, no `.psd` persistence, no
  Character Styles panel; faux bold/italic and the OpenType features are stored
  and authored, not painted; `language` and `vertical_roman_alignment` are
  model-only; Preview is disabled while creating a style.
