# Document variables and data sets

## Purpose

Keeps the bindings and values for a reusable layered template and applies one chosen variant to its document.

## ADDED Requirements

### Requirement: Variables bind compatible layers by stable identity

The document SHALL own named variables, per-layer bindings, and an ordered collection of named data sets. A variable name SHALL match `[A-Za-z_:][A-Za-z0-9_.:-]*`. One variable SHALL have one kind and MAY bind multiple layers. A layer SHALL have at most one binding per kind. Visibility SHALL be available on non-Background layers, including groups. Text replacement SHALL target editable type layers. Pixel replacement SHALL target non-Background layers with editable raster pixels. Binding identity SHALL survive layer rename and reorder. A duplicate layer SHALL start without bindings. Deleting a layer SHALL remove its bindings, deleting a variable and its values when its last binding is removed. Undo SHALL restore the original bindings and values.

#### Scenario: Linked bindings follow layer identity [vars_bindings]

- **WHEN** two eligible layers are linked, one is renamed and reordered, and the other is duplicated
- **THEN** setting the shared value updates the original two layers and excludes the duplicate

#### Scenario: Invalid names and incompatible bindings are refused [vars_validation]

- **WHEN** a user supplies an invalid name, shares one name across kinds, or binds an ineligible layer
- **THEN** the edit is refused with a reason and the variable store is unchanged

#### Scenario: Deletion and undo restore bindings [vars_delete_undo]

- **WHEN** the last layer bound to a variable is deleted and that deletion is undone
- **THEN** undo restores the layer, variable, and all stored values for that variable

### Requirement: Data sets hold typed values

Each set SHALL store one visibility boolean, text string, or replacement file choice per variable. The replacement choice SHALL distinguish a file from Do Not Replace. Renaming a variable SHALL preserve its values. Adding a variable SHALL initialize it in existing sets from the current document layer state, using Do Not Replace for pixels. A new set SHALL copy the selected set's values when a set is selected and otherwise initialize from the current document layer state. For a visibility or text variable with multiple eligible bindings, initialization from current state SHALL use the first eligible bound layer in top-to-bottom document stack order, visiting a group before its children recursively; pixels still initialize to Do Not Replace. If no eligible bound layer exists, initialization SHALL fail with a reason and leave the store unchanged. An empty or multiline string SHALL be a valid text value. New sets SHALL default to the next unused `Data Set N` name. Data set names SHALL be nonempty and unique within a document.

#### Scenario: Set edits and variable rename retain values [vars_set_values]

- **WHEN** a set is created and edited, a second set is created from it, and a variable is renamed
- **THEN** both sets retain their typed values under the renamed variable and subsequent edits to one set leave the other set's values intact

#### Scenario: Linked layers with different values initialize deterministically [vars_set_linked_defaults]

- **WHEN** linked eligible layers have different current text or visibility and a set is created with no selected set
- **THEN** its value comes from the first eligible binding in top-to-bottom recursive document stack order

### Requirement: Apply changes the document as one transaction

Apply SHALL evaluate a set against the current document and commit its changed layer state as one undoable `Apply Data Set: <name>` edit. An unchanged result SHALL add no history state. Visibility SHALL change the target's visibility, including group visibility. Text replacement SHALL keep the type layer editable and use its current layout and modeled style through the type engine, including a blank rendered result for an empty value. Do Not Replace SHALL leave the target pixels as they are when Apply starts. An unreadable replacement file, an invalid target, or an unsupported mode SHALL fail the set with a useful error and leave the live document and history unchanged.

#### Scenario: A mixed set is undoable and keeps editable type [vars_apply_undo]

- **WHEN** a set changes visibility, styled multiline text, and raster pixels and the user applies, undoes, and redoes it
- **THEN** exactly one history state covers the changes, the replacement text remains editable, and undo and redo restore the corresponding layers and set definitions

#### Scenario: Empty text is an editable blank layer [vars_empty_text]

- **WHEN** a set replaces text with an empty string
- **THEN** the layer renders no glyphs, remains editable, and reopens with empty text after save

#### Scenario: Do Not Replace preserves the result of an earlier Apply [vars_skip_pixels]

- **WHEN** a user applies a file replacement and then applies a set whose value is Do Not Replace
- **THEN** the replacement remains in the live document and a set with no other changes adds no history state

#### Scenario: Failed application is atomic [vars_apply_atomic]

- **WHEN** a set mixes a valid visibility change with an unreadable replacement image or an unsupported target
- **THEN** an error identifies the failed variable and the document, selection, and history remain as before

### Requirement: Pixel replacement places pixels in the target layer frame

Pixel replacement SHALL use each bound layer's pixel rectangle at the start of the operation. Fit SHALL preserve aspect ratio and stay inside the rectangle. Fill SHALL preserve aspect ratio and cover it. As Is SHALL use the source size. Conform SHALL scale both axes to the rectangle. The nine alignments SHALL position Fit, Fill, and As Is within the target frame. Clipping SHALL be available for Fill and As Is and SHALL use the target rectangle. Replacement SHALL keep the layer's identity, masks, effects, and blend settings. It SHALL use the document's mode and depth through the supported image conversion paths and report unsupported conversions. A zero-area target SHALL be refused without changing the layer.

#### Scenario: Placement, alignment, and clipping match the selected options [vars_pixel_placement]

- **WHEN** a non-square source is placed in an offset frame through all four methods and clipped and unclipped variants are compared
- **THEN** the pixel bounds and alignment match the selected methods, clipping respects the original frame, and the layer keeps its mask and identity

#### Scenario: Native-depth replacement preserves document encoding [vars_native_pixels]

- **WHEN** a replacement image is applied in a supported high-depth or non-RGB raster document
- **THEN** the layer and its saved channels use the document mode and depth, and the displayed replacement is current

#### Scenario: Empty replacement frame is refused [vars_empty_frame]

- **WHEN** a set targets a zero-area raster layer with a replacement file
- **THEN** Apply reports the unusable target and records no change
