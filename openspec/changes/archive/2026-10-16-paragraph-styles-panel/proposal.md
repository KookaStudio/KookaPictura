# Proposal: paragraph-styles-panel

## Why

Issue #76: port photorust's `ParagraphStylesPanel` and `ParagraphStyleDialog`
(`shell/src/panels/ParagraphStylesPanel.*`, `shell/src/dialogs/ParagraphStyleDialog.*`).

Kooka already has the engine model — `pictura_core::ParagraphStyle`,
`TextStyleSheet`, and the bridge (`type_create_/edit_/delete_paragraph_style`,
`type_apply_style`, `apply_type_style`), added by the `type-style-model`
change. What is missing is the panel itself: Type ▸ Panels ▸ Paragraph Styles
and Window ▸ Panels ▸ Paragraph Styles are disabled stubs and there is no UI
to list, apply, create, edit, or delete styles.

## What Changes

- New bridge read-back functions in `cxxqt_object/type_tools.rs`:
  `type_paragraph_style_count`, `type_paragraph_style_name`,
  `type_paragraph_style_character`, `type_paragraph_style_paragraph`,
  `type_paragraph_style_font` (the resolved attributes, so a sparse style
  reports the document default for the fields it leaves unset).
- New `paragraph_style_dialog.{h,cpp}` (+ `paragraph_style_dialog_pages.cpp`):
  CS6's full Paragraph Style Options — Style Name, Preview, and all seven pages
  (Basic Character Formats, Advanced Character Formats, OpenType Features,
  Indents and Spacing, Composition, Justification, Hyphenation). The type model
  grows the attributes those pages need (faux bold/italic, the ten OpenType
  features, language, vertical Roman alignment, auto leading, and the
  hyphenation dictionary), the codec reads and authors the EngineData keys, and
  Preview re-applies an edited style live without recording history.
- New `panels/paragraph_styles_panel.{h,cpp}`: the document's paragraph styles
  as a list; a single click applies the style to the active type layer, a
  double-click opens it for editing (re-applying to every layer that set it),
  and a footer creates and deletes styles. The default `Basic Paragraph` style
  cannot be deleted.
- Wiring: the two disabled command leaves become enabled checkable commands
  (`window.panels.paragraphStyles`, `type.panels.paragraphStyles`) with the
  panel toggles; the panel joins the hidden type group and follows the active
  document and Layers selection.
- Test: Qt Test `tst_paragraph_styles_panel` (list, apply, create/delete with
  the protected default, and both menu toggles).

## Capabilities

### Added Capabilities

- `ui/paragraph-styles-panel`: the Paragraph Styles panel.

## Impact

- `pictura-app` (bridge, C++). No new dependency. New `.cpp`/`.h` files are
  added to `CMakeLists.txt` explicitly; the Qt Test is added to
  `cpp/tests/CMakeLists.txt`.

## Provenance

Source: https://github.com/perfecto25/photorust (`shell/src/panels/ParagraphStylesPanel.*`,
`shell/src/dialogs/ParagraphStyleDialog.*`).
Co-authored-by: Zawaro <zawaroarts@gmail.com>
Signed-off-by: Mike <mike@...>
Signed-off-by: Zawaro <zawaroarts@gmail.com>

Ceiling (`ponytail:`): no style-override "+" marker (CS6 compares the layer's
formatting against the style); styles are not persisted to `.psd`; the
Character Styles panel is a separate follow-up. The extended model's rendering
stays with the type engine (faux bold/italic and OpenType features are stored
and authored, not painted); `language` and `vertical_roman_alignment` are
model-only (their EngineData encoding is not modelled), and Preview is disabled
while creating a style (nothing applies it yet).
