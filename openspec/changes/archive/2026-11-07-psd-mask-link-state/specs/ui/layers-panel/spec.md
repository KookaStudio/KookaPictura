# Spec Delta

## ADDED Requirements

### Requirement: Mask link controls express state through opacity

The Layers panel SHALL draw a raster or vector mask's existing linked-chain
glyph at 100% opacity when that mask is linked and its existing broken-chain
glyph at 50% opacity when unlinked. Opacity SHALL apply only to the link glyph,
leaving the row, thumbnails, focus markers, and other badges unaffected. Both
states SHALL keep the existing glyph position, size, rotation, and clickable
area. Clicking SHALL toggle the clicked mask's linkage in one undoable step;
undo and redo SHALL restore both the link state and its corresponding glyph
opacity. This requirement applies to the controls between layer/mask
thumbnails, not the separate layer-to-layer link badge or toolbar button.

#### Scenario: Raster mask glyph opacity follows linkage [pml_ui_raster_opacity]

- **WHEN** a raster-masked row is shown first linked and then unlinked
- **THEN** its linked glyph is rendered at 100% opacity and its unlinked glyph at 50%, with unchanged geometry and no opacity change outside the glyph

#### Scenario: Vector mask glyph opacity follows linkage [pml_ui_vector_opacity]

- **WHEN** a vector-masked row is shown first linked and then unlinked
- **THEN** its linked glyph is rendered at 100% opacity and its unlinked glyph at 50%, with unchanged geometry and no opacity change outside the glyph

#### Scenario: A dimmed control still toggles and follows history [pml_ui_toggle_history]

- **WHEN** the user clicks an unlinked raster or vector mask's dimmed control and then undoes and redoes the edit
- **THEN** only the clicked mask becomes linked in one history step, undo restores the unlinked half-opacity glyph, and redo restores the linked full-opacity glyph
