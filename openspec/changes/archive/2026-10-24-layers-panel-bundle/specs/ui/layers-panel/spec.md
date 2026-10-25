# Spec Delta — ui/layers-panel

## MODIFIED Requirements

### Requirement: Layer operations

The system SHALL offer New Layer, New Group, Duplicate Layer(s), Delete
Layer(s), Group Layers, Ungroup Layers, and Move Layer Up/Down from the panel
and its menus, applying them to the active document through the bridge, and the
document's rows SHALL update afterwards. A new layer SHALL be an empty, fully
transparent raster layer at the document size and SHALL NOT change the
composite. A new group SHALL be an empty group with a Normal blend. New
Layer/New Group SHALL insert directly above the selected layer, or, when the
selected row is a group, as that group's topmost child; with no selection they
SHALL insert at the top of the stack, and the new row SHALL become selected.
The same insertion rule SHALL apply to an adjustment or fill layer created from
the Layers panel, the Adjustments panel, or a `Layer` menu, and the created
node SHALL become selected. Duplicating SHALL deep-copy each selected layer —
children, mask, adjustment data, and all attributes included — directly above
itself, naming the copy `"<name> copy"`, except that the copy of the Background
SHALL be an ordinary layer: not the Background, unlocked, and with an opaque
alpha channel. Deleting SHALL remove every selected, eligible layer. Move
Layer Up/Down SHALL swap the selected layer with its neighbour within its own
container and MUST refuse a move that would place the layer below the
Background. Each applied operation SHALL mark the document modified and be one
undoable step.

#### Scenario: Add a layer

- **WHEN** the user clicks New Layer with a layer selected
- **THEN** an empty transparent layer is inserted directly above it, becomes selected, the row count grows by one, and the composite is unchanged

#### Scenario: Add a layer inside a selected group [m39_tree]

- **WHEN** the user clicks New Layer with a group selected
- **THEN** an empty transparent layer is inserted as that group's topmost child
  and becomes the selected row

#### Scenario: Add a group

- **WHEN** the user clicks New Group with no layer selected
- **THEN** an empty group is inserted at the top of the stack, becomes selected, and a new row appears

#### Scenario: Duplicate a layer

- **WHEN** the user duplicates the selected layer
- **THEN** a deep copy named `"<name> copy"` is inserted directly above it, the row count grows by one, and the copy becomes selected

#### Scenario: Delete a layer

- **WHEN** the user deletes the selected layer
- **THEN** the layer is removed and the panel updates

#### Scenario: Move a layer

- **WHEN** the user moves the selected layer up or down from the panel or row menu
- **THEN** its order within its container changes, the rows reorder, and the composite updates

#### Scenario: Move Down at the Background floor is refused

- **WHEN** the layer directly above the Background is moved down
- **THEN** the order is unchanged, no history state is added, and the Background
  stays bottom-most

#### Scenario: Duplicating the Background makes an ordinary layer

- **WHEN** the Background is duplicated
- **THEN** `Background copy` sits above it as an unlocked pixel layer with no lock badge, and the original stays the Background

#### Scenario: An adjustment inserts above the active layer [lpo_anchor_adjustment]

- **WHEN** a layer in the middle of a group is selected and an adjustment layer is created
- **THEN** the adjustment is inserted immediately above that layer inside the same group, becomes selected, and is one undo state

#### Scenario: A fill inserts above the active layer [lpo_anchor_fill]

- **WHEN** a layer in the middle of the stack is selected and a Solid Color or Gradient fill layer is created
- **THEN** the fill is inserted immediately above that layer in the same container, becomes selected, and is one undo state

### Requirement: Panel Options

The system SHALL provide a `Panel Options…` dialog from the panel menu with a
thumbnail size (`None`, `Small`, `Medium`, `Large`), thumbnail contents
(`Entire Document`, `Layer Bounds`), an `Expand New Effects` toggle, an
`Add "copy" to Copied Layers and Groups` toggle, and a `Use Default Masks on
Fill Layers` toggle. The defaults SHALL be `Medium`, `Entire Document`, and
enabled for the three toggles respectively, and all five values SHALL persist in
the session store, with an older or missing value loading the default. Thumbnail
contents SHALL be resolved when the thumbnail is built: `Layer Bounds` fills the
thumbnail from the layer's own bounds, while `Entire Document` places the
layer's content at its document position scaled into a document-sized thumbnail.
`Expand New Effects` SHALL have no visible effect until effect rows exist. When
`Add "copy"` is enabled, a duplicated layer or group SHALL be named
`"<name> copy"`; when disabled, the copy SHALL keep the original's name. When
`Use Default Masks on Fill Layers` is enabled, a fill layer created from the
panel or a `Layer` menu SHALL receive the active selection as a layer mask when
one exists, and otherwise a white document-sized layer mask; when disabled, the
new fill layer SHALL have no mask. The `Add Mask by Default` preference itself
belongs to the Adjustments panel capability and SHALL NOT be controlled by
`Panel Options`.

#### Scenario: Defaults on a fresh install [m39_options]

- **WHEN** the panel options are read with no saved session values
- **THEN** the thumbnail size is Medium, the thumbnail contents is Entire Document, Expand New Effects is enabled, Add "copy" is enabled, and Use Default Masks is enabled

#### Scenario: Thumbnail contents changes the thumbnail [m39_options]

- **WHEN** a small layer in a large document is shown with Layer Bounds and then with Entire Document
- **THEN** the Layer Bounds thumbnail is filled by the layer while the Entire Document thumbnail shows the layer smaller in its document context

#### Scenario: Options persist [m39_options]

- **WHEN** the user changes any panel-option value and the session is saved and reloaded
- **THEN** the panel restores those values, including Add "copy" and Use Default Masks

#### Scenario: The copy toggle names a duplicate [lpo_copy_name]

- **WHEN** `Add "copy"` is enabled and a layer named `Base` is duplicated
- **THEN** the copy is named `Base copy`

#### Scenario: Disabling the copy toggle keeps the source name [lpo_copy_name_off]

- **WHEN** `Add "copy"` is disabled and a layer named `Base` is duplicated
- **THEN** the copy keeps the name `Base`

#### Scenario: A fill layer takes the selection as a default mask [lpo_default_mask]

- **WHEN** `Use Default Masks` is enabled, a selection is active, and a fill layer is created
- **THEN** the created fill layer carries a layer mask derived from the selection

#### Scenario: A fill layer without a selection gets a white mask [lpo_default_mask_white]

- **WHEN** `Use Default Masks` is enabled, no selection is active, and a fill layer is created
- **THEN** the created fill layer carries a white document-sized layer mask

#### Scenario: A fill layer created without the option has no mask [lpo_default_mask_off]

- **WHEN** `Use Default Masks` is disabled, a selection is active, and a fill layer is created
- **THEN** the created fill layer has no layer mask

### Requirement: Panel and row menus

The system SHALL provide a panel menu and a row context menu. The panel menu
SHALL offer `Panel Options…`, New Layer, New Group, Duplicate Layer(s), Delete
Layer(s), Group Layers, Ungroup Layers, Move Layer Up, and Move Layer Down;
commands the system does not yet implement SHALL appear disabled with a
`— not implemented yet` tooltip rather than being hidden.

The row context menu SHALL be assembled **per layer kind** and SHALL emit one
separator between adjacent non-empty command blocks, in this order:

1. Blending Options… (style-capable rows) and Edit Adjustment… (adjustment
   rows).
2. Duplicate Layer(s), Delete Layer(s), Group from Layers…, New Layer, New
   Group, Ungroup Layers, Move Layer Up, and Move Layer Down.
3. Quick Export as PNG, then Export As… (pixel and smart-object rows only).
4. Convert to Smart Object (pixel and Background rows).
5. Rasterize Type, Rasterize Shape, Rasterize Layer, and Rasterize Layer Style
   (applicable kinds; the style row is not yet implemented and is disabled).
6. Add/Delete/Enable/Disable Layer Mask, Add/Delete/Enable/Disable Vector Mask,
   and Create/Release Clipping Mask.
7. Link Layers (or Unlink Layers when the row is linked) and Select Linked
   Layers.
8. Copy Layer Style, Paste Layer Style, and Clear Layer Style.
9. Copy Shape Attributes, Paste Shape Attributes, and Rasterize Shape.
10. Release from Isolation, only while isolation is active, near the bottom of
    the menu.
11. Merge Down (or Merge Layers for a multi-selection), Merge Visible, Merge
    Clipping Mask, and Flatten Image.
12. The Color Label submenu.

Each kind (pixel, background, group, adjustment, type, shape, smart object)
SHALL be offered the commands that apply to it, and SHALL NOT be offered
commands that do not apply. Every applicable command that is not yet implemented
SHALL still appear, rendered disabled, carrying a `— not implemented yet`
tooltip, so that implementing a command enables an existing row rather than
adding one. The commands common to every kind except the Background SHALL
include Rename and a color-label submenu containing `None`, `Red`, `Orange`,
`Yellow`, `Green`, `Blue`, `Violet`, and `Gray`.

The **Background row SHALL use a brief dedicated menu**: `Layer from
Background…`, `Duplicate Layer…`, `Delete Layer` (disabled), `Quick Export as
PNG`, `Export As…`, and `Convert to Smart Object`. It SHALL NOT offer Rename,
Blending Options…, the color-label submenu, merge/flatten, or any other row.

A shape row SHALL, in addition to its kind's commands, offer **Copy Shape
Attributes**, **Paste Shape Attributes**, and **Rasterize Shape**, each wired to
the shape bridge and acting on that row. A shape row SHALL be resolved by its
shape-layer projection (a fill cut to a vector mask), not by its adjustment
kind, so it is not offered the adjustment-only `Edit Adjustment…` row.

A pixel row SHALL offer **Convert to Smart Object**, wired to the convert bridge
and acting on that row; it SHALL be enabled only when the row is a convertible
raster pixel layer, and a successful conversion SHALL select the new smart
object. A smart-object row SHALL, in addition to the common and pixel commands,
offer **Reset Transform**, **Convert to Layers**, **New Smart Object via Copy**,
**Edit Contents**, **Replace Contents…**, and **Export Contents…**, each wired
to its bridge and acting on that row, plus the not-yet-implemented
**Relink to File…** and **Convert to Linked…** rows rendered disabled with the
`— not implemented yet` tooltip. Reset Transform and Convert to Layers SHALL be
enabled only when the row's embedded source parses as a PSD/PSB document; New
Smart Object via Copy SHALL be enabled for a non-group, non-adjustment
`Embedded` smart object with a payload; Edit Contents SHALL be enabled only when
the row's embedded source parses as a PSD/PSB document; Replace Contents SHALL
be enabled only for a replaceable `Embedded` object with a payload; and Export
Contents SHALL be enabled only for a smart object with a non-empty payload. A
non-smart row SHALL NOT offer the smart-object-only rows. Each applied action
SHALL be one undoable step.

For a pixel row and a group row the menu SHALL offer **Add Layer Mask**,
**Delete Layer Mask**, **Enable Layer Mask**, and **Disable Layer Mask**, each
wired to the layer-mask bridge: Add Layer Mask SHALL add a `reveal-selection`
mask when a selection exists and a `reveal-all` mask otherwise; Delete Layer
Mask SHALL remove the mask; and Enable/Disable Layer Mask SHALL set the mask's
enabled state. A pixel row SHALL also offer **Add Vector Mask**, **Delete
Vector Mask**, **Enable Vector Mask**, and **Disable Vector Mask**, each wired
to the vector-mask bridge and gated by vector-mask presence. A pixel row SHALL
offer **Create Clipping Mask** when the row can clip, and **Release Clipping
Mask** when it is clipped. Each applied action SHALL be one undoable step.

A pixel or group row SHALL offer **Merge Down** when a single row is targeted,
**Merge Layers** when several rows are targeted, **Merge Visible**, **Merge
Clipping Mask**, and **Flatten Image**, wired to their merge bridges; Merge
Down/Merge Layers SHALL be enabled only where the selection can merge. A pixel
row SHALL offer **Link Layers** when it is not linked and **Unlink Layers** when
it is, plus **Select Linked Layers**. **Group from Layers…** SHALL open the
Group from Layers dialog for the row's selection. A row with a layer style
SHALL offer the style rows. **Rasterize Type** SHALL rasterize a type row,
**Rasterize Shape** a shape row, and **Rasterize Layer** a smart-object or
fill-content row; a plain pixel row and a group SHALL hide the rasterize rows.
**Rasterize Layer Style** SHALL appear disabled with the `— not implemented
yet` tooltip on a styled row. Right-clicking an unselected row SHALL select it
before a row command runs, so every row action acts on the clicked row.

Right-clicking the visibility toggle SHALL offer show/hide this layer only and
show/hide all. The bottom action strip SHALL remain exactly the CS6 seven
buttons, so Move Up/Down SHALL NOT appear in the strip.

#### Scenario: The panel menu contains the wired commands [m39_menus]

- **WHEN** the panel menu is opened
- **THEN** it contains Panel Options, New Layer, New Group, Duplicate, Delete,
  Group, Ungroup, Move Up, and Move Down; commands that are not yet implemented
  are shown disabled with the `— not implemented yet` tooltip

#### Scenario: The row menu sets a color label [m39_menus]

- **WHEN** the user right-clicks a row and picks Red from the color-label submenu
- **THEN** that layer's color label becomes Red

#### Scenario: Move Up/Down reorders from the menu [m39_menus]

- **WHEN** the user picks Move Layer Up for a row that is not the top of its container
- **THEN** the layer swaps with the layer above it within its container in one undo step

#### Scenario: The action strip has no Move buttons [m39_strip]

- **WHEN** the Layers panel is shown
- **THEN** its bottom strip contains exactly the seven CS6 buttons and no Move Up
  or Move Down button

#### Scenario: Pixel and smart-object rows offer export

- **WHEN** the user opens the row menu for a pixel layer or a smart-object layer
- **THEN** it contains Export As… and Quick Export as PNG, with Quick Export first

#### Scenario: Group and adjustment rows hide export

- **WHEN** the user opens the row menu for a group or an adjustment layer
- **THEN** it does not offer Export As… or Quick Export as PNG

#### Scenario: Type rows hide export

- **WHEN** the user opens the row menu for a type layer
- **THEN** it does not offer Export As… or Quick Export as PNG

#### Scenario: A shape row offers its shape commands

- **WHEN** the user opens the row menu for a shape layer
- **THEN** it contains Copy Shape Attributes, Paste Shape Attributes, and
  Rasterize Shape, enabled and wired to the shape bridge

#### Scenario: A shape row is not offered the adjustment edit

- **WHEN** the user opens the row menu for a shape layer
- **THEN** it does not contain Edit Adjustment…

#### Scenario: A pixel row offers Convert to Smart Object [lpr_convert_row]

- **WHEN** the user opens the row menu for a convertible raster pixel layer
- **THEN** it contains Convert to Smart Object, enabled and wired to the convert
  bridge, and picking it converts the layer and selects the new smart object

#### Scenario: An existing smart object disables Convert to Smart Object

- **WHEN** the user opens the row menu for a layer that is already a smart object
- **THEN** it offers Convert to Smart Object, disabled, because the row is not a
  convertible raster pixel layer

#### Scenario: A smart-object row offers its smart-object commands [lpr_smart_rows]

- **WHEN** the user opens the row menu for a smart-object layer
- **THEN** it contains Reset Transform, Convert to Layers, New Smart Object via
  Copy, Edit Contents, Replace Contents…, and Export Contents…, enabled and wired
  to their bridges

#### Scenario: A smart-object row offers the linked rows disabled [lpr_smart_linked]

- **WHEN** the user opens the row menu for a smart-object layer
- **THEN** it contains Relink to File… and Convert to Linked…, each disabled and
  showing the `— not implemented yet` tooltip

#### Scenario: A plain pixel row omits the smart-object commands

- **WHEN** the user opens the row menu for a plain pixel layer
- **THEN** it does not contain Reset Transform, Convert to Layers, New Smart
  Object via Copy, Edit Contents, Replace Contents…, Export Contents…, Relink to
  File…, or Convert to Linked…

#### Scenario: A pixel row offers its full command set

- **WHEN** the user opens the row menu for a pixel layer
- **THEN** it contains the common commands, the color-label submenu, `Export
  As…`, `Quick Export as PNG`, and the pixel-applicable mask, style, clipping,
  smart-object, and rasterize commands — each either enabled or disabled with a
  tooltip when not yet implemented

#### Scenario: The row menu adds and deletes a mask [lmk_rowmenu]

- **WHEN** the user picks Add Layer Mask for a pixel row, then Delete Layer Mask
- **THEN** a mask is added and then removed from the active layer, each in one undo step

#### Scenario: The row menu toggles mask enablement [lmk_rowmenu_enable]

- **WHEN** the user picks Enable Layer Mask or Disable Layer Mask for a masked layer
- **THEN** the mask's disabled bit is cleared or set in one undo step

#### Scenario: A group row omits content commands

- **WHEN** the user opens the row menu for a group row
- **THEN** it does not offer `Export As…`, `Quick Export as PNG`, or fill
  commands, it offers `Ungroup Layers`, and it offers the layer-mask rows

#### Scenario: A type row offers rasterize

- **WHEN** the user opens the row menu for a type layer
- **THEN** it offers `Rasterize Type`

#### Scenario: An unimplemented applicable command is disabled, not absent

- **WHEN** the user opens the row menu for a kind and a command applies to that
  kind but is not yet implemented
- **THEN** the command is present, disabled, and shows the `— not implemented
  yet` tooltip

#### Scenario: The row blocks appear in CS6 order [lpr_row_blocks]

- **WHEN** the row menu is opened for a pixel layer with an editable style, a mask, and a selection that can merge
- **THEN** Blending Options, the layer operations, Quick Export/Export As, Convert to Smart Object, Rasterize, the mask and clipping rows, link rows, style rows, merge rows, and the color label appear in that order, with one separator between each non-empty block

#### Scenario: The Background row menu is brief [lpr_background_brief]

- **WHEN** the user opens the row menu for the Background layer
- **THEN** it contains only Layer from Background…, Duplicate Layer…, Delete Layer (disabled), Quick Export as PNG, Export As…, and Convert to Smart Object, and it does not contain Rename, Blending Options…, a color label, merge, or flatten

#### Scenario: A group row offers the mask and merge rows [lpr_group_rows]

- **WHEN** the user opens the row menu for a group
- **THEN** it offers Add/Delete/Enable/Disable Layer Mask, Link/Unlink Layers, Select Linked Layers, Merge Down/Merge Layers, Merge Visible, Merge Clipping Mask, and Flatten Image, and it does not offer export or fill rows

#### Scenario: A pixel row offers clipping and vector-mask rows [lpr_pixel_mask_rows]

- **WHEN** the user opens the row menu for a pixel layer that can clip
- **THEN** it offers Create Clipping Mask and the four vector-mask rows, and a clipped row offers Release Clipping Mask instead of Create Clipping Mask

#### Scenario: The row menu links and selects linked layers [lpr_link_rows]

- **WHEN** the user opens the row menu for an unlinked pixel row
- **THEN** it offers Link Layers and Select Linked Layers; once linked, the row offers Unlink Layers

#### Scenario: The row menu offers the merge family [lpr_merge_rows]

- **WHEN** the user opens the row menu for a single pixel row above another layer
- **THEN** it offers Merge Down, Merge Visible, Merge Clipping Mask, and Flatten Image, and a multi-selection offers Merge Layers instead of Merge Down

#### Scenario: The row menu rasterizes by kind [lpr_rasterize_rows]

- **WHEN** the user opens the row menu for a type layer, a shape layer, a fill layer, a smart object, and a plain pixel layer
- **THEN** the first four offer Rasterize Type, Rasterize Shape, Rasterize Layer, and Rasterize Layer respectively, the plain pixel row hides the rasterize rows, and a styled row also offers the disabled Rasterize Layer Style

#### Scenario: Release from Isolation sits near the bottom [lpr_isolation_row]

- **WHEN** isolation is active and the row menu is opened
- **THEN** Release from Isolation appears after the style and shape blocks and before the merge block

#### Scenario: A row action acts on the clicked row [lpr_row_action_target]

- **WHEN** the user right-clicks an unselected row and picks a row action
- **THEN** the panel selects that row first and the action applies to it

### Requirement: Layer style panel integration

The Layers panel SHALL be the surface that reaches the layer-style feature. A
row double-click outside the name and the eye/mask controls SHALL open the Layer
Style dialog on Blending Options for that layer when the layer can carry a style
(`layer_style_can_edit`), and SHALL do nothing otherwise. The row context menu
SHALL offer **Blending Options…**, **Copy Layer Style**, **Paste Layer Style**,
and **Clear Layer Style** as implemented commands: Blending Options SHALL appear
for pixel, group, and type rows and SHALL be enabled only where the layer can
carry a style; Copy and Clear SHALL be enabled when the row's layer carries a
style; Paste SHALL be enabled when the app-wide style clipboard is non-empty.
Choosing a command SHALL act on the row's layer through the layer-style bridge
and SHALL record through the bridge's own history. The Background's brief row
menu SHALL NOT offer Blending Options or the style clipboard rows.

An `Alt`-click on a styled-or-adjustment row's right-edge `fx` region SHALL
toggle every layer's effects through `layer_style_set_all_visible`, showing
them when all were hidden and hiding them when any were shown, as one undoable
step.

The filter row's **Effect** dimension SHALL be enabled and SHALL list the ten
effect keys from `layer_style_effect_names()`; selecting an effect SHALL match
the rows whose layer carries that effect (and, as with the other dimensions,
their ancestors), and a row whose layer does not carry it SHALL be filtered out.

#### Scenario: A double-click opens Blending Options

- **WHEN** a pixel layer row is double-clicked outside its name and its eye/mask controls
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: A background double-click does not open a style

- **WHEN** the Background row is double-clicked outside its name
- **THEN** the Background conversion path runs and no Layer Style dialog opens

#### Scenario: The style rows report their enablement

- **WHEN** the row menu is opened for a pixel layer that carries a color overlay and has a copied style available
- **THEN** Blending Options, Copy Layer Style, Paste Layer Style, and Clear Layer Style are all enabled

#### Scenario: An unstyled pixel row disables Copy and Clear

- **WHEN** the row menu is opened for a pixel layer with no layer style
- **THEN** Blending Options is enabled and Copy Layer Style, Paste Layer Style, and Clear Layer Style are disabled

#### Scenario: An adjustment row omits the style rows

- **WHEN** the row menu is opened for an adjustment layer
- **THEN** it does not offer Blending Options, Copy Layer Style, Paste Layer Style, or Clear Layer Style

#### Scenario: The row menu clears a row's style

- **WHEN** Clear Layer Style is chosen for a pixel row that carries a style
- **THEN** the row's style is removed in one undoable step and its fx badge disappears

#### Scenario: Alt-clicking the fx region toggles all effects

- **WHEN** the user `Alt`-clicks the fx badge of a styled row while effects are shown
- **THEN** every layer's effects are hidden in one undoable step, and a second `Alt`-click shows them again

#### Scenario: The Effect dimension matches styled rows

- **WHEN** the filter uses the Effect dimension with an effect a layer carries
- **THEN** that layer's row is shown and rows without the effect are hidden

#### Scenario: The Effect dimension lists the effect keys

- **WHEN** the filter's Effect dimension is selected
- **THEN** its choices are the ten effect keys from the layer-style effect list

### Requirement: Layers panel thumbnail layout

Within a layer row the mask thumbnail SHALL sit immediately to the right of the
image thumbnail with a chain glyph between them (a broken-chain glyph when the
link is off), and a vector-mask thumbnail SHALL sit immediately to the right of
the mask thumbnail with the same chain glyph between them. The mask and vector
thumbnails SHALL have the same aspect ratio as the image thumbnail. The chain
glyph SHALL be drawn two pixels larger than its slot and rotated a quarter turn
so it reads vertically between the side-by-side thumbnails, in both the linked
and unlinked states. A row's active edit target SHALL be marked with brackets
around that thumbnail, and clicking a thumbnail SHALL make it the active target;
Alt+clicking a thumbnail SHALL also activate it in the composite. A group row
SHALL NOT draw brackets around its icon. An **adjustment row's image thumbnail
SHALL be a neutral card bearing the icon for its decoded adjustment kind**, with
no checkerboard; a **fill row's image thumbnail SHALL show its content preview**
(a flat solid swatch, a gradient ramp, or a scaled pattern tile) in the same
letterboxed geometry as a pixel thumbnail.

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
