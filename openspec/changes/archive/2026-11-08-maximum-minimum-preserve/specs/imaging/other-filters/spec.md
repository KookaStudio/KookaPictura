# Spec Delta

## MODIFIED Requirements

### Requirement: Maximum and Minimum

The system SHALL implement `Filter::Maximum { radius: u32, preserve: MorphologyPreserve }` and `Filter::Minimum { radius: u32, preserve: MorphologyPreserve }`. For each color channel independently, every output sample SHALL be the maximum (`Maximum`) or the minimum (`Minimum`) of the input samples within the footprint centered on it, where `r` is `radius`. `MorphologyPreserve::Squareness` SHALL use the `(2r+1)²` square footprint. `MorphologyPreserve::Roundness` SHALL use the disk footprint: the offsets `(dx, dy)` with `dx² + dy² <= r²`. `radius == 0` SHALL be a bit-exact no-op. `radius` SHALL be clamped to 500; a request above 500 MUST behave as 500 and MUST NOT error. Both footprints MUST sample with clamp-to-edge in both axes, MUST run over the alpha plane of a four-channel buffer as they run over the color planes, and MUST be deterministic. Oracle expectation: differential against ImageMagick `-morphology Dilate Square:{radius}` (Maximum) and `-morphology Erode Square:{radius}` (Minimum) for `Squareness` within an absolute tolerance of 0. `Roundness` has no faithful ImageMagick operator and SHALL be classified as no-equivalent with known-value tests. The larger radius, the `Preserve` choice and the disk footprint are project extensions beyond the CS6 contract; Adobe's exact disk kernel is closed, so the footprint formula is the project's recorded approximation.

#### Scenario: Maximum grows bright regions and Minimum shrinks them

- **WHEN** Maximum and Minimum are each applied at the same non-zero radius to a white-on-black mask
- **THEN** the white region grows under Maximum and shrinks under Minimum

#### Scenario: Radius zero is a no-op

- **WHEN** Maximum or Minimum is applied with radius 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Radius above 500 clamps

- **WHEN** Maximum or Minimum is applied with radius 600 and with radius 500 to the same buffer
- **THEN** both outputs are bit-identical and no error is returned

#### Scenario: Matches ImageMagick

- **WHEN** Maximum or Minimum with a given radius is applied to the oracle image and ImageMagick applies the matching `Square:N` morphology
- **THEN** no sample differs by more than 1

#### Scenario: Squareness keeps the square footprint [morph_squareness]

- **WHEN** Maximum or Minimum runs with `Squareness` at radius 1 on a 3×3 buffer whose only bright sample is a diagonal neighbour of the inspected pixel
- **THEN** the diagonal sample is inside the footprint, so Maximum takes it and Minimum does not

#### Scenario: Roundness uses the disk footprint [morph_roundness]

- **WHEN** Maximum or Minimum runs with `Roundness` at radius 1 on the same 3×3 buffer, and then at radius 2
- **THEN** at radius 1 the diagonal sample is outside the disk and is ignored, and at radius 2 the diagonal offset `(1, 1)` is inside the disk and is taken

#### Scenario: Roundness moves the alpha plane too [morph_roundness_alpha]

- **WHEN** Maximum or Minimum runs with `Roundness` on a four-channel buffer whose alpha plane carries the same diagonal pattern
- **THEN** the alpha plane follows the same disk rule as the colour planes

#### Scenario: Radius reaches 500 and clamps above it [morph_radius_500]

- **WHEN** Maximum runs at radius 150 on a row with a bright sample 120 px away, and at radius 600 on a row with a bright sample 520 px away, compared with radius 100 and radius 500 respectively
- **THEN** radius 150 takes the 120 px sample that radius 100 misses, and radius 600 is bit-identical to radius 500 so the 520 px sample stays outside the clamped footprint

#### Scenario: The Preserve slot selects the footprint [morph_preserve_slots]

- **WHEN** `maximum` and `minimum` are mapped with an empty slot list, with `[radius, 0]`, and with `[radius, 1]`
- **THEN** the empty list and `[radius, 0]` build `Squareness`, `[radius, 1]` builds `Roundness`, and a list of any other length is refused
