# color/colour-temperature Specification

## Purpose
Shared chromaticity math: conversion between CIE xy and correlated colour temperature with tint, RGB-to-XYZ matrices from primaries, Bradford adaptation, and a linear ProPhoto ICC profile, used by the camera colour stage (#398).

## Requirements

### Requirement: Chromaticities convert to and from colour temperature and tint

`pictura_color::chroma` SHALL convert a CIE 1931 xy chromaticity to a
correlated colour temperature in kelvin and a tint, and back. The temperature
SHALL come from the closest point on the Planckian locus in CIE 1960 uv
(Krystek's approximation), clamped to 1000–15000 K. The tint SHALL be −3000
times the signed distance from the locus, positive toward magenta. The tint
scale approximates Camera Raw's and is not measured against it.

#### Scenario: Standard illuminants [cct_illuminants]

- **WHEN** the chromaticities of CIE illuminant A (0.44757, 0.40745) and D65 (0.31271, 0.32902) are converted
- **THEN** A gives 2856 K within 10 K and a tint within ±1, and D65 gives 6504 K within 15 K and a tint between −11 and −8

#### Scenario: Round trip [cct_round_trip]

- **WHEN** every temperature from 2000 K to 15000 K in 500 K steps, with tints −50, 0 and +50, is converted to xy and back
- **THEN** the temperature returns within 0.1 % and the tint within 0.05

### Requirement: Shared matrix and adaptation math

`pictura_color::chroma` SHALL provide the RGB-to-XYZ matrix of a set of
primaries and a white point, and the Bradford chromatic adaptation matrix
between two white points.

#### Scenario: ProPhoto and Bradford [chroma_matrices]

- **WHEN** the matrix of the sRGB primaries is built for D65 and adapted by Bradford from D65 to D50
- **THEN** it equals the published Bradford-adapted sRGB-to-XYZ-D50 matrix within 1e-4, and the ProPhoto matrix maps (1, 1, 1) to the D50 white

### Requirement: A linear ProPhoto ICC profile

`Profile::linear_pro_photo()` SHALL return an RGB profile with the ROMM
primaries, a D50 white and a linear tone curve, so the camera colour stage's
output converts through the existing profile conversion.

#### Scenario: Linear to encoded ProPhoto [linear_pro_photo_profile]

- **WHEN** 16-bit linear ProPhoto grey 0.25 and white are converted to `Profile::pro_photo()` with relative colorimetric intent
- **THEN** grey becomes 0.25^(1/1.8) of full scale within 2 counts per 65535 on every channel, and white stays white
