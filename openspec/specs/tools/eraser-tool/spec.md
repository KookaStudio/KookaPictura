# tools/eraser-tool Specification

## Purpose
The Eraser tool: erase pixels to transparency along a brushed stroke.

## Requirements

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
and Block (a hard square of 16 screen pixels at full strength, ignoring
Opacity, Flow, and the brush preset's shape dynamics, which the bar greys out).
The Block square's side in document pixels SHALL be 16 divided by the canvas
zoom, rounded to the nearest whole document pixel and never less than one
document pixel, and a Block dab on a layer SHALL cover exactly that many
document pixels along each axis, whatever the dab's position in its pixel. A
Block dab on a layer mask paints the mask's coverage with the brush tip, as the
mask's own rules say.

#### Scenario: Block erases a square

- **WHEN** a Block eraser dab lands on an ordinary layer at 10 % Opacity and a 100 % zoom
- **THEN** the pixels of a 16 document pixel square, corners included, are fully transparent, and the pixels just outside the square are untouched

#### Scenario: The Block square scales with the zoom [er_block_zoom]

- **WHEN** the canvas is zoomed to 400 % and a Block eraser dab lands
- **THEN** the square covers 4 document pixels, so it is 16 screen pixels wide
- **WHEN** the canvas is zoomed to 25 % and a Block eraser dab lands
- **THEN** the square covers 64 document pixels, so it is 16 screen pixels wide

### Requirement: Erase To History

With Erase to History checked, or with Alt held at the press, the Eraser SHALL
paint the active layer back as it was in the History Brush's source state, and
SHALL refuse the stroke when that state has no pixel layer at the active path.

#### Scenario: Alt-drag paints the opening state back

- **WHEN** the Eraser is Alt-dragged over the red-erased Background
- **THEN** one "Eraser" state is recorded and the pixels under the drag are white again

### Requirement: The Block cursor is a square of the block's size

While the Eraser's Mode is Block and the pointer is over the canvas, the system
SHALL draw a square outline of the block's size at the pointer position — the
same number of document pixels along each axis that a Block dab covers — and
SHALL NOT draw the brush-size circle the other brush tools draw. On a layer
mask a Block dab paints the coverage with the brush tip, so the outline is the
brush-size circle there. The outline SHALL be the brush-size circle again when
the Mode leaves Block.

#### Scenario: The Block cursor matches the block [er_block_cursor]

- **WHEN** the Eraser is in Block mode and the pointer is over the canvas
- **THEN** the drawn outline is a square of the block's size at the pointer, 16 screen pixels wide at a 100 % zoom

#### Scenario: The Block cursor follows the zoom [er_block_cursor_zoom]

- **WHEN** the canvas is zoomed while the Eraser stays in Block mode
- **THEN** the drawn square is resized to the block's new size, so it stays 16 screen pixels wide
