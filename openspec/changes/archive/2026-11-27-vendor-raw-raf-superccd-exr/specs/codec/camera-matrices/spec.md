## MODIFIED Requirements

### Requirement: Fujifilm bodies take camera-JPEG matrices

Fujifilm RAF bodies SHALL take the matrices
`scripts/fit-camera-jpeg-matrices.py` fits, by the method and acceptance
limits of "Panasonic bodies take spectral or camera-JPEG matrices", from one
raw.pixls.us sample RAF each. The camera rendering SHALL be the JPEG section
the RAF header names, make and model SHALL come from its Exif IFD0, and the
presets from the raw directory's `0xF00C` record of (green, red, blue, EXIF
light source) quadruples: the daylight matrix under D65 (code 21) and the
tungsten matrix under Standard A (code 17). A RAF whose sensor section is
not a TIFF SHALL take its presets from the directory's records instead:
`0x2100` (green, red, green, blue) as Fine Weather (code 9) for the daylight
matrix and `0x2400` as Tungsten (code 3) for the tungsten matrix. The
tungsten matrix SHALL be dropped when the pair does not read back its lights
within 2 %. The camera RGB SHALL be the demosaiced, upright image for a
SuperCCD file, and the mosaic otherwise. Rows SHALL be keyed by make
`FUJIFILM`. These matrices reproduce Fujifilm's rendering and are
approximate.

#### Scenario: Fujifilm bodies get their matrices [raf_jpeg_matrices]

- **WHEN** synthetic RAFs name make `FUJIFILM` and each fitted body, and the model `X-T99`
- **THEN** each fitted body has a matrix under its daylight light and, when the table pairs it, one under its tungsten light; the X-T3 differs from the X-T2; and the X-T99 has none

#### Scenario: Fujifilm matrices keep white neutral [raf_jpeg_matrices_white]

- **WHEN** each Fujifilm body's as-shot transform is built for an `AsShotNeutral` equal to each of its matrices' response to the white at that matrix's light
- **THEN** each neutral is positive and the reported temperature is that light's within 2 %

#### Scenario: A fitted body's RAF decodes [raf_known_camera_decodes]

- **WHEN** a synthetic `X-T3` RAF with green, red and blue levels 302, 583 and 584 is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of its colour tags and lies between 2000 K and 12000 K

#### Scenario: A fitted SuperCCD body's RAF decodes upright [raf_superccd_decodes]

- **WHEN** a synthetic SuperCCD word-section RAF named after a fitted SuperCCD body, with `0x2FF0` levels green 80, red 167, green 80, blue 119, is decoded with `decode_raw_source`
- **THEN** it decodes to the rotated size at an as-shot temperature between 2000 K and 12000 K

#### Scenario: Real Fujifilm files take colour [raf_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.RAF` files
- **THEN** each readable file from a table body decodes with `decode_raw_source` at an as-shot temperature between 2000 K and 12000 K

#### Scenario: The Fujifilm fits are reproducible [raf_jpeg_fits_generated]

- **WHEN** `scripts/fit-camera-jpeg-matrices.py` is run on the RAF samples `camera-jpeg-fits.json` names, then `scripts/gen-camera-matrices.py`
- **THEN** both outputs are rewritten byte for byte

#### Scenario: Fujifilm renders follow the camera's JPEG [raf_render_matches_camera]

- **WHEN** each fitted body's sample is opened through File ▸ Open at its as-shot white balance and compared beside its embedded camera JPEG
- **THEN** its colours follow the camera's rendering, apart from tone and lens corrections the open does not apply
