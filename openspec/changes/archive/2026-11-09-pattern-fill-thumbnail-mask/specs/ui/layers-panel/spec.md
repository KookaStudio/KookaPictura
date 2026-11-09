# Spec Delta

## MODIFIED Requirements

### Requirement: Layers panel content and mask thumbnail layout

Within a layer row the mask thumbnail SHALL sit immediately to the right of the
image thumbnail with a chain glyph between them (a broken-chain glyph when the
link is off), and a vector-mask thumbnail SHALL sit immediately to the right of
the mask thumbnail with the same chain glyph between them. The mask and vector
thumbnails SHALL follow the document aspect independently of the image
thumbnail and owning layer kind. The chain
glyph SHALL be drawn two pixels larger than its slot and rotated a quarter turn
so it reads vertically between the side-by-side thumbnails, in both the linked
and unlinked states. A row's active edit target SHALL be marked with brackets
around that thumbnail, and clicking a thumbnail SHALL make it the active target;
Alt+clicking a thumbnail SHALL also activate it in the composite. A group row
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
