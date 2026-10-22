# Spec Delta

## ADDED Requirements

### Requirement: Convert to Profile transforms retained native samples

When a 16- or 32-bit RGB document is converted to another profile, the retained
native color samples SHALL be transformed at the stored depth together with the
8-bit working copies. The document's `source_planes` color planes and every
layer's `source_channels` color planes (channel ids `0..3`) SHALL be converted;
alpha (id `-1`), mask (id `-2`), and document extra planes MUST be copied
unchanged. The
destination profile SHALL then be assigned. A save after conversion MUST re-emit
the converted samples: a 16-bit ramp MUST retain more than 256 distinct levels in
the destination profile, and 32-bit float samples outside `[0, 1]` MUST NOT be
clamped by the conversion.

#### Scenario: A 16-bit conversion keeps native levels [picc_convert_16bit]

- **WHEN** a 16-bit RGB PSD with a 0..65535 ramp is converted from sRGB to Adobe RGB, saved, and reopened
- **THEN** the saved file MUST carry more than 256 distinct levels in the destination profile and the working copy MUST match the converted native samples

#### Scenario: A 32-bit conversion does not clamp [picc_convert_32bit]

- **WHEN** a 32-bit float RGB document with color samples outside `[0, 1]` is converted to another profile
- **THEN** the converted native samples MUST stay 32-bit floats and MUST NOT be pre-clamped to `[0, 1]`, while alpha and mask planes MUST remain byte-for-byte equal
