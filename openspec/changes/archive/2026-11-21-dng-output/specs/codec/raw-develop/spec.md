## ADDED Requirements

### Requirement: A linear DNG decodes without a demosaic

`decode_raw_source` SHALL read a DNG whose full-resolution image is
`LinearRaw` with `read_linear_dng`, refuse unmappable colour as for a CFA
image, and keep the file's camera RGB as the source without demosaicing.

#### Scenario: A linear DNG decodes to a raw source [raw_source_linear]

- **WHEN** a synthetic 16 × 12 linear DNG with `ColorMatrix1`, `AsShotNeutral` and a default crop is decoded
- **THEN** the source's camera RGB equals the file's samples, and its crop and as-shot white balance equal the file's
