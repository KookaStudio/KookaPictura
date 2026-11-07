# Spec Delta

## ADDED Requirements

### Requirement: Raster mask cards remain opaque after fitting

The Layers delegate SHALL cover the entire document-aspect raster mask card
with its opaque grayscale preview, including when independently rounded image
and card sizes differ. A second aspect fit SHALL NOT expose the checkerboard
along any card edge. Square group, type, adjustment and fill cards SHALL retain
their existing fitted-preview layout.

#### Scenario: A portrait PSD opens without an edge strip [mtp_portrait]

- **WHEN** portrait and landscape PSD masks are opened at each thumbnail size
- **THEN** their source thumbnails are opaque and their document-aspect cards contain grayscale coverage through every interior edge, with a black outline and no checkerboard strip, including at fractional display scale

#### Scenario: Square mask cards keep their layout [mtp_square]

- **WHEN** mask previews are displayed on adjustment or fill rows in a portrait document
- **THEN** the cards remain square and their document-aspect previews remain fitted within those cards

## MODIFIED Requirements

### Requirement: Layer mask row indicators

When a layer row carries a layer mask, the delegate SHALL draw a link glyph
between the layer thumbnail and the mask thumbnail when that mask is linked to
its layer, and SHALL draw a red cross over the mask thumbnail when the mask is
disabled. When a row carries a vector mask, the delegate SHALL likewise draw a
vector-mask thumbnail with its own link glyph when linked and red cross when
disabled. Clicking a mask's link glyph SHALL toggle that mask's linked state,
and `Shift`-clicking a mask thumbnail SHALL toggle that mask's enabled state,
through the matching bridge; a `Shift`-click SHALL NOT end a mask solo view on
that mask (the solo keeps showing the mask's stored coverage); those clicks SHALL
be consumed so they do not start a rename, select another row, or begin a drag. A plain click on a layer mask
thumbnail SHALL make that mask the view's mask edit target through the bridge,
and a plain click on the layer thumbnail SHALL clear the target, so the mask
thumbnail of the targeted row SHALL use the existing white corner brackets as
its only white selection indicator. Raster mask cards SHALL have the normal
black thumbnail outline and SHALL NOT gain a white rectangular focus outline.
The brackets SHALL follow the selected thumbnail and SHALL clear from the mask
when its layer thumbnail or another layer is selected. An `Alt`/`Option`-click
on a layer mask thumbnail SHALL toggle the mask solo view (see
`ui/mask-solo-view`): it SHALL select the clicked row (activating its layer),
make that mask the mask edit target so the brackets stay on it, and toggle
the solo display, recording no history state. It SHALL be consumed so it starts
no rename, selects no *other* row, and begins no drag. `Alt`+`Shift`-clicking a
mask thumbnail SHALL be consumed without toggling the mask's enabled state or
the solo view: that combination is reserved for the rubylith overlay, which is
not implemented here. The model SHALL expose the per-row mask and vector-mask
linked and disabled states through roles populated from the bridge.

#### Scenario: A linked mask shows the link glyph [lmk_row_link]

- **WHEN** a row whose mask is linked is shown
- **THEN** a link glyph is drawn between the layer and mask thumbnails

#### Scenario: A disabled mask shows a red cross [lmk_row_disabled]

- **WHEN** a row whose mask is disabled is shown
- **THEN** a red cross is drawn over the mask thumbnail

#### Scenario: Clicking the link glyph unlinks the mask [lmk_row_link_click]

- **WHEN** the user clicks the link glyph on a linked mask row
- **THEN** the mask is unlinked in one undoable step

#### Scenario: Shift-clicking the mask thumbnail disables it [lmk_row_shift]

- **WHEN** the user `Shift`-clicks the mask thumbnail of an enabled mask
- **THEN** the mask is disabled in one undoable step

#### Scenario: Shift-clicking a soloed mask keeps it soloed [lmk_row_shift_solo]

- **WHEN** a mask is soloed and the user `Shift`-clicks its thumbnail to toggle
  its enabled state
- **THEN** the mask's enabled state toggles and the canvas still shows the mask
  plane

#### Scenario: Selecting another layer ends the solo [lmk_row_solo_exit]

- **WHEN** a mask is soloed and the user selects a different layer row
- **THEN** the solo view ends and the canvas shows the composite

#### Scenario: A plain click activates the mask [lmk_row_activate]

- **WHEN** the user clicks the mask thumbnail of a row that carries a mask
- **THEN** that mask becomes the mask edit target and the thumbnail draws white corner
  brackets and a black outline, with no history state

#### Scenario: A layer-thumbnail click deactivates the mask [lmk_row_deactivate]

- **WHEN** a mask is the edit target and the user clicks that row's layer
  thumbnail
- **THEN** the mask edit target is cleared and the mask brackets disappear and the image brackets become active

#### Scenario: Alt-clicking the mask thumbnail toggles solo [lmk_row_alt_solo]

- **WHEN** the user `Alt`-clicks the mask thumbnail of a raster-mask row
- **THEN** the row is selected, the mask becomes the edit target with its white corner
  brackets, and the canvas shows the mask plane, with no history state

#### Scenario: A second Alt-click restores the composite [lmk_row_alt_restore]

- **WHEN** the user `Alt`-clicks the same mask thumbnail again while it is soloed
- **THEN** the composite is shown again and the mask stays the edit target

#### Scenario: Alt+Shift is reserved for rubylith [lmk_row_alt_shift]

- **WHEN** the user `Alt`+`Shift`-clicks a mask thumbnail
- **THEN** neither the solo view nor the enabled-state toggle takes effect

#### Scenario: A vector mask shows its own indicator [vmk_row_vector]

- **WHEN** a row whose vector mask is linked and enabled is shown
- **THEN** a vector-mask thumbnail is drawn alongside the layer thumbnail with
  its link glyph

#### Scenario: A disabled vector mask shows a red cross [vmk_row_vector_disabled]

- **WHEN** a row whose vector mask is disabled is shown
- **THEN** a red cross is drawn over the vector-mask thumbnail

#### Scenario: Clicking the vector link glyph unlinks it [vmk_row_vector_link_click]

- **WHEN** the user clicks the vector-mask link glyph on a linked row
- **THEN** the vector mask is unlinked in one undoable step

#### Scenario: Shift-clicking the vector thumbnail disables it [vmk_row_vector_shift]

- **WHEN** the user `Shift`-clicks the vector-mask thumbnail of an enabled vector mask
- **THEN** the vector mask is disabled in one undoable step

#### Scenario: Mask selection keeps a black outline [mtp_black_outline]

- **WHEN** a raster mask thumbnail on a pixel layer or group is selected
- **THEN** its card has the normal black outline and white corner brackets, without an additional white rectangular outline

#### Scenario: Another layer clears mask highlighting [mtp_other_layer]

- **WHEN** a mask thumbnail is selected and the user clicks another layer's raster thumbnail
- **THEN** only the newly active raster thumbnail has white brackets and the old mask retains only its black outline
