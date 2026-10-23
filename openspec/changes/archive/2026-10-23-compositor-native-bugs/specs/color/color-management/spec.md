# Spec Delta

## MODIFIED Requirements

### Requirement: Bit depths and channel counts

`convert` SHALL support 8-bit, 16-bit, and 32-bit floating-point samples and
1-, 3-, and 4-channel interleaved input (gray, RGB, and RGBA respectively),
mapping gray/RGB/RGBA input into the destination color space. Any other
channel-count/bit-depth combination MUST return a `ColorError::Unsupported`.

#### Scenario: 16-bit input is preserved at 16-bit

- **WHEN** a 16-bit RGB buffer is converted
- **THEN** the output is the same number of 16-bit samples as the input

#### Scenario: 32-bit float input is preserved at 32-bit [cm_convert_32bit]

- **WHEN** a 32-bit float RGB buffer is converted
- **THEN** the output is the same number of 32-bit float samples as the input and the samples are finite

#### Scenario: Unsupported format is rejected

- **WHEN** `convert` is called with e.g. 2 channels or 12 bits
- **THEN** it returns a `ColorError::Unsupported`
