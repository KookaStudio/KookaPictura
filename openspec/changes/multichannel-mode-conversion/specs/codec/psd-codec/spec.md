## MODIFIED Requirements

### Requirement: Header validation for the composite subset
The system SHALL parse the channel count, height, width, bit depth, and color mode, accepting 1–56 channels and non-zero dimensions within the PSD/PSB limit. Bitmap (0), Grayscale (1), Indexed (2), RGB (3), CMYK (4), Multichannel (7), Duotone (8), and Lab (9) SHALL be accepted at the depths and channel layouts specified by `codec/psd-color-modes` and `psd-bit-depth`. Unknown mode codes, unsupported depths, and unsupported Multichannel layouts SHALL return `PsdError::Unsupported`. A channel count or dimension outside the accepted range SHALL return `PsdError::Invalid`.

#### Scenario: Zero dimension
- **WHEN** the header declares a width or height of zero
- **THEN** it returns `PsdError::Invalid`

#### Scenario: Depth 16 and 32 are accepted
- **WHEN** the header declares bit depth 16 or 32 for Grayscale, RGB, CMYK, or Lab
- **THEN** the header is accepted and the samples are read and narrowed to 8-bit

#### Scenario: Unsupported depth
- **WHEN** the header declares a bit depth other than 1, 8, 16, or 32, or bit depth 16/32 for a Bitmap or Indexed document
- **THEN** it returns `PsdError::Unsupported`

#### Scenario: Bitmap depth 1 is accepted
- **WHEN** the header declares Bitmap mode with bit depth 1
- **THEN** the header is accepted and the composite is decoded as 1-bit rows

#### Scenario: Channel count out of range
- **WHEN** the header declares zero channels or more than 56 channels
- **THEN** it returns `PsdError::Invalid`

#### Scenario: A non-RGB color mode is accepted
- **WHEN** the header declares Indexed (2), CMYK (4), or Lab (9)
- **THEN** the header is accepted and the document is normalized to RGB

#### Scenario: Multichannel and Duotone follow their mode contracts [multi_header_modes]
- **WHEN** a Multichannel or Duotone header declares a supported depth and channel layout
- **THEN** it is accepted under `codec/psd-color-modes`, and an unsupported depth or layout is refused
