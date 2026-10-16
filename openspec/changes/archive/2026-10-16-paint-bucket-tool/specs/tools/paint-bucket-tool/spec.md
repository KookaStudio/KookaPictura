## ADDED Requirements

### Requirement: Paint Bucket tool

A Paint Bucket click SHALL fill the pixels matching the clicked one within
Tolerance on every channel, alpha included — connected to it when Contiguous,
sampled from the active layer or with All Layers from the composite — with the
foreground colour or the chosen pattern tiled from the document origin, on the
active layer, inside the selection when there is one, at Opacity in the chosen
Mode, with a softened edge when Anti-alias is on, recording exactly one "Paint
Bucket" history state when pixels changed. A transparency-locked layer SHALL
keep transparent pixels empty, and a layer whose pixels are locked SHALL be
refused.

#### Scenario: Filling the connected region

- **WHEN** the `tst_fill_tools` Paint Bucket test clicks the white around two red squares with a blue foreground
- **THEN** one "Paint Bucket" state is recorded, the white is blue, and the squares stay red

#### Scenario: All Layers fills the active layer from the composite

- **WHEN** All Layers is on and a red square is clicked with an empty layer active
- **THEN** the square shows the foreground colour and the other square stays red

#### Scenario: Pattern fill

- **WHEN** Fill is Pattern with the checkerboard and Contiguous is off
- **THEN** the filled pixels show the tile's light and dark cells

#### Scenario: Locked pixels refuse the fill

- **WHEN** the active layer's pixels are locked and the canvas is clicked
- **THEN** no state is recorded
