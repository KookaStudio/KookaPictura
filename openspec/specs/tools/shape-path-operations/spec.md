# tools/shape-path-operations Specification

## Purpose
The shape options bar's Path Operations menu and boolean combining of the new
outline with an existing shape or Work Path.

## Requirements

### Requirement: Path operation menu

The shape options bar SHALL offer New Shape, Combine Shapes, Subtract From
Shape Area, Intersect Shape Areas, and Exclude Overlapping Shape Areas. The
chosen operation SHALL persist in the shared shape options.

#### Scenario: The menu lists the five operations

- **WHEN** a shape tool is active
- **THEN** its Path Operations menu is enabled and lists New Shape, Combine Shapes, Subtract From Shape Area, Intersect Shape Areas, and Exclude Overlapping Shape Areas

### Requirement: Shape-mode path operations

In Shape mode, a non-zero path operation SHALL combine the newly drawn outline
with the active shape layer's outline and replace that layer's outline in one
"Path Operation" state. Combine with no active shape layer SHALL create a new
shape layer; Subtract, Intersect, and Exclude with no target SHALL change
nothing. New Shape SHALL keep the existing new-layer behaviour.

#### Scenario: Subtract removes the overlap

- **WHEN** a rectangle shape layer is active, Subtract is chosen, and a second rectangle overlapping it is drawn
- **THEN** the active layer's outline is the first rectangle minus the second and the overlap shows the layer below

#### Scenario: Intersect keeps the overlap

- **WHEN** a rectangle shape layer is active, Intersect is chosen, and a second overlapping rectangle is drawn
- **THEN** the active layer's outline is the intersection of the two rectangles

#### Scenario: Exclude removes the shared area

- **WHEN** a rectangle shape layer is active, Exclude is chosen, and a second overlapping rectangle is drawn
- **THEN** the active layer's outline covers both rectangles except their overlap

#### Scenario: Combine with no target creates a layer

- **WHEN** no shape layer is active and Combine is chosen while a rectangle is drawn
- **THEN** a new shape layer with the rectangle outline is created

### Requirement: Path-mode path operations

In Path mode, a non-zero path operation SHALL combine the newly drawn outline
with the Work Path and replace it in one "Path Operation" state.

#### Scenario: Subtract on the Work Path

- **WHEN** the Work Path holds a rectangle, Subtract is chosen, and a second overlapping rectangle is drawn in Path mode
- **THEN** the Work Path becomes the difference of the two rectangles
