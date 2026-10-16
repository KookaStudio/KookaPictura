# Spec Delta

## Purpose

Retires the stale "every Other filter keeps channel 4" rule that contradicted
the Offset requirement: Maximum, Minimum, and Offset move the layer edge;
High Pass and Custom keep it (#406).

## ADDED Requirements

### Requirement: Other alpha follows the kernel

For 4-channel buffers, Maximum and Minimum SHALL run their kernel over channel 4 as they do over the colour planes, and Offset SHALL translate channel 4 per the Offset requirement. High Pass and Custom SHALL leave channel 4 bit-identical; only channels 1 through 3 SHALL be modified.

#### Scenario: Maximum, Minimum, and Offset move the layer edge

- **WHEN** Maximum, Minimum, or a non-zero Offset is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: High Pass and Custom preserve alpha

- **WHEN** High Pass or Custom is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

## REMOVED Requirements

### Requirement: Alpha channel preservation for Other filters

**Reason**: It held channel 4 bit-identical for the whole family, which
contradicts the `imaging/filter-application` alpha contract and the fold
(#248): a kernel on `filters_alpha` runs over channel 4 so the layer edge
moves with the colour.

**Migration**: Replaced by "Other alpha follows the kernel", which names the variants that move
channel 4 and those that keep it bit-identical.
