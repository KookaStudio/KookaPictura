# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
