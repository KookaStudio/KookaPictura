## MODIFIED Requirements

### Requirement: A SuperCCD grid is rotated upright after the demosaic

For an image with a `fuji_width` `fw`, `demosaic` SHALL demosaic the grid
with the chosen method, then rotate it as LibRaw 0.22.1's `fuji_rotate`
does: with `f = fw − 1` and `s = √½`, the output SHALL be `⌊f / s⌋` wide and
`⌊(H − f) / s⌋` high for a grid `H` high, and output pixel `(row, col)`
SHALL be the grid sampled bilinearly at row `f + (row − col)·s` and column
`(row + col)·s`, truncated to an integer, or black when that position is
negative or its integer part lies past the grid's second-last row or
column. The rotated output SHALL be charged against the image budget with
the demosaiced grid. Parity with LibRaw's rendering is by geometry; the
demosaic itself is ours.

#### Scenario: The rotated size [superccd_rotate_size]

- **WHEN** SuperCCD images of each layout bit with odd and even `fw` are demosaiced, and one whose rotated output exceeds the budget
- **THEN** each output has the formula's size, and the oversized one is refused

#### Scenario: Rotation samples LibRaw's positions [superccd_rotate_positions]

- **WHEN** a SuperCCD grid mosaiced from an RGB image linear in row and column is demosaiced with each method
- **THEN** every output pixel whose sample position lies at least six cells inside the grid equals that image at LibRaw's position within 2

#### Scenario: Real SuperCCD renders match LibRaw [superccd_rotate_libraw]

- **WHEN** `PICTURA_RAW_CORPUS` holds SuperCCD RAFs and python3 with rawpy is available, the SR files read with header offset `0x80` cleared so only their primary frame is read, as LibRaw reads them
- **THEN** each demosaiced and oriented image has the size of LibRaw's linear, unscaled camera-RGB rendering, and each channel's 8 × 8 block means correlate with LibRaw's at 0.99 or more
