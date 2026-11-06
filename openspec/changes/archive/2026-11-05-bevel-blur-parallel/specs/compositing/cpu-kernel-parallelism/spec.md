# Spec Delta

## ADDED Requirements

### Requirement: Bevel Gaussian passes preserve floating-point results in parallel

The CPU renderer SHALL evaluate the independent rows of bevel height rounding
and highlight/shadow softening in parallel. For every finite, correctly sized
floating-point input field used by bevel, each pass MUST return the same
single-precision sample bits as the former serial convolution. Gaussian weights
and per-sample accumulation SHALL retain double precision, kernel tap order,
clamped-edge sampling, and single-precision rounding after each separable pass.
The existing radius-to-sigma mapping MUST remain unchanged. Empty fields SHALL
produce empty output without panicking.

#### Scenario: Floating-point samples equal the former serial blur [bb_field_bits]

- **WHEN** fixed nonuniform and uniform fields are blurred at fractional and
  large bevel radii through the parallel path and the former serial algorithm
- **THEN** every output sample has identical bits in both results, including
  pixels at the clamped edges and kernels wider than the field

#### Scenario: Empty and single-axis fields remain valid [bb_degenerate]

- **WHEN** empty, one-pixel, one-row, and one-column fields are blurred
- **THEN** the empty result is empty without panic and every nonempty result
  matches the former serial algorithm bit-for-bit

### Requirement: Independent bevel geometry passes preserve serial output

The independent columns and rows of the bevel distance transform, height
conversion, and shading SHALL run in parallel without changing their
mathematical models. The distance field MUST retain the sample bits returned
by the former serial transform, including empty and fully seeded fields.

#### Scenario: Parallel distance fields equal the former serial transform [bb_distance]

- **WHEN** empty, fully seeded, and deterministic asymmetric seed fields are
  transformed with one and multiple workers, including single-axis fields
- **THEN** every distance sample has the same bits as the former serial transform

### Requirement: CPU-specific Gaussian acceleration preserves scalar sample bits

Gaussian row sums SHALL use wider CPU instructions only when supported by the
running CPU, with a portable fallback on other machines. Both paths MUST retain
the scalar sample bits for single-precision, double-precision, and mixed
single-input/double-accumulator sums, including partial vector tails. Existing
8-bit Gaussian filter outputs MUST remain unchanged.

#### Scenario: Accelerated and portable row sums match scalar expressions [bb_simd]

- **WHEN** weighted rows of zero length, full vector widths, and partial vector
  tails are accumulated through the runtime-selected path and portable path
- **THEN** every accumulator has the same bits as the ordered scalar expression
  and existing 8-bit Gaussian digest tests still pass

### Requirement: Parallel bevel blur preserves rendered output

Replacing the bevel blur MUST preserve the composed bytes for every supported
bevel style and technique, including nonzero softening. The result SHALL be
independent of worker count, and existing filter callers MUST retain their
current floating-point arithmetic and output.

#### Scenario: Bevel rounding and softening are deterministic across worker counts [bb_composite]

- **WHEN** generated masked layers with each supported bevel style and technique
  are composited with softening off and on through the former serial blur and
  through the replacement using one worker and multiple workers
- **THEN** the replacement's complete composite buffers are byte-identical to
  the former-blur buffers and to each other between worker counts, and the
  existing bevel and filter regression checks continue to pass

### Requirement: Cold bevel composite latency is measured independently of cache reuse

Performance acceptance SHALL time the first full CPU composite after decoding in
a fresh process, excluding file reading and decoding and using the same
optimized build profile and worker count for baseline and changed code. The
large-bevel acceptance workload supplied for issue #1178 MUST composite in less
than four seconds on the machine used for acceptance. This hardware-specific
check SHALL be opt-in rather than a general CI wall-clock limit; reporting MUST
include the profile, worker count, and measured durations.

#### Scenario: The cold composite meets the acceptance target [bb_cold]

- **WHEN** the opt-in cold-composite profile runs against the acceptance workload
  on the acceptance machine, once per fresh process with unchanged settings
- **THEN** the changed implementation's first full CPU composite takes less
  than four seconds, with decoding excluded and timings reported separately
