# Spec Delta

## ADDED Requirements

### Requirement: The Pictura stack-mode marker round-trips

The writer SHALL author a Pictura-private layer block, key `PiSt`, carrying a
one-byte version (`1`) and a one-byte mode code, when a layer's smart object has
an active stack mode, and SHALL emit at most one `PiSt` block per layer: a
preserved `PiSt`, recognized or not, SHALL be replaced when the model authors a
marker. The reader SHALL set `SmartObject.stack_mode` from a `PiSt` block with a
recognized version and mode code and SHALL treat that block as modelled rather
than re-emitting it as an unknown block. A `PiSt` block with an unrecognized
version or code SHALL NOT set a mode and SHALL be preserved in the layer's
unmodelled blocks, but SHALL be re-emitted only when no mode is modelled.
Clearing the mode SHALL remove the authored block on the next write. The block
SHALL NOT alter the layer's raster proxy, and the existing `SoLd`/`lnk*` bytes
SHALL be unchanged.

#### Scenario: A written mode reads back [cso_stack_round_trip]
- **WHEN** a document whose layer carries an active stack mode is written and read back
- **THEN** the layer reports the same mode and the payload and config descriptor are unchanged

#### Scenario: Clearing removes the marker [cso_stack_cleared]
- **WHEN** the mode is cleared and the document is written and read back
- **THEN** the layer reports no mode and the output carries no `PiSt` block

#### Scenario: An unknown marker is preserved [cso_stack_unknown]
- **WHEN** a layer carries a `PiSt` block with an unrecognized version or mode code
- **THEN** the layer reports no mode and the block is re-emitted unchanged

#### Scenario: A modelled mode replaces an unknown marker [cso_stack_replaced]
- **WHEN** a layer carries an unrecognized `PiSt` block and a mode is applied and written
- **THEN** the output carries exactly one `PiSt` block, the modelled marker

#### Scenario: The proxy is untouched [cso_stack_proxy]
- **WHEN** a mode is applied to a converted object and the document is written
- **THEN** the layer's colour and alpha channel bytes equal the pre-mode bytes
