# Spec Delta

## MODIFIED Requirements

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
3. Quick Export as PNG, then Export As… (every non-Background kind).
4. Artboard/Frame placeholders, then Convert to Smart Object (pixel, Background, and
   type rows enabled; group and adjustment rows disabled).
5. Rasterize Type, Rasterize Shape, Rasterize Layer, and Rasterize Layer Style
   (applicable kinds; the style row is not yet implemented and is disabled).
6. Add Layer Mask (a submenu of Reveal All, Hide All, Reveal Selection, Hide
   Selection, and From Transparency), Delete/Enable/Disable Layer Mask, Add
   Vector Mask (a submenu of Reveal All, Hide All, and Current Path),
   Delete/Enable/Disable Vector Mask, and Create/Release Clipping Mask.
7. Link Layers (or Unlink Layers when the row is linked) and Select Linked
   Layers.
8. Copy Layer Style, Paste Layer Style, and Clear Layer Style.
9. Copy Shape Attributes and Paste Shape Attributes.
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
PNG`, `Export As…`, and `Convert to Smart Object`. Each of those rows SHALL map
to a registry command id. It SHALL NOT offer Rename, Blending Options…, the
color-label submenu, merge/flatten, or any other row.

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

For a pixel, group, or adjustment/fill row the menu SHALL offer the **Add Layer
Mask** submenu, **Delete Layer Mask**, **Enable Layer Mask**, and **Disable
Layer Mask**, each wired to the layer-mask bridge. The Add Layer Mask submenu
SHALL offer **Reveal All**, **Hide All**, **Reveal Selection**, **Hide
Selection**, and **From Transparency**; the two selection commands SHALL be
enabled only while a selection exists. The submenu SHALL be enabled only when
the clicked layer has no layer mask, Delete Layer Mask only when it has one,
Enable Layer Mask only when that mask is disabled, and Disable Layer Mask only
when it is enabled. A pixel, group, or adjustment/fill row SHALL also offer the
**Add Vector Mask** submenu (**Reveal All**, **Hide All**, **Current Path**),
**Delete Vector Mask**, **Enable Vector Mask**, and **Disable Vector Mask**,
each wired to the vector-mask bridge and gated by vector-mask presence. A pixel
or adjustment/fill row SHALL offer **Create Clipping Mask** when the row can
clip, and **Release Clipping Mask** when it is clipped. Each applied action
SHALL be one undoable step.

A pixel, group, or adjustment/fill row SHALL offer **Merge Down** when a single row is targeted,
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
yet` tooltip on a styled row.

Every row action that maps to a document command SHALL be a registry command:
the action SHALL carry the command's stable identifier, its enabled, checked,
and label state SHALL come from the registry, and triggering it SHALL dispatch
that identifier. Rename (an inline tree edit) and the Color Label submenu MAY
remain panel controls. The Background menu rows SHALL map to registry ids. For a
command offered on more than one surface, the surfaces that offer it SHALL agree
on its enabled, checked, and label state.

Right-clicking a row SHALL make it the active anchor before a row command runs:
an unselected row SHALL become the only selected row, and a row already inside a
multi-selection SHALL become the active path while every selected path stays
selected. A row command SHALL act on that anchor, and a selection-wide command
SHALL act on the whole selection.

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

#### Scenario: Group and adjustment rows offer export

- **WHEN** the user opens the row menu for a group or an adjustment layer
- **THEN** it offers Quick Export as PNG and Export As…

#### Scenario: Type rows offer export

- **WHEN** the user opens the row menu for a type layer
- **THEN** it offers Quick Export as PNG and Export As…

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

- **WHEN** the user picks Add Layer Mask > Reveal All for a pixel row, then Delete Layer Mask
- **THEN** a mask is added and then removed from the active layer, each in one undo step

#### Scenario: The Add Layer Mask submenu creates a mask [lpr_add_mask_submenu]

- **WHEN** the user opens the Add Layer Mask submenu on a pixel row and picks Reveal All, then picks Reveal Selection with a selection active
- **THEN** the first adds a reveal-all mask and the second adds a reveal-selection mask, each in one undo step, and Reveal Selection is disabled while no selection exists

#### Scenario: The Add Vector Mask submenu creates a vector mask [lpr_add_vector_mask_submenu]

- **WHEN** the user opens the Add Vector Mask submenu on a pixel row and picks Reveal All
- **THEN** a reveal-all vector mask is added in one undo step

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
- **THEN** it offers the Add/Delete/Enable/Disable Layer Mask rows, the Add/Delete/Enable/Disable Vector Mask rows, Quick Export/Export As, Link/Unlink Layers, Select Linked Layers, Merge Layers/Merge Visible/Merge Clipping Mask, and Flatten Image, and it does not offer fill rows

#### Scenario: A pixel row offers clipping and vector-mask rows [lpr_pixel_mask_rows]

- **WHEN** the user opens the row menu for a pixel layer that can clip
- **THEN** it offers Create Clipping Mask and the Add/Delete/Enable/Disable Vector Mask rows, and a clipped row offers Release Clipping Mask instead of Create Clipping Mask

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

#### Scenario: Right-clicking inside a selection makes the row the anchor [lpr_row_action_anchor]

- **WHEN** two layers are selected and the user right-clicks the second and picks a layer-specific command
- **THEN** both layers stay selected, the clicked row becomes the active path, and the command applies to it

#### Scenario: A row action carries its command id [lpr_row_action_registry]

- **WHEN** the row menu is opened for a pixel layer
- **THEN** every implemented row action that maps to a document command carries the registry command identifier, and its enabled, checked, and label state equals the registry's
