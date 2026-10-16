# imaging/liquify-filter Specification

## Purpose
The Liquify mesh and its resampling: warp, twirl, pucker, bloat, and
reconstruct.

## Requirements

### Requirement: Liquify mesh and resampling

The system SHALL implement Liquify as a `LiquifyMesh` holding an inverse displacement per node of a regular grid over the layer, plus a freeze mask, and a `Filter::Liquify { mesh }` variant applied through `pictura_filters::apply`. Resampling SHALL show, at each output pixel `p`, the source at `p + d(p)`, with edge pixels repeating past the layer. An identity mesh, and a mesh whose distortion was fully removed, SHALL leave the layer bit-identical. A mesh built for another layer size SHALL be stretched to fit, scaling its offsets with it.

#### Scenario: An identity mesh is a no-op

- **WHEN** `apply` receives `Filter::Liquify` with an identity mesh
- **THEN** every sample of the buffer is unchanged

#### Scenario: A uniform offset translates exactly

- **WHEN** every node of a mesh holds the offset `(3, 0)`
- **THEN** each output pixel equals the source pixel three columns to its right

### Requirement: Liquify distortion tools

Each brush tool SHALL distort only nodes inside the brush, most strongly at its centre. Forward Warp SHALL move pixels along the drag. Twirl SHALL rotate pixels clockwise about the brush centre, further with each Brush Rate tick, and counterclockwise with Alt. Pucker SHALL move pixels toward the centre and Bloat away from it, each reversed by Alt. Push Left SHALL move pixels left on an upward drag and right on a downward one, reversed by Alt. Mirror SHALL reflect the far side of the stroke onto the side left of it (right with Alt) and leave the far side untouched. Turbulence SHALL scramble pixels smoothly and deterministically. Reconstruct SHALL ease an existing distortion back toward none.

#### Scenario: Forward Warp follows the drag and stays inside the brush

- **WHEN** a 30 px Forward Warp brush drags 12 px to the right
- **THEN** pixels at the end of the drag read from the left, less so toward the brush edge, and nodes beyond the brush keep a zero offset

#### Scenario: Pucker and Bloat move pixels radially

- **WHEN** Pucker is held still over a point
- **THEN** pixels inside the brush move toward that point, and Bloat (or Pucker with Alt) moves them away

### Requirement: Liquify freeze mask and reconstruction

Frozen nodes SHALL take no distortion from any tool. Freeze Mask and Thaw Mask SHALL paint the mask with the brush; Mask All SHALL freeze every node, Mask None thaw every node, and Invert All swap frozen and thawed. Reconstruct with an amount SHALL scale unfrozen offsets toward zero by that percentage, so 0 is a no-op and 100 removes the unfrozen distortion; Restore All SHALL remove every distortion, frozen areas included.

#### Scenario: A frozen layer is protected and Invert All releases it

- **WHEN** Mask All is applied and a Forward Warp stroke is drawn
- **THEN** the mesh stays identity, and after Invert All the same stroke distorts it

#### Scenario: Restore All returns the original

- **WHEN** a distorted, fully frozen mesh is reconstructed at 100 and then restored
- **THEN** Reconstruct leaves the frozen offsets and Restore All leaves an identity mesh whose resample is bit-identical to the source
