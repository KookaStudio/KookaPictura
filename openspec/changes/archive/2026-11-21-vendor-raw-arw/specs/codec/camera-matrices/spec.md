## ADDED Requirements

### Requirement: The matrix table seeds Sony bodies

The generated table SHALL include every Sony body rawtoaces-data publishes
sensitivities for at the pinned commit (ILCE-7M3, ILCE-7M4, ILCE-7RM2,
ILCE-7RM3, ILCE-7RM4, ILCE-7SM2, ILCE-7SM3, ILCE-7CM2, ILCE-9, ILCE-6400 and
DSC-RX100M4) under make `SONY`, with rawtoaces-data's same-sensor aliases
(ILCE-7RM3A with the ILCE-7RM3, ILCE-7RM4A and ILCE-7RM5 with the
ILCE-7RM4). Other Sony bodies SHALL keep no matrices. An ARW's as-shot
neutral SHALL come from the multipliers of `codec/vendor-raw-containers`.

#### Scenario: Sony models get both matrices [sony_matrices_lookup]

- **WHEN** synthetic ARWs name make `SONY` and each of the 11 seeded models, the three aliases, and `ILCE-1`
- **THEN** each seeded model has both matrices with illuminants 17 and 21, each alias equals its body, the ILCE-7M3 differs from the ILCE-7RM3, and the ILCE-1 has none

#### Scenario: A known camera's ARW decodes [arw_known_camera_decodes]

- **WHEN** a synthetic `ILCE-7M3` ARW with multipliers (2272, 1024, 1024, 1800) is decoded with `decode_raw_source`
- **THEN** its neutral is (1024/2272, 1, 1024/1800), it decodes, and its as-shot white balance equals `as_shot_transform` of its colour tags and lies between 2000 K and 12000 K

#### Scenario: Real ARW files take colour [arw_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.ARW` files
- **THEN** each readable file from a table camera decodes with `decode_raw_source` and reports an as-shot temperature between 2000 K and 12000 K, and each other readable file is refused as unsupported camera colour
