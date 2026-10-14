## ADDED Requirements

### Requirement: Picture Frame renders a procedural decorative border

The system SHALL provide `Filter::PictureFrame { options: FrameOptions }`. `FrameOptions` SHALL carry a frame number (1–47), a vine colour, margin, size, arrangement, a flower index (0 = none, 1–22), a flower colour and size, a leaf index (0 = none, 1–23), a leaf colour and size, and the Advanced controls Number of Lines, Thickness, Angle, Fade, and Invert. The filter SHALL draw a decorative border over the layer: motif frames lay a vine, flowers, and leaves along a rounded-rectangle track, and the ruled and moulded frames draw lines and a bevelled band from a corner distance field. Everything SHALL be procedural geometry — no external or traced artwork. The frame SHALL be composited source-over, so it can add opacity where the layer was transparent, and the result SHALL be deterministic (the arrangement is a seed, not a random draw). Every size SHALL be a fraction of the layer's shorter side, so the same parameters on a scaled copy produce a scaled copy of the frame. Any parameter outside its documented range SHALL be rejected with `FilterError::InvalidParams` before any mutation.

#### Scenario: Every frame draws a border and spares the middle

- **WHEN** `PictureFrame` runs with each frame number 1–47 at CS6's defaults on a white buffer
- **THEN** each output changes more than 200 pixels from the input, and a central region of the picture is bit-identical to the input

#### Scenario: Colours, margins, and arrangements take effect

- **WHEN** the vine and flower colours are set, the margin is raised, and the arrangement is changed
- **THEN** the output contains pixels of the vine colour and of the flower colour, a larger margin moves the first ink further in from the edge, and two arrangements give different outputs while the same arrangement is bit-identical across runs

#### Scenario: Advanced controls change the frame

- **WHEN** Thickness is raised, Number of Lines is raised, Fade is applied, and Invert is set
- **THEN** a thicker run changes more pixels than a thin one, more lines pack more crossings along the top band, Fade thins the middle of a side while sparing its corner, and Invert turns a motif frame and a moulded frame inside out

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `PictureFrame` is called with a frame number outside 1–47, a flower index above 22, a leaf index above 23, or any other control outside its range
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

### Requirement: Post-CS6 render entries are surfaced by decision

The `Filter ▸ Render` menu SHALL list the CS6 entries (Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects) and SHALL additionally surface `Picture Frame`, a CC 2014.2 entry shipped as a deliberate post-CS6 addition to Kooka. `Flame` and `Tree` were added in Photoshop CC 2014.2 and remain out of scope; they MUST NOT be surfaced in the menu and SHALL remain unsurfaced unless the project's parity scope changes.

#### Scenario: Picture Frame is present and Flame and Tree are absent

- **WHEN** the `Filter ▸ Render` menu is built
- **THEN** it contains `Picture Frame` and does not contain `Flame` or `Tree`

## REMOVED Requirements

### Requirement: Post-CS6 render entries are not surfaced

**Reason**: Superseded by *Post-CS6 render entries are surfaced by decision*: issue #227 ships Picture Frame as a deliberate post-CS6 addition, so the blanket prohibition no longer holds.

**Migration**: The menu surfaces `Picture Frame`; `Flame` and `Tree` stay unsurfaced under the replacement requirement.
