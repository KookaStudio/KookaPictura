## ADDED Requirements

### Requirement: A whole-document restore shares level 0

When a whole-document restore installs level 0 from a composite that needs no
display conversion, the display frame SHALL share that composite's
copy-on-write plane instead of copying it, and both planes SHALL be fractured so
the first write forks only the bands it touches. A frame that needs a conversion
SHALL own the converted plane. The displayed pixels MUST equal a full copy of
the composite.

#### Scenario: A resize undo does not copy level 0 [hrr_level0_shared]

- **WHEN** an undo of a resize installs level 0 on an sRGB document
- **THEN** the frame shares the restored composite's plane and its pixels equal
  the composite's

#### Scenario: The first write forks only its bands [hrr_level0_forks]

- **WHEN** a region is patched into a shared level 0
- **THEN** only the bands covering the region are copied, the composite's bytes
  are unchanged, and the frame still shows the patched pixels

#### Scenario: A converted document owns its frame [hrr_level0_converted]

- **WHEN** a document with a non-sRGB profile installs level 0
- **THEN** the frame holds the converted sRGB pixels and does not share the
  composite's plane

### Requirement: A soloed mask's display source builds on every core

A restore that rebuilds the solo mask's display source SHALL fill its
document-sized grayscale plane as parallel row work rather than one serial pass.
The plane and its pyramid MUST equal the same samples filled serially.

#### Scenario: The parallel mask plane equals the serial samples [hrr_mask_solo_exact]

- **WHEN** a solo mask's display plane is built for a document with an off-rect
  mask and a default colour
- **THEN** every pixel equals the serial reference, opaque and grey

#### Scenario: A soloed mask restore stops blocking the GUI thread [hrr_mask_solo_manual]

- **WHEN** a resize of a 16507×16196 document with a soloed mask is undone
- **THEN** the mask plane and its pyramid are rebuilt in about the parallel
  time, measured by the ignored `structural_restore_profile`
