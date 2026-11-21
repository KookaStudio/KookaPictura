# tools/shape-fill-stroke-options Specification

## Purpose
The shape tools' Fill and Stroke appearance: the four fill and stroke sources,
stroke line types, mirroring, and attribute copy/paste.

## ADDED Requirements

### Requirement: Fill and stroke sources

A shape layer SHALL support Fill None, Solid Color, Gradient, or Pattern and
Stroke None, Solid Color, Gradient, or Pattern, with a stroke width in pixels
and Align Inside, Center, or Outside. A Gradient source SHALL reuse the
Gradient Fill layer's gradient parameters; a Pattern source SHALL reuse the
Pattern Fill layer's pattern parameters. New shapes SHALL take the bar's Fill
and Stroke, and editing them while a shape layer is active SHALL restyle that
layer with one history state per changed aspect.

#### Scenario: Gradient and pattern fill on a new shape

- **WHEN** the shape options bar is set to a Gradient fill and a rectangle is drawn in Shape mode
- **THEN** the new shape layer is filled with that gradient inside its outline

#### Scenario: Gradient and pattern stroke on a new shape

- **WHEN** the shape options bar is set to a Pattern stroke and a rectangle is drawn in Shape mode
- **THEN** the new shape layer's outline is stroked with that pattern at the bar's width and Align

#### Scenario: Restyling the active shape mirrors the bar

- **WHEN** a shape layer with a solid fill is selected and the bar is changed to a Gradient fill and a Dashed stroke
- **THEN** the layer shows the gradient fill and the dashed stroke, the bar mirrors them when the layer is reselected, and each change records one state

### Requirement: Stroke line types

The Stroke type pop-up SHALL offer Solid, Dashed, and Dotted. A dashed or
dotted stroke SHALL render as gaps along the shape's outline while keeping the
configured width and Align.

#### Scenario: Dashed stroke has gaps

- **WHEN** a rectangle shape is stroked 4 px dashed and the composite is sampled along its top edge
- **THEN** some sampled pixels carry the stroke colour and some show the layer below

#### Scenario: Dotted stroke has smaller marks

- **WHEN** a rectangle shape is stroked with the Dotted line type
- **THEN** the stroke is a sequence of short marks rather than a continuous band

### Requirement: Appearance copy and paste

Copy Shape Attributes SHALL copy the fill and stroke source, parameters, width,
Align, and line type; Paste Shape Attributes SHALL replace the target shape
layer's appearance with them, one state when it changes.

#### Scenario: Copy and paste a gradient stroke

- **WHEN** shape A has a gradient fill and a dashed pattern stroke, its attributes are copied, and they are pasted onto shape B
- **THEN** shape B's fill and stroke match shape A's, including the line type
