## ADDED Requirements

### Requirement: An embedded PNG profile inflates within a limit

Probing a PNG for its embedded `iCCP` profile SHALL inflate at most 16 MiB.
A profile that inflates past the limit, or a chunk too short to hold its
keyword, separator, and method byte, SHALL be reported as present with no
profile bytes, and SHALL NOT panic.

#### Scenario: A profile bomb [ui_png_profile_bomb]

- **WHEN** a PNG's `iCCP` chunk holds a few KiB of zlib data that inflates
  to 64 MiB
- **THEN** the probe reports a present profile with no bytes, and inflates
  no more than 16 MiB plus one byte

#### Scenario: A truncated profile chunk [ui_png_profile_truncated]

- **WHEN** a PNG's `iCCP` chunk ends with the keyword's terminating NUL
- **THEN** the probe reports a present profile with no bytes and does not
  panic, and a well-formed `iCCP` chunk still probes its profile

### Requirement: Ghostscript raster renders band past a bitmap ceiling

Every Ghostscript raster render SHALL pass `-dMaxBitmap=536870912`, so a page
bitmap larger than 512 MiB is rendered in bands instead of as one full-page
allocation.

#### Scenario: The render arguments carry the ceiling [ui_gs_max_bitmap]

- **WHEN** the PNG render arguments are built for any resolution
- **THEN** they include `-dMaxBitmap=536870912` before the input file
