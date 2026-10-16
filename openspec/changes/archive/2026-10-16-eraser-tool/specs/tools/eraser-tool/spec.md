## ADDED Requirements

### Requirement: Eraser tool

The Eraser SHALL erase the active pixel layer along the drag, recording exactly
one "Eraser" history state per stroke that changed pixels. On an ordinary layer
it SHALL erase to transparency, multiplying each pixel's alpha down by the
stroke's coverage times Opacity; on the Background, a layer without an alpha
channel, or a layer with Lock Transparency on, it SHALL paint the background
colour instead. It SHALL refuse a layer whose pixels are locked.

#### Scenario: Erasing the Background

- **WHEN** the `eraser_tool` self-test drags the Eraser across the opened Background with a red background colour
- **THEN** one "Eraser" state is recorded and the erased pixels are red

#### Scenario: Erasing an ordinary layer

- **WHEN** the Eraser drags over black paint on a new layer above a white Background
- **THEN** the erased pixels are transparent and the white Background shows through

### Requirement: Eraser modes

The Mode menu SHALL offer Brush (the Brush tip), Pencil (the hard aliased tip),
and Block (a hard square at full strength, ignoring Opacity and Flow, which the
bar greys out).

#### Scenario: Block erases a square

- **WHEN** a Block eraser dab lands on an ordinary layer at 10 % Opacity
- **THEN** the pixels of a 16 px square, corners included, are fully transparent

### Requirement: Erase To History

With Erase to History checked, or with Alt held at the press, the Eraser SHALL
paint the active layer back as it was in the History Brush's source state, and
SHALL refuse the stroke when that state has no pixel layer at the active path.

#### Scenario: Alt-drag paints the opening state back

- **WHEN** the Eraser is Alt-dragged over the red-erased Background
- **THEN** one "Eraser" state is recorded and the pixels under the drag are white again
