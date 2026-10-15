# layers-panel Specification (delta)

## ADDED Requirements

### Requirement: A Background row offers Convert to Smart Object

The Layers-panel row context menu SHALL offer **Convert to Smart Object** on a
Background row, in addition to a pixel row, wired to the convert bridge and
acting on that row. It SHALL be enabled only when the row is a convertible raster
pixel or Background layer. A successful conversion SHALL refresh the panel and
select the row, which is no longer a Background.

#### Scenario: A Background row offers the command

- **WHEN** the user opens the row context menu for the Background layer
- **THEN** it contains Convert to Smart Object, enabled

#### Scenario: A group row does not offer the command

- **WHEN** the user opens the row context menu for a group layer
- **THEN** it does not contain Convert to Smart Object

#### Scenario: The Background row converts and reselects

- **WHEN** the user picks Convert to Smart Object on the Background row
- **THEN** the row becomes a smart object, is no longer a Background, one undo state is recorded, and the panel reselects the row
