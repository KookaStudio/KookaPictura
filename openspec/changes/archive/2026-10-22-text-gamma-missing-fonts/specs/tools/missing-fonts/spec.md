## ADDED Requirements

### Requirement: Missing fonts are detected and reported on open

A type layer's font SHALL be missing when it is not the bundled Liberation
Sans and no installed font family matches it, by family name or by a
PostScript name's family stem. Opening a document whose type layers use one or
more missing fonts SHALL show one non-blocking warning naming each missing font
once. A document with no missing font SHALL show no alert. The affected layers
SHALL keep their stored pixels, and their Layers panel rows SHALL be marked as
using a missing font.

#### Scenario: Installed and bundled fonts are not missing [mf_detect]

- **WHEN** the bundled family, an installed family, and an installed family's PostScript name are checked, and so is an unknown name
- **THEN** only the unknown name is missing

#### Scenario: Opening alerts once with the missing names [mf_alert]

- **WHEN** a PSD with two type layers in one unknown font is opened
- **THEN** one warning opens that names the font once

#### Scenario: No alert without missing fonts [mf_no_alert]

- **WHEN** a PSD whose type layer uses the bundled family is opened
- **THEN** no missing-font warning opens

#### Scenario: The Layers row is marked [mf_badge]

- **WHEN** a document holds one type layer in an unknown font and one in the bundled family
- **THEN** only the first layer's row reports a missing font

### Requirement: Replace All Missing Fonts substitutes the default font

`Type > Replace All Missing Fonts` SHALL re-set every type layer whose font is
missing, in every group, in Liberation Sans Regular, keeping its text and
other attributes, and SHALL record one "Replace All Missing Fonts" state. A
manual font-family or font-style override SHALL follow the replacement. The
command SHALL be disabled when the active document has no missing font, and
when it is not 8-bit RGB, the only mode type is re-set in; there the open-time
alert SHALL say to convert the document first, and a replacement SHALL change
nothing.

#### Scenario: The engine replaces only the named fonts [mf_replace_engine]

- **WHEN** `replace_type_fonts` runs with one missing name over a document holding a grouped layer in that font and a layer in another font
- **THEN** it returns 1, the grouped layer reports Liberation Sans with its text unchanged, and the other layer is unchanged

#### Scenario: A style override follows the replacement [mf_style_override]

- **WHEN** a layer in a missing font carries a manual Bold style override and its fonts are replaced
- **THEN** its style and its style override are Regular, and no family override is added

#### Scenario: The command records one state [mf_replace_command]

- **WHEN** Replace All Missing Fonts is dispatched on a document with two type layers in an unknown font
- **THEN** both layers report Liberation Sans, both rows lose the mark, one "Replace All Missing Fonts" state was added, and undo restores the unknown font on both

#### Scenario: Outside 8-bit RGB nothing is replaced [mf_replace_not_rgb8]

- **WHEN** a document holding type in an unknown font is converted to Grayscale
- **THEN** the type layer remains, Replace All Missing Fonts is disabled, and a replacement changes nothing

#### Scenario: The command is disabled without missing fonts [mf_replace_disabled]

- **WHEN** the active document's type layers all use installed fonts
- **THEN** Replace All Missing Fonts is disabled
