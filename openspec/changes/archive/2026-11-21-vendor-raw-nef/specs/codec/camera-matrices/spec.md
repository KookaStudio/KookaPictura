## ADDED Requirements

### Requirement: Nikon NEF bodies take fitted matrices and the as-shot white balance

The generated table SHALL seed the Nikon bodies whose sensitivities
rawtoaces-data publishes (D70, D200, D700, D800E, D810, D850, D3300, D5100,
D5300, D7000 and Z f) with their same-sensor aliases (D800, Z 6_2), under
make key `NIKON`. The NEF reader SHALL set `AsShotNeutral` from the Nikon
maker note's `WB_RBLevels` (0x000C) red and blue multipliers `R` and `B` as
`(1/R, 1, 1/B)`; a missing tag or a multiplier that is not positive SHALL
leave it empty.

#### Scenario: Known Nikon models get both matrices [nef_camera_matrices]

- **WHEN** synthetic NEFs name make "NIKON CORPORATION" and the models "NIKON D850", "NIKON D800", "NIKON D800E", "NIKON Z f" and "NIKON Z 6_2", and one names "NIKON D4"
- **THEN** each but the D4 has both colour matrices with illuminants 17 and 21, the D800 and D800E matrices are equal, the Z f and Z 6_2 matrices are equal, and the D4 has none

#### Scenario: As-shot white balance [nef_as_shot]

- **WHEN** a synthetic "NIKON D850" NEF carries `WB_RBLevels` (1.9, 1.375, 1, 1), another carries a zero red multiplier, and another none
- **THEN** the first reports `AsShotNeutral` (1/1.9, 1, 1/1.375) within 1e-9 and decodes with `decode_raw_source` at a temperature between 2000 K and 12000 K, and the other two have no `AsShotNeutral`

#### Scenario: Fitted Nikon matrices keep white neutral [nef_matrices_white]

- **WHEN** every seeded Nikon body's as-shot transform is built for an `AsShotNeutral` equal to its D65 matrix's response to the D65 white, and then its A matrix's response to the A white
- **THEN** the reported white balance is 6504 K and then 2856 K, each within 2 %

#### Scenario: Real Nikon files take colour [nef_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.NEF` files
- **THEN** each read file from a table camera decodes with `decode_raw_source` at an as-shot temperature between 2000 K and 12000 K, and each other read file is refused as unsupported camera colour
