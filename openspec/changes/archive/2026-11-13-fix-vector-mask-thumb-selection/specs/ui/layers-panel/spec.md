# Spec Delta

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
be consumed so they do not start a rename, select another row, or begin a drag.

A layer row's thumbnail selection SHALL consist of an optional vector-mask
selection and exactly one companion, either the content thumbnail or the
raster-mask thumbnail. A plain click on the content thumbnail SHALL select the
content companion and SHALL preserve an active vector-mask selection. A plain
click on the raster-mask thumbnail SHALL select the raster-mask companion and
SHALL preserve an active vector-mask selection. While the raster mask is the
companion it SHALL be the view's mask edit target through the bridge; the
content companion SHALL clear the target. A plain click on the vector-mask
thumbnail SHALL toggle its selection while keeping the current companion. A
layer whose only mask is a vector mask SHALL use the content thumbnail as its
companion. A thumb click on a layer that is not the active layer SHALL activate
that layer first and SHALL start its selection from the content companion: its
content thumbnail SHALL select content only, its raster-mask thumbnail SHALL
select the raster mask only, and its vector-mask thumbnail SHALL select content
and the vector mask. Selecting another layer SHALL clear the previous layer's
mask and vector selection. A plain click on the row body SHALL remain governed
by the plain-row-click requirement. A `Shift`-click on either mask thumbnail, a
`Ctrl`-click on the raster-mask thumbnail, and an `Alt`+`Shift`-click on the
raster-mask thumbnail SHALL perform only their existing enabled-state,
mask-coverage-selection, or rubylith effect and SHALL NOT change the companion
or vector-mask selection; activating an inactive row MAY still default it to the
content companion, and an `Alt`-click on the raster-mask thumbnail SHALL still
select the raster mask as the companion and edit target.

Each selected thumbnail other than a group row's content icon SHALL use the
existing white corner brackets as its only white selection indicator; a row MAY
draw the content and vector brackets together, or the raster-mask and vector
brackets together, and a group row SHALL NOT draw brackets around its icon.
Raster mask cards SHALL have the normal black thumbnail outline and SHALL NOT
gain a white rectangular focus outline. An `Alt`/`Option`-click on a layer mask
thumbnail SHALL toggle the mask solo view (see `ui/mask-solo-view`): it SHALL
select the clicked row (activating its layer), make that mask the mask edit
target so the brackets stay on it, and toggle the solo display, recording no
history state. It SHALL be consumed so it starts no rename, selects no *other*
row, and begins no drag. `Alt`+`Shift`-clicking a mask thumbnail SHALL be
consumed without toggling the mask's enabled state or the solo view: that
combination is reserved for the rubylith overlay, which is not implemented here.
The model SHALL expose the per-row mask and vector-mask linked and disabled
states through roles populated from the bridge.

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
- **THEN** the mask edit target is cleared, the mask brackets disappear, the
  content brackets become active, and an active vector mask stays selected

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

#### Scenario: Modifier clicks keep the thumbnail selection [lmk_row_modifiers_keep_selection]

- **WHEN** the user `Shift`-clicks either mask thumbnail, `Ctrl`-clicks the
  raster-mask thumbnail, or `Alt`+`Shift`-clicks the raster-mask thumbnail
- **THEN** only that mask's enabled state, the mask-coverage selection, or the
  rubylith overlay changes, and the row's companion and vector-mask selection
  stay unchanged, except that activating an inactive row MAY default it to the
  content companion

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

#### Scenario: A vector click selects both masks [vmk_row_vector_selects_both]

- **WHEN** the active layer has a raster mask and a vector mask, the raster mask
  is the companion, and the user clicks the vector-mask thumbnail
- **THEN** the vector mask is selected, the raster mask stays selected, and the
  raster mask remains the edit target; both mask thumbnails draw white brackets

#### Scenario: A second vector click deselects only the vector [vmk_row_vector_toggle_off]

- **WHEN** the user clicks the selected vector-mask thumbnail again
- **THEN** the vector mask is deselected, the raster mask stays selected, and it
  remains the edit target

#### Scenario: A raster-mask click preserves the vector state [lmk_row_raster_keeps_vector_state]

- **WHEN** the user clicks the raster-mask thumbnail while the vector mask is
  selected, and again while it is not selected
- **THEN** the raster mask becomes the edit target and the vector selection is
  unchanged in both cases

#### Scenario: A content click keeps the vector companion [vmk_row_vector_companion_content]

- **WHEN** the vector mask is selected and the user clicks the content thumbnail
- **THEN** the content thumbnail becomes the companion, the vector mask stays
  selected, the mask edit target is cleared, and both thumbnails draw brackets

#### Scenario: A raster-mask click switches the companion [vmk_row_vector_companion_mask]

- **WHEN** the vector mask is selected alongside the content thumbnail and the
  user clicks the raster-mask thumbnail
- **THEN** the raster mask becomes the companion and the edit target, the vector
  mask stays selected, and the content brackets clear

#### Scenario: A vector-only row uses the content companion [vmk_row_vector_only_companion]

- **WHEN** a layer has a vector mask and no raster mask and its vector-mask
  thumbnail is clicked
- **THEN** the vector mask and the content thumbnail are selected and the mask
  edit target stays clear

#### Scenario: An inactive row's content thumb selects content [vmk_row_inactive_content]

- **WHEN** the user clicks the content thumbnail of a layer that is not active
- **THEN** that layer becomes active with only its content thumbnail selected
  and no mask edit target

#### Scenario: An inactive row's mask thumb selects the mask [vmk_row_inactive_mask]

- **WHEN** the user clicks the raster-mask thumbnail of a layer that is not active
- **THEN** that layer becomes active with only its raster mask selected, the
  raster mask is the edit target, and the vector mask is not selected

#### Scenario: An inactive row's vector thumb selects content and vector [vmk_row_inactive_vector]

- **WHEN** the user clicks the vector-mask thumbnail of a layer that is not active
- **THEN** that layer becomes active with its content thumbnail and vector mask
  selected, and the mask edit target is clear

#### Scenario: Selecting another layer clears the previous selection [vmk_row_other_layer_clears]

- **WHEN** a row has mask or vector thumbnails selected and the user selects
  another layer
- **THEN** the previous row's mask and vector selections are cleared, and
  returning through its row body selects only the content thumbnail

### Requirement: Layers panel content and mask thumbnail layout

Within a layer row the mask thumbnail SHALL sit immediately to the right of the
image thumbnail with a chain glyph between them (a broken-chain glyph when the
link is off), and a vector-mask thumbnail SHALL sit immediately to the right of
the mask thumbnail with the same chain glyph between them. The mask and vector
thumbnails SHALL follow the document aspect independently of the image
thumbnail and owning layer kind. The chain
glyph SHALL be drawn two pixels larger than its slot and rotated a quarter turn
so it reads vertically between the side-by-side thumbnails, in both the linked
and unlinked states. The content and raster-mask thumbnails SHALL be the row's
active edit target: clicking either SHALL make it the edit target and mark it
with brackets. The vector-mask thumbnail SHALL add a bracketed selection without
becoming the edit target; clicking it SHALL toggle that selection. `Alt`-clicking
a thumbnail SHALL also activate it in the composite. A group row
SHALL NOT draw brackets around its icon. An **adjustment row's image thumbnail
SHALL be a neutral card bearing the icon for its decoded adjustment kind**, with
no checkerboard; a **fill row's image thumbnail SHALL show its content preview**
(a flat solid swatch, a gradient ramp, or a scaled pattern tile) **in a 1:1
square card**, the preview image itself rendered square rather than at the
document aspect so a ramp or tile is not stretched into the card. A loaded
pattern fill layer's preview SHALL be the pattern decoded from the document's
`Patt`/`Pat2`/`Pat3` resource rather than the grey placeholder, and its mask
card SHALL be present when the layer carries a layer mask. Mask and vector-mask cards on adjustment, fill, type and group rows SHALL
use the document aspect; the left content card keeps its existing geometry.

#### Scenario: Mask thumbnail adjacency [uip_layers_mask_adjacency]

- **WHEN** a layer with a mask and a vector mask is shown
- **THEN** the mask thumbnail is directly right of the image thumbnail and the vector mask directly right of the mask, each with a chain glyph

#### Scenario: Active thumbnail brackets [uip_layers_active_thumb]

- **WHEN** a layer thumbnail is clicked to become the edit target
- **THEN** brackets mark that thumbnail and a group row never shows brackets on its icon

#### Scenario: The chain glyph is larger and rotated [uip_layers_link_glyph]

- **WHEN** a masked or vector-masked row is shown, linked or unlinked
- **THEN** its chain glyph is drawn two pixels larger than the slot and rotated
  a quarter turn, so its ink is taller than it is wide

#### Scenario: An adjustment row shows its kind icon [lpr_thumb_adjustment_icon]

- **WHEN** an Invert, Levels, or other adjustment layer is shown
- **THEN** its thumbnail is a neutral card with that adjustment kind's icon and no checkerboard

#### Scenario: A fill row shows its content preview [lpr_thumb_fill_preview]

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is shown
- **THEN** its thumbnail shows the fill's solid colour, gradient ramp, or scaled pattern tile instead of a generic checkerboard

#### Scenario: A loaded pattern fill row shows its pattern and mask [lpr_thumb_pattern_fill_loaded]

- **WHEN** a PSD carrying a Pattern fill layer that references a Photoshop 24-slot pattern and has a painted layer mask is opened
- **THEN** the fill row's thumbnail shows the tiled pattern rather than the grey placeholder and the row carries a non-null mask thumbnail with the mask's light and dark coverage

#### Scenario: An adjustment row's card is 1:1 [lpr_thumb_adjustment_square]

- **WHEN** an adjustment layer is shown in a document whose width and height
  differ
- **THEN** its image thumbnail card is 1:1 (width equals height) rather than
  letterboxed to the document

#### Scenario: A fill preview is a 1:1 card [lpr_thumb_fill_square]

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is shown in a document
  whose width and height differ
- **THEN** its card is 1:1, the preview image the bridge supplies is square, and
  the preview fills the card without a document-aspect margin or distortion

#### Scenario: A mask thumbnail follows the document beside a square card [lpr_thumb_mask_square]

- **WHEN** an adjustment or fill layer carries a layer mask or a vector mask
- **THEN** the mask and vector-mask cards follow the document aspect while the
  left content thumbnail keeps its square card
