## ADDED Requirements

### Requirement: Pictura Raw renders 16-bit RGB

`render_pictura_raw_rgb16` SHALL apply the Pictura Raw controls to interleaved
16-bit RGB with the same stages as the 8-bit render, reading and writing
16-bit samples so no stage quantizes through 8 bits. Default settings SHALL
return the samples unchanged. A buffer whose length is not width × height × 3,
or an empty image, SHALL be refused.

#### Scenario: Default settings are a no-op at 16 bits [pictura_raw_rgb16_default_noop]

- **WHEN** a random 16-bit image renders with default settings
- **THEN** every sample is unchanged

#### Scenario: 16-bit matches 8-bit on 8-bit values [pictura_raw_rgb16_matches_8bit]

- **WHEN** an image whose 16-bit samples are 8-bit values × 257 renders with exposure +0.5, contrast 20 and saturation −30 at both depths
- **THEN** each 16-bit result divided by 257 is within 1 of the 8-bit result

#### Scenario: A malformed 16-bit buffer is refused [pictura_raw_rgb16_refuses]

- **WHEN** a buffer one sample short, and a 0 × 0 image, render
- **THEN** each returns an error
