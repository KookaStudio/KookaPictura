# ui/type-menu Specification

## Purpose
Wire the remaining Type menu rows to the type engine and keep their checked and enabled state in sync with the current type layer and the saved Type preferences.

## Requirements

### Requirement: Anti-Alias and Orientation apply to the current type layer

The five `Type > Anti-Alias` rows and the two `Type > Orientation` rows SHALL be enabled for an editable type layer, SHALL be radio-checked from the layer's current anti-aliasing method and orientation, and each SHALL re-set the layer and record exactly one `Edit Type Layer` state. They SHALL be disabled for a non-type layer or a document whose type layers cannot be re-set.

#### Scenario: Check states follow the layer [tm_aa_check]

- **WHEN** the Type menu is opened on a type layer with Smooth anti-aliasing and vertical orientation
- **THEN** `Smooth` and `Vertical` are checked and the other anti-aliasing and orientation rows are unchecked

#### Scenario: An anti-aliasing row applies [tm_aa_apply]

- **WHEN** `Type > Anti-Alias > Crisp` is triggered and then undone once
- **THEN** the layer's anti-aliasing becomes Crisp, one state is recorded, and the undo restores the previous method

#### Scenario: An orientation row applies [tm_orientation]

- **WHEN** `Type > Orientation > Vertical` is triggered and then undone once
- **THEN** the layer re-sets vertically, one state is recorded, and the undo restores horizontal orientation

#### Scenario: The rows are disabled without a type layer [tm_row_disabled]

- **WHEN** the current layer is not a type layer
- **THEN** the anti-aliasing and orientation rows are disabled

### Requirement: Rasterize Type Layer and Update All Text Layers

`Type > Rasterize Type Layer` SHALL dispatch the existing rasterize-type operation and record one `Rasterize Type` state, and SHALL be disabled for a non-type layer or a document whose type layers cannot be re-set. `Type > Update All Text Layers` SHALL refresh the pixels of type layers that can be reproduced from a single set of character and paragraph attributes, preserving their type data. Imported layers with distinct runs SHALL retain their stored pixels and type data. The command SHALL record one `Update All Text Layers` state when at least one layer is refreshed, and SHALL be disabled when the document has no type layers or its type layers cannot be re-set.

#### Scenario: Rasterize Type Layer dispatches the rasterizer [tm_rasterize]

- **WHEN** `Type > Rasterize Type Layer` runs on a type layer
- **THEN** the layer becomes a pixel layer, one `Rasterize Type` state is recorded, and the command is disabled afterwards because the layer is no longer type

#### Scenario: Update All Text Layers re-renders the document [tm_update_all]

- **WHEN** `Type > Update All Text Layers` runs on a document with type layers
- **THEN** the reproducible type layers are re-rendered and one `Update All Text Layers` state is recorded; with no type layers or a non-editable document the command is disabled

#### Scenario: Updating text preserves imported formatting [tm_update_all_runs]

- **WHEN** `Type > Update All Text Layers` runs on a document containing uniform type and imported type with distinct style runs
- **THEN** the uniform layer's pixels are refreshed without rewriting its type descriptor, and the mixed-run layer keeps its pixels and type descriptor

### Requirement: Language Options reflect the Type preferences

`Type > Language Options` SHALL offer `Default features`, `East Asian features`, and `Middle Eastern features`. The two feature rows SHALL mirror their existing type preferences as independent checkmarks, and `Default features` SHALL be checked when both feature preferences are off. Triggering a feature row SHALL toggle its preference; triggering `Default features` SHALL clear both preferences. Each trigger SHALL update the Character and Paragraph panels, record no document history state, and persist the choice in the typed preferences store.

#### Scenario: The menu reflects the preferences [tm_language]

- **WHEN** East Asian options are on and Middle Eastern options are off
- **THEN** `East Asian features` is checked, `Default features` and `Middle Eastern features` are unchecked, and the Character panel shows its Asian options

#### Scenario: Choosing Default clears the feature options [tm_language_default]

- **WHEN** both feature options are on and `Default features` is triggered
- **THEN** both preferences turn off, the panel options hide, no document history state is recorded, and the choice survives an application relaunch

### Requirement: Font Preview Size sets the font-picker preview

`Type > Font Preview Size` SHALL offer `None`, `Small`, `Medium`, `Large`, `Extra Large`, and `Huge`, radio-checked from the saved preview-size preference. `None` SHALL omit the sample from every font family picker; every other value SHALL set the sample's point size. Triggering a row SHALL record no document history state.

#### Scenario: The choice changes the sample [tm_font_preview]

- **WHEN** `Type > Font Preview Size > Huge` is triggered and a font family picker is opened
- **THEN** its samples are drawn larger than at `Small`, the row is checked, and the choice survives an application relaunch

#### Scenario: None removes the sample [tm_font_preview_none]

- **WHEN** `Type > Font Preview Size > None` is triggered and a font family picker is opened
- **THEN** each row draws only the family name in the UI font and no sample

### Requirement: Paste Lorem Ipsum inserts placeholder text

`Type > Paste Lorem Ipsum` SHALL be enabled while a Type tool session is open or the current layer is a type layer. With a session open it SHALL insert a lorem ipsum block at the session's caret, replacing any selection, and SHALL not record a document history state before the session commits. With a selected type layer, no session, and an editable document it SHALL append the block at the end of the layer's text and record one `Paste Lorem Ipsum` state. Otherwise it SHALL be disabled.

#### Scenario: A live session receives the text [tm_lorem_session]

- **WHEN** `Type > Paste Lorem Ipsum` is triggered while a type session is open
- **THEN** the lorem ipsum block replaces the session's selection or is inserted at its caret, the session stays open, and no history state is recorded until it commits

#### Scenario: A selected type layer receives the text [tm_lorem_layer]

- **WHEN** `Type > Paste Lorem Ipsum` is triggered with a type layer current, existing text, and no session open
- **THEN** the block is appended after the existing text, the layer re-renders, and one `Paste Lorem Ipsum` state is recorded

#### Scenario: Neither a session nor an editable type layer [tm_lorem_disabled]

- **WHEN** no type session is open and the current layer is not type or the document's type layers cannot be re-set
- **THEN** `Paste Lorem Ipsum` is disabled

### Requirement: Glyphs and OpenType rows

`Type > Panels > Glyphs` SHALL toggle the Glyphs panel and report its visibility like the Window menu entry; it SHALL work without a type layer. `Type > OpenType` SHALL list the ten modelled character features (standard ligatures, contextual alternates, discretionary ligatures, swash, oldstyle, stylistic alternates, titling alternates, ornaments, ordinals, and fractions), SHALL check them from the current type layer, and each SHALL re-set the layer and record one `Edit Type Layer` state. The Character panel menu keeps its eight existing features; the Type menu is the only surface for Oldstyle and Ornaments, and both surfaces toggle the same underlying flags. The OpenType rows SHALL be disabled for a non-type layer or a document whose type layers cannot be re-set.

#### Scenario: The Glyphs row toggles the panel [tm_glyphs]

- **WHEN** `Type > Panels > Glyphs` is triggered twice
- **THEN** the Glyphs panel shows, then hides, and the row's checked state follows the panel's visibility, with or without a type layer

#### Scenario: An OpenType row checks and applies [tm_opentype]

- **WHEN** `Type > OpenType > Fractions` is triggered on a type layer and then undone once
- **THEN** the row becomes checked, the layer's fractions flag turns on, one state is recorded, and the undo restores the previous flag

#### Scenario: The two OpenType surfaces share their flags [tm_opentype_cross_surface]

- **WHEN** `Type > OpenType > Fractions` is toggled and the Character panel menu is opened
- **THEN** the Character panel's Fractions row reflects the same flag, and toggling it there changes the Type menu's checked state

### Requirement: Default Type Styles menu rows

`Type > Save Default Type Styles` SHALL be enabled only while a type layer is current. `Type > Load Default Type Styles` SHALL be enabled only while a type layer is current, defaults have been saved, and the document's type layers can be re-set. Their behaviour is specified by `tools/type-default-styles`.

#### Scenario: The rows follow the layer and the saved defaults [tm_default_rows]

- **WHEN** the Type menu is opened before any defaults exist and then after saving defaults on a type layer
- **THEN** `Save Default Type Styles` is enabled whenever a type layer is current, and `Load Default Type Styles` is disabled before the save and enabled after it; a non-editable document keeps Load disabled

### Requirement: Layer-mutating Type rows are disabled during a Type session

While a Type tool session is open, the anti-aliasing, orientation, OpenType, Rasterize Type Layer, Update All Text Layers, and Load Default Type Styles rows SHALL be disabled and SHALL record no state. Paste Lorem Ipsum is the exception and remains enabled.

#### Scenario: A live Type session gates the option rows [tm_session_gate]

- **WHEN** a Type tool session is open and the Type menu is opened
- **THEN** the anti-aliasing, orientation, OpenType, Rasterize, Update All, and Load Defaults rows are disabled, while Paste Lorem Ipsum remains enabled

### Requirement: Doc-divergent Type rows are absent

The Type menu SHALL NOT contain `Check Spelling…`, `Find And Replace Text…`, or `Extrude to 3D`. Check Spelling remains an Edit-menu entry and is tracked by #1443; Find And Replace is an Edit-menu entry specified by `ui/find-and-replace-text`.

#### Scenario: The Type catalogue omits the rows [tm_removed_rows]

- **WHEN** the command catalogue and the built Type menu are inspected
- **THEN** no Type path contains `Check Spelling…`, `Find And Replace Text…`, or `Extrude to 3D`
