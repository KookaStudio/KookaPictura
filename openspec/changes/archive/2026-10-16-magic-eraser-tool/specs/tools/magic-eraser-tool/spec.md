## ADDED Requirements

### Requirement: Magic Eraser tool

A Magic Eraser click SHALL erase to transparency the pixels the Magic Wand
would select there — within Tolerance of the clicked colour, connected to it
when Contiguous, sampled from the active layer or with Sample All Layers from
the composite — at Opacity, with a softened edge when Anti-alias is on,
recording exactly one "Magic Eraser" history state when pixels changed. It
SHALL turn the Background into a layer first, SHALL fill a transparency-locked
layer with the background colour instead, and SHALL refuse a layer whose pixels
are locked.

#### Scenario: Erasing one of two regions

- **WHEN** the `magic_eraser_tool` self-test clicks one of two separate red squares on an opened white image
- **THEN** one "Magic Eraser" state is recorded, that square is transparent, and the other square and the white are kept

#### Scenario: Erasing every similar colour

- **WHEN** Contiguous is off and the white is clicked
- **THEN** all the white is transparent and the red square is kept

#### Scenario: Opacity scales the erase

- **WHEN** a masked pixel is erased at 50 % Opacity
- **THEN** half its alpha remains and its colour is unchanged
