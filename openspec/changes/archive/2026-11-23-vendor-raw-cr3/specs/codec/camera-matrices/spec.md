## ADDED Requirements

### Requirement: Canon CR3 bodies take fitted matrices and the CTMD as-shot white balance

The generated table SHALL include, under make `CANON`, every CR3 body
rawtoaces-data publishes sensitivities for at the pinned commit: the EOS R,
RP, R5, R5m2, R6, R6m2, R10 and 200D II, with rawtoaces-data's aliases (R5
Mark II, R50, R8, 250D, Rebel SL3, Kiss X10). Other CR3 bodies SHALL keep no
matrices. A CR3's as-shot neutral SHALL come from the colour data (tag
`0x4001`) of the first maker note found in its `CTMD` track's first sample
(records of types 7 to 9, block `0x927C`), read with the CR2 layouts of
"A CR2 reports its as-shot white balance". Without one it SHALL be empty.

#### Scenario: CR3 models get both matrices [cr3_matrices_lookup]

- **WHEN** synthetic CR3s name make "Canon" and the models "Canon EOS R", "Canon EOS RP", "Canon EOS R5", "Canon EOS R5m2", "Canon EOS R6", "Canon EOS R6m2", "Canon EOS R10", "Canon EOS 250D", "Canon EOS R50", "Canon EOS R8" and "Canon EOS R3"
- **THEN** each but the R3 has both matrices with illuminants 17 and 21, the R50 equals the R10, the R8 equals the R6m2, the 250D equals the 200D II, and the R3 has none

#### Scenario: Colour data in the CTMD track [cr3_as_shot]

- **WHEN** synthetic CR3s carry CTMD colour data of versions 19, 32, 48 and 64 with multipliers (2000, 1000, 1024, 1500) at the layout's index, one has no CTMD track, and one has a CTMD record whose size runs past its sample
- **THEN** the first four report `AsShotNeutral` (1012/2000, 1, 1012/1500) within 1e-9 and the other two read with none

#### Scenario: A known camera's CR3 decodes [cr3_known_camera_decodes]

- **WHEN** a synthetic "Canon EOS R5" CR3 with version-34 colour data of multipliers (2208, 1024, 1024, 1967) is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of the image's colour tags and lies between 2000 K and 12000 K

#### Scenario: Real CR3 files take colour [cr3_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.CR3` files
- **THEN** each file from a table camera decodes with `decode_raw_source` and reports an as-shot temperature between 2000 K and 12000 K, and each other file is refused as unsupported camera colour
