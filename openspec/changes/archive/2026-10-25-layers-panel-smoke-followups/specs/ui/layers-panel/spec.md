## ADDED Requirements

### Requirement: CS6 row-menu follow-ups for type, fill, and group rows

The Layers row context menu SHALL expose the CS6 commands for the row's kind and
SHALL render unimplemented applicable commands disabled with a
`— not implemented yet` tooltip.

Type rows SHALL add Horizontal and Vertical orientation, None/Sharp/Crisp/Strong/Smooth
anti-aliasing, Convert to Point Text or Convert to Paragraph Text, and Warp
Text…, all wired to the type bridge and acting on the clicked row. They SHALL
also offer Create Work Path and Convert to Shape disabled, Windows LCD and
Windows anti-aliasing disabled, and Copy CSS/Copy SVG, Artboard from Layers…,
Frame from Layers…, and Convert to Frame disabled. Type rows SHALL offer Quick
Export as PNG, Export As…, and an enabled Convert to Smart Object.

Fill and adjustment rows SHALL offer Add/Delete/Enable/Disable Layer Mask,
Add/Delete/Enable/Disable Vector Mask, Create/Release Clipping Mask, Merge
Layers, Quick Export as PNG, Export As…, and Convert to Smart Object. Convert to
Smart Object SHALL be disabled for a fill or adjustment target because the
engine refuses those kinds. A fill row SHALL use the `Merge Layers` label even
for a single row.

Group rows SHALL offer Quick Export as PNG, Export As…, Add/Delete/Enable/Disable
Vector Mask, Merge Layers, and an enabled Blending Options…; Convert to Smart
Object SHALL be expressed as disabled. Group effect pages remain governed by
`ui/layer-style-dialog`.

Duplicate and Delete labels SHALL be per kind and per selection size:
`Duplicate Layer…`/`Delete Layer` for one pixel, type, or adjustment/fill row,
`Duplicate Layers…`/`Delete Layers` for a multi-selection, and
`Duplicate Group…`/`Delete Group` for a group.

#### Scenario: A type row offers its CS6 text controls

- **WHEN** the user opens the row menu for a type layer
- **THEN** Horizontal, Vertical, the five anti-aliasing rows, Convert to
  Paragraph Text, and Warp Text… are enabled, and Create Work Path, Convert to
  Shape, Copy CSS, Copy SVG, and the Windows anti-aliasing rows are disabled

#### Scenario: A fill row offers its mask and merge rows

- **WHEN** the user opens the row menu for a solid-fill row above another layer
- **THEN** Delete Layer Mask, Add Vector Mask, Create Clipping Mask, Merge
  Layers, Quick Export as PNG, Export As…, and Rasterize Layer are present, and
  Convert to Smart Object and Copy CSS are disabled

#### Scenario: A group row offers export and Blending Options

- **WHEN** the user opens the row menu for a group
- **THEN** Quick Export as PNG, Export As…, Add Vector Mask, Merge Layers, and
  Blending Options… are present and Convert to Smart Object is disabled

#### Scenario: Per-kind Duplicate and Delete labels

- **WHEN** the user opens the row menu for a pixel row, a group, and a
  multi-selection
- **THEN** the labels read Duplicate Layer…/Delete Layer, Duplicate Group…/Delete
  Group, and Duplicate Layers…/Delete Layers respectively
