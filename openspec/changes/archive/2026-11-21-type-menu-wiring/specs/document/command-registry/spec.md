# Spec Delta

## MODIFIED Requirements

### Requirement: Layer management commands in the command table

The system SHALL declare the layer-management commands in the declarative
command table under stable `layer.*` identifiers and SHALL register a handler
for each implemented command. The declared set SHALL include
`layer.merge.layers` (Merge Down / Merge Layers), `layer.merge.visible`,
`layer.merge.clippingMask`, `layer.flatten.image`,
`layer.new.layerFromBackground`, `layer.new.backgroundFromLayer`,
`layer.new.layerViaCopy`, `layer.new.layerViaCut`, `layer.new.groupFromLayers`,
`layer.delete.hiddenLayers`, `layer.select.similar`, `layer.select.linked`,
`layer.link.layers`, `layer.unlink.layers`, `layer.hide.layers`,
`layer.rasterize.fillContent`, `layer.rasterize.type`, `layer.rasterize.layer`,
and `layer.rasterize.allLayers`. Each command that does not apply to the current
selection or document SHALL be disabled by an enable provider evaluated when the
menu is opened. The `Type` rasterize entry SHALL have a handler and SHALL be
enabled for a type layer. The `Shape`, `Vector Mask`, `Smart Object`, `Video`, and
`3D` rasterize entries SHALL remain in the table with no handler and SHALL stay
visible and disabled. Changing a label or shortcut SHALL NOT change an
identifier.

#### Scenario: A registered management command dispatches

- **WHEN** a management menu item with a registered handler is triggered
- **THEN** its handler runs and performs the command's action

#### Scenario: Inapplicable commands are disabled on open

- **WHEN** a document or selection makes a management command inapplicable and
  the menu is opened
- **THEN** that command is greyed rather than hidden, and it becomes enabled
  without rebuilding the menu when it later applies

#### Scenario: The Type rasterize entry is enabled for type [cr_type_rasterize]

- **WHEN** a type layer is current and the Rasterize submenu is opened
- **THEN** `Type` is enabled and dispatches the type rasterizer

#### Scenario: A kind-less rasterize variant is present but disabled

- **WHEN** the Rasterize submenu is built
- **THEN** Shape, Vector Mask, Smart Object, Video, and 3D appear with no
  handler and are disabled
