## MODIFIED Requirements

### Requirement: Feather

`feather(radius)` SHALL blur the coverage with a separable Gaussian to build a
transition ramp across hard edges. A radius of 0 or less, or a non-finite
radius, MUST return the input unchanged. A positive radius MUST produce
intermediate coverage values at edges and MUST preserve monotonicity of the
ramp. The Gaussian MUST use the inferred mapping `sigma = radius / 2` with
clamped edge sampling. For `sigma >= 2` the implementation SHALL approximate
the Gaussian with three variance-matched box passes per axis, whose widths
`w_i` satisfy `sum (w_i^2 - 1) / 12 = sigma^2`, so a pixel costs `O(1)`
instead of `O(sigma)`; the approximation MUST keep the ramp's monotonicity,
its width, and its 50 % crossing, and MUST stay within 6 coverage levels of
the direct Gaussian on a hard edge whose ramp is clear of the canvas edge.

#### Scenario: Zero radius is identity

- **WHEN** `feather(0.0)` is applied
- **THEN** the returned selection equals the input byte for byte

#### Scenario: A hard edge becomes a ramp

- **WHEN** a hard-edged rectangular selection is feathered with a positive radius
- **THEN** some boundary pixels hold values strictly between 0 and 255 and coverage falls monotonically across the edge

#### Scenario: A wide kernel stays close to the direct Gaussian

- **WHEN** a hard step is feathered with radius 4, 8, 20, 100, or 250
- **THEN** every coverage byte is within 6 levels of the direct separable
  Gaussian with `sigma = radius / 2`, and the 50 % crossing moves by less
  than one pixel
- **AND WHEN** a disk whose ramp is clear of the canvas edge is feathered
  with radius 4, 8, or 20
- **THEN** every coverage byte is within 6 levels of the direct Gaussian
