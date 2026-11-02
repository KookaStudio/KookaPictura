# Spec Delta

## ADDED Requirements

### Requirement: De-Interlace field reconstruction

The engine SHALL provide a De-Interlace filter that reconstructs one interlaced field from the other. It SHALL take a field choice and a method. Rows SHALL be numbered from zero; the field choice SHALL be `Odd` or `Even`, where `Odd` eliminates the rows whose index is odd and `Even` eliminates the rows whose index is even, so the buffer's first row belongs to the even field. The method SHALL be `Interpolation` or `Duplication`. An eliminated row MUST be reconstructed from the retained rows adjacent to it: `Interpolation` SHALL set the row to the rounded mean of the retained row above and the retained row below, and `Duplication` SHALL copy the retained row above. A retained row MUST be bit-identical to its input. At the top edge, where no row above exists, both methods SHALL use the retained row below; at the bottom edge, where no row below exists, both SHALL use the retained row above.

The filter MUST operate in place on the engine's planar 8-bit buffer with 3 or 4 channels, MUST reconstruct every channel plane including a fourth (alpha) plane, MUST be deterministic, and MUST NOT change the buffer's dimensions. A buffer whose channel count is not 3 or 4, or whose data length does not match its dimensions, MUST return an error and remain unmodified. A buffer with fewer than two rows MUST be returned unchanged. Adobe's exact kernel is closed, so the row-parity, edge and duplication conventions above are the project's recorded approximation. Sixteen- and 32-bit-per-channel documents are outside the filter engine's current depth support and are not claimed by this requirement.

#### Scenario: The odd field is reconstructed [deinterlace_odd]

- **WHEN** the filter runs with the `Odd` choice on a buffer whose rows carry a known pattern
- **THEN** every odd-indexed row is reconstructed and every even-indexed row is bit-identical to its input

#### Scenario: The even field is reconstructed, including the first row [deinterlace_even]

- **WHEN** the filter runs with the `Even` choice on a buffer whose rows carry a known pattern
- **THEN** every even-indexed row is reconstructed, the first row takes its single retained neighbour below, and every odd-indexed row is bit-identical to its input

#### Scenario: Interpolation averages the two retained neighbours [deinterlace_interpolation]

- **WHEN** the filter runs with the `Interpolation` method
- **THEN** an eliminated row is the rounded mean of the retained rows above and below it, including a mean that rounds up, and an edge row with a single retained neighbour takes that neighbour's value

#### Scenario: Duplication copies the retained row above [deinterlace_duplication]

- **WHEN** the filter runs with the `Duplication` method
- **THEN** an eliminated row equals the retained row above it, and the first row of an `Even` run equals the retained row below it

#### Scenario: Alpha is reconstructed with the colour planes [deinterlace_alpha]

- **WHEN** the filter runs on a 4-channel buffer with a non-uniform alpha plane
- **THEN** the alpha plane is reconstructed by the same rule as the colour planes and the retained rows keep their alpha bit-identical

#### Scenario: Malformed input is refused [deinterlace_invalid]

- **WHEN** the filter receives a buffer with a channel count other than 3 or 4, an empty buffer, or a data length that does not match its dimensions
- **THEN** it returns a filter error, does not panic, and does not modify the buffer

#### Scenario: A buffer with fewer than two rows is unchanged [deinterlace_short]

- **WHEN** the filter runs on a buffer with one row
- **THEN** every plane is bit-identical to the input

### Requirement: De-Interlace command

`Filter ▸ Video ▸ De-Interlace` SHALL open a dialog. The filter parameter descriptor SHALL record the kind `de-interlace`, the path `Filter/Video/De-Interlace`, the label `De-Interlace`, two radio controls — `Eliminate:` with the choices `Odd Fields` and `Even Fields`, and `Create New Fields by:` with the choices `Interpolation` and `Duplication` — each defaulting to its first choice, and the shared preview presentation flags with the whole-layer preview. The kind SHALL resolve to the engine's De-Interlace filter with the field and method the two slots select and MUST refuse a slot list of any other length. The menu entry MUST be enabled exactly when the active layer is an unlocked normal pixel layer, MUST end with an ellipsis, and choosing it MUST open the dialog and commit exactly one history state labelled `Filter: De-Interlace`.

#### Scenario: The entry opens its dialog and commits one state [deinterlace_command]

- **WHEN** the user chooses `Filter ▸ Video ▸ De-Interlace` on a document with an unlocked normal pixel layer and accepts the dialog
- **THEN** the dialog shows the two radio groups with `Odd Fields` and `Interpolation` selected, the layer's rows change, and the history gains one state labelled `Filter: De-Interlace`

#### Scenario: The kind maps its two slots and refuses any other length [deinterlace_mapping]

- **WHEN** the command maps `de-interlace` with two values, with no values, and with one value
- **THEN** the two-value list selects the field and method it names, the empty list selects both defaults, and the one-value list is refused without modifying the document

#### Scenario: No filter target disables the entry [deinterlace_disabled]

- **WHEN** no editable pixel layer is active
- **THEN** the De-Interlace menu entry is disabled
