# tools/bristle-brushes Specification

## Purpose
Bristle tips: a deformable bundle of individual bristle tracks drawn as one
dab, plus the per-bristle Mixer Brush reservoir.

## ADDED Requirements

### Requirement: Bristle tip model

A brush tip SHALL be either the existing smooth profile or a bristle tip. A
bristle tip SHALL carry a Shape, Bristles (0–100 %), Length (0–100 %),
Thickness (0–100 %), and Stiffness (0–100 %). A bristle tip with Bristles 0 SHALL
still paint at least one bristle. The default tip SHALL remain the smooth
profile, and a smooth tip SHALL paint exactly the pixels it painted before this
change.

#### Scenario: A smooth tip is unchanged

- **WHEN** a stroke with no bristle tip is painted after this change
- **THEN** its pixels are identical to the same stroke before the change

#### Scenario: Zero bristles still paint

- **WHEN** a bristle tip with Bristles 0 paints a dab
- **THEN** the dab covers at least one pixel

### Requirement: Bristle shapes and tracks

The six Shapes SHALL be Round, Round Fan, Round Curve, Flat, Flat Fan, and Flat
Curve. A Round shape SHALL place its bristle roots on an arc and a Flat shape
SHALL place them on a line, so their union has a round versus a chiselled
outer edge. Fan variants SHALL spread each track outward from the root, and
Curve variants SHALL bend the root line. Tracks SHALL be tapered capsules, and
Bristles, Length, and Thickness SHALL set the count, length, and width of those
tracks.

#### Scenario: Flat and round marks differ

- **WHEN** a Flat tip and a Round tip paint one dab with the same size and
  bristle count
- **THEN** the Flat dab's cross-section has a wider, flatter extent than the
  Round dab's

#### Scenario: Bristles changes the streak count

- **WHEN** the Bristles value rises from low to high
- **THEN** the number of bristle tracks in a dab rises

#### Scenario: Length and thickness change the mark

- **WHEN** Length rises or Thickness rises
- **THEN** the mark reaches farther along the stroke or its streaks become
  wider

### Requirement: Bristle rasterization

A bristle dab SHALL composite the union of its tapered tracks through the
existing coverage and blend path, so a stroke shows individually resolvable
streaks rather than one smooth gradient. The dab's bounds SHALL include the
full track reach, and the per-stroke coverage accumulation, opacity, flow, and
paint mode SHALL apply as for a smooth tip.

#### Scenario: A bristle stroke shows streaks

- **WHEN** a bristle tip paints a straight stroke
- **THEN** a cross-section through the stroke has more than one painted run
  separated by unpainted or lighter gaps

#### Scenario: Spacing still separates dabs

- **WHEN** a bristle stroke uses a large spacing percentage
- **THEN** its dabs are separated along the path as for a smooth tip

### Requirement: Stiffness deformation

Stiffness SHALL control how far each bristle bends away from its radial line:
low Stiffness SHALL move the track tips and collapse the shape, and Stiffness
100 SHALL leave the radial lines unchanged. The bend SHALL respond to the stroke
direction and pressure, so repeated strokes in different directions deform the
tip differently at low Stiffness.

#### Scenario: Low stiffness deforms the tip

- **WHEN** the same bristle tip paints at Stiffness 0 and at Stiffness 100
- **THEN** the low-stiffness dab's tracks are displaced relative to the
  high-stiffness dab's tracks

#### Scenario: High stiffness keeps the shape

- **WHEN** a bristle tip at Stiffness 100 paints two strokes in different
  directions
- **THEN** its track roots and directions stay on the same radial family

### Requirement: Per-bristle Mixer reservoir

A Mixer Brush stroke with a bristle tip SHALL keep one paint reservoir per
bristle. Wet pickup SHALL update the reservoirs individually and the deposit
SHALL use the strongest covering bristle's reservoir, so a wet bristle stroke
varies in colour across the tip and a dry one deposits the loaded colour
crisply. The mean reservoir SHALL be carried into the next stroke. A non-bristle
Mixer stroke SHALL keep the existing single-reservoir behaviour.

#### Scenario: A wet bristle stroke varies across the tip

- **WHEN** a bristle Mixer stroke crosses a colour boundary with Wet above zero
- **THEN** painted pixels across one cross-section do not all carry the same
  colour

#### Scenario: A dry bristle stroke stays crisp

- **WHEN** a bristle Mixer stroke with Wet 0 crosses a colour boundary
- **THEN** it deposits the loaded colour without dragging the canvas colour

### Requirement: Deterministic replay and CPU oracle

Every bristle track's randomness SHALL come from the stroke's fixed seed and
the dab index, so the same bristle stroke replays exactly. A bristle stroke
SHALL use the CPU rasterizer; the GPU stroke path SHALL decline it and fall
back to the CPU.

#### Scenario: The same bristle stroke replays

- **WHEN** the same bristle stroke is painted twice
- **THEN** both documents have identical pixels

#### Scenario: Bristle declines the GPU

- **WHEN** a stroke begins with a bristle tip and a GPU adapter is available
- **THEN** the stroke uses the CPU bristle rasterizer
