## ADDED Requirements

### Requirement: Apply Image grows a smaller destination layer

When Preserve Transparency is off and the destination layer is not
transparency-locked, Apply Image SHALL grow the destination pixel layer's rect
to the document before blending, padding the new area with transparent pixels
at every depth. Preserve Transparency or a transparency lock SHALL keep the
layer's rect. The active selection and the Mask group SHALL still confine the
blend. A preview that grew the layer SHALL shrink it back to its pre-preview
rect on Cancel, and repeated previews SHALL NOT compound across a growth.

#### Scenario: A small layer grows to the document [ai_grow_document]

- **WHEN** Apply Image runs on a layer smaller than the document with Preserve
  Transparency off
- **THEN** the layer's rect becomes the document rect, the old pixels keep their
  place, and the new area takes the source colour and alpha

#### Scenario: Preserve Transparency keeps the rect [ai_grow_preserve]

- **WHEN** Preserve Transparency is on for a smaller layer
- **THEN** the layer's rect and its clear pixels are unchanged

#### Scenario: A transparency lock keeps the rect [ai_grow_locked]

- **WHEN** the destination layer is transparency-locked and Preserve
  Transparency is off
- **THEN** the layer's rect is unchanged

#### Scenario: Selection confines the blend, not the growth [ai_grow_selection]

- **WHEN** an active selection covers part of the document and Apply Image runs
  on a smaller layer with Preserve Transparency off
- **THEN** the layer grows to the document, only selected pixels change, and
  unselected new pixels stay transparent

#### Scenario: Cancel restores the pre-preview rect [ai_grow_cancel]

- **WHEN** a preview grows a smaller layer and the dialog is cancelled
- **THEN** the layer's rect and every channel are bit-identical to the
  pre-preview layer

#### Scenario: Previews do not compound across a growth [ai_grow_no_compound]

- **WHEN** a control changes twice on a smaller layer and the preview grows it
  each time
- **THEN** the result equals one application to the original pixels

#### Scenario: A grown native-depth layer keeps its store [ai_grow_native]

- **WHEN** a 16- or 32-bit layer with a native store grows to the document
- **THEN** the store covers the document rect and keeps values finer than
  8-bit steps, and the 8-bit channels equal its narrowing
