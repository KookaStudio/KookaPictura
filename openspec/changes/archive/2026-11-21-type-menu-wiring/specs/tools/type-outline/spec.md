# Spec Delta

## Purpose

Turn a live type layer's glyph outlines into a document work path or a shape layer while preserving the type's visible geometry.

## ADDED Requirements

### Requirement: Create Work Path traces a type layer

`Type > Create Work Path` SHALL trace the current type layer's glyph outlines in document pixels into the document's single work path, leave the type layer unchanged, make the work path the active Paths-panel row, and record exactly one `Create Work Path` state. A new work path SHALL replace the previous work path. The command SHALL be disabled for a non-type layer, a face without outline data, a document whose type layers cannot be re-set, or an open Type tool session, and SHALL record no state when it does not apply. The outline follows the engine's single-run type model (the first style run), which is a documented ceiling for an imported layer with multiple style runs.

#### Scenario: The path follows the rendered type [tout_path_bounds]

- **WHEN** `Type > Create Work Path` runs on an engine-authored, single-run point-type layer
- **THEN** the work path's outline lies within one pixel of the rendered glyphs, the path is the active Paths-panel row, and the type layer's content and history are otherwise unchanged

#### Scenario: A new work path replaces the old one [tout_path_replaces]

- **WHEN** the document already has an unsaved work path and `Type > Create Work Path` runs on a type layer
- **THEN** the new path replaces the old work path, one undo restores the old path, and the type layer is unchanged

#### Scenario: Outline-less fonts are refused [tout_path_no_outlines]

- **WHEN** the current type layer's face has no outline data
- **THEN** `Create Work Path` is disabled and records no history state

### Requirement: Convert to Shape replaces the type layer

`Type > Convert to Shape` SHALL replace the current type layer in place with a shape layer carrying the same name, stack position, blend mode, opacity, raster layer mask, and layer effects. The shape SHALL be filled with the type layer's fill colour and cut to the glyph outline. The glyph outline becomes the shape's vector mask; an existing vector mask on the type layer is replaced. The command SHALL record exactly one `Convert to Shape` state, and undoing that state SHALL restore the type layer. It SHALL be disabled for a non-type layer, a face without outline data, a document whose type layers cannot be re-set, or an open Type tool session.

#### Scenario: The type layer becomes a shape layer [tout_shape_replaces]

- **WHEN** `Type > Convert to Shape` runs on a type layer
- **THEN** the same row is a shape layer with a vector mask matching the glyph outline within one pixel, filled with the type layer's fill colour, the type tool data is gone, and one `Convert to Shape` state is recorded

#### Scenario: Appearance is retained [tout_shape_keeps]

- **WHEN** a styled, raster-masked type layer is converted
- **THEN** its name, stack position, blend mode, opacity, raster layer mask, and layer effects are retained on the shape layer

#### Scenario: Undo restores the type layer [tout_shape_undo]

- **WHEN** the conversion is undone once
- **THEN** the row is a type layer again with its original text and attributes

#### Scenario: The converted shape round-trips [tout_shape_round_trip]

- **WHEN** a converted shape layer is saved and reopened as PSD
- **THEN** it reopens as a shape layer with the same outline, fill colour, and layer effects, not as a type layer

#### Scenario: Outline-less fonts are refused [tout_shape_no_outlines]

- **WHEN** the current type layer's face has no outline data
- **THEN** `Convert to Shape` is disabled and records no history state

### Requirement: Outline commands cover the type placements

Create Work Path and Convert to Shape SHALL use the visible outlines of warped type, the rotated glyphs of type on a path, and the laid-out glyphs of area type. Area type SHALL trace its laid-out glyphs without the render-time clip to the area path, which is the documented approximation. For point and paragraph type the outline follows the engine's single-run type model (the first style run); a multi-run imported layer may not match its stored pixels.

#### Scenario: Warped type traces its warped outline [tout_warped]

- **WHEN** a warped type layer is converted to a work path or a shape
- **THEN** the outline follows the warped glyphs rather than the unwarped layout

#### Scenario: On-path type traces its placed glyphs [tout_onpath]

- **WHEN** a type-on-a-path layer is converted to a work path or a shape
- **THEN** each glyph outline sits where the type renders along the path, rotated with the path tangent

#### Scenario: Area type traces its laid-out glyphs [tout_area]

- **WHEN** an area-type layer is converted
- **THEN** the outline covers the glyphs as laid out in the area box, without clipping them to the area path

### Requirement: Outline commands are disabled during a Type session

While a Type tool session is open, Create Work Path and Convert to Shape SHALL be disabled and SHALL record no state.

#### Scenario: A live Type session gates the outline commands [tout_session_gate]

- **WHEN** a Type tool session is open and the Type or Layers menu is opened on its layer
- **THEN** Create Work Path and Convert to Shape are disabled and record no state until the session commits or cancels

### Requirement: The Layers row menu offers the same outline commands

The Layers panel's type rows SHALL offer `Create Work Path` and `Convert to Shape` with the same enablement and behaviour as the Type menu entries, and SHALL dispatch the stable `type.create.work.path` and `type.convert.to.shape` command ids. The rows SHALL be absent for non-type rows, while the Type menu entries SHALL be disabled for a non-type layer.

#### Scenario: The row menu and the Type menu agree [tout_layer_rows]

- **WHEN** a type layer is current and either its Layers row menu or the Type menu is opened
- **THEN** both entries are enabled for an outline face, the Type menu entries are disabled for a non-type layer, the Layers rows are absent for a non-type row, and triggering either performs the same operation and carries the stable command id
