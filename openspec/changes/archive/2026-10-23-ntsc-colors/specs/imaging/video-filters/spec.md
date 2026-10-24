# Spec Delta

## ADDED Requirements

### Requirement: NTSC Colors gamut clamp

The engine SHALL provide an NTSC Colors filter that clamps a colour into the NTSC video-safe gamut. It SHALL convert each pixel's sRGB colour, normalised to `0..=1`, to YIQ, reduce the chroma vector `(I, Q)` to a magnitude of at most `0.5` when its magnitude exceeds `0.5` while preserving the vector's direction, convert the result back to sRGB, and clamp each output channel to `0..=1`. Luma and hue MUST be unchanged; only saturation SHALL be reduced. A colour whose chroma magnitude is already at most `0.5` MUST be returned unchanged. Adobe's exact kernel is closed, so the YIQ formulation is the project's recorded approximation.

The filter MUST operate in place on the engine's planar 8-bit buffer with 3 or 4 channels, MUST leave a fourth (alpha) plane bit-identical, and MUST be deterministic. A buffer whose channel count is not 3 or 4, or whose data length does not match its dimensions, MUST return an error and remain unmodified. Sixteen- and 32-bit-per-channel documents are outside the filter engine's current depth support and are not claimed by this requirement.

#### Scenario: A saturated primary is pulled into the gamut [ntsc_clamp]

- **WHEN** the filter runs on an opaque buffer whose pixel is fully saturated red, green, magenta or cyan
- **THEN** that pixel's chroma magnitude is at most `0.5`, it is lower than the input's, its luma is unchanged, and its hue direction is preserved

#### Scenario: An in-gamut colour is unchanged [ntsc_in_gamut]

- **WHEN** the filter runs on a neutral grey or a muted colour whose chroma magnitude is at most `0.5`
- **THEN** that pixel's colour planes are bit-identical to the input

#### Scenario: Alpha is preserved [ntsc_alpha]

- **WHEN** the filter runs on a 4-channel buffer
- **THEN** the alpha plane is bit-identical to the input

#### Scenario: Malformed input is refused [ntsc_invalid]

- **WHEN** the filter receives a buffer with a channel count other than 3 or 4, an empty buffer, or a data length that does not match its dimensions
- **THEN** it returns a filter error, does not panic, and does not modify the buffer

### Requirement: NTSC Colors command

`Filter ▸ Video ▸ NTSC Colors` SHALL be a parameterless command. The filter parameter descriptor SHALL record the kind `ntsc-colors`, the path `Filter/Video/NTSC Colors`, the label `NTSC Colors`, an empty control list, and the shared preview presentation flags. The kind SHALL resolve to the engine's NTSC Colors filter when mapped with an empty slot list and MUST refuse a non-empty slot list. The menu entry MUST be enabled exactly when the active layer is an unlocked normal pixel layer, MUST carry no trailing ellipsis, and choosing it MUST apply the filter immediately with no dialog and commit exactly one history state labelled `Filter: NTSC Colors`.

#### Scenario: The entry applies with no dialog [ntsc_command]

- **WHEN** the user chooses `Filter ▸ Video ▸ NTSC Colors` on a document with an unlocked normal pixel layer
- **THEN** no dialog opens, the layer's colour planes change, and the history gains one state labelled `Filter: NTSC Colors`

#### Scenario: The kind maps with no slots and refuses a list [ntsc_mapping]

- **WHEN** the command maps `ntsc-colors` with an empty parameter list and then with one value
- **THEN** the empty list produces the NTSC Colors filter and the one-value list is refused without modifying the document

#### Scenario: No filter target disables the entry [ntsc_disabled]

- **WHEN** no editable pixel layer is active
- **THEN** the NTSC Colors menu entry is disabled
