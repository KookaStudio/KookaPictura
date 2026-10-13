# layers-panel Specification (delta)

## MODIFIED Requirements

### Requirement: Panel and row menus

The system SHALL provide a panel menu and a row context menu. The panel menu
SHALL offer `Panel Options…`, New Layer, New Group, Duplicate Layer(s), Delete
Layer(s), Group Layers, Ungroup Layers, Move Layer Up, and Move Layer Down;
commands the system does not yet implement SHALL appear disabled with a
`— not implemented yet` tooltip rather than being hidden.

The row context menu SHALL be assembled **per layer kind**. Each kind (pixel,
background, group, adjustment, type, shape, smart object) SHALL be offered the
commands that apply to it, and SHALL NOT be offered commands that do not apply
(for example, a group or adjustment row SHALL NOT offer `Export As…` or `Quick
Export as PNG`, and a group row SHALL NOT offer `Fill`-related content). Every
applicable command that is not yet implemented SHALL still appear, rendered
disabled, carrying a `— not implemented yet` tooltip, so that implementing a
command enables an existing row rather than adding one. The commands common to
every kind SHALL include Rename and a color-label submenu containing `None`,
`Red`, `Orange`, `Yellow`, `Green`, `Blue`, `Violet`, and `Gray`.

A shape row SHALL, in addition to the common commands, offer **Copy Shape
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

For a pixel row the menu SHALL offer **Add Layer Mask**, **Delete Layer Mask**,
**Enable Layer Mask**, and **Disable Layer Mask**, each wired to the layer-mask
bridge: Add Layer Mask SHALL add a `reveal-selection` mask when a selection
exists and a `reveal-all` mask otherwise; Delete Layer Mask SHALL remove the
mask; and Enable/Disable Layer Mask SHALL set the mask's enabled state. Each
applied action SHALL be one undoable step.

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
- **THEN** it contains Export As… and Quick Export as PNG

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
  commands, and it offers `Ungroup Layers`

#### Scenario: A type row offers rasterize

- **WHEN** the user opens the row menu for a type layer
- **THEN** it offers `Rasterize Type`

#### Scenario: An unimplemented applicable command is disabled, not absent

- **WHEN** the user opens the row menu for a kind and a command applies to that
  kind but is not yet implemented
- **THEN** the command is present, disabled, and shows the `— not implemented
  yet` tooltip
