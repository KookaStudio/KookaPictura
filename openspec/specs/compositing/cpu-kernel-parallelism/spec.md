# compositing/cpu-kernel-parallelism Specification

## Purpose
The O(region) per-pixel passes on the CPU render path run in parallel across
independent pixels, so a full-resolution composite uses the whole machine
without changing a single output byte.

## Requirements

### Requirement: Independent per-pixel kernels run in parallel

The CPU render path SHALL evaluate independent per-pixel kernels in parallel.
This covers the destructive and adjustment-layer application of every
applicable adjustment, the layer-effect matte mask multiply, the clipping blend
and its coverage collection, the canvas-to-8-bit conversion, and the display
colour transform of a profiled document. Each of these kernels MUST produce
output byte-identical to the same kernel run serially, and MUST NOT change any
value the serial execution returns.

#### Scenario: Adjustment application is byte-identical to the serial oracle [pk_parallel]

- **WHEN** every applicable adjustment kind is applied to the same buffer
  through the parallel path and through the serial oracle (the three fill
  variants return `Unsupported` and are skipped)
- **THEN** the two buffers are byte-identical

#### Scenario: A region composite is byte-identical [pk_region]

- **WHEN** a document with an adjustment layer is composited over a region of
  128 rows or fewer, once through the parallel path and once with the render
  crate's parallel execution disabled
- **THEN** the two buffers are byte-identical

#### Scenario: The display colour transform is byte-identical [pk_display]

- **WHEN** a non-sRGB document's composite is converted for display through the
  parallel path and through the serial oracle
- **THEN** the two buffers are byte-identical, including the untouched alpha
  plane

### Requirement: A serial execution path stays available to tests

Each crate that gains a parallel kernel SHALL keep a serial execution path that
a test can select, and each parallel kernel SHALL have a test that compares its
parallel and serial results bit-for-bit. The serial path MUST NOT be observable
in a production build.

#### Scenario: Parallel and serial agree for every touched kernel [pk_serial]

- **WHEN** the test-only serial switch is enabled and the same input is run
  through the parallel and serial branches
- **THEN** every output byte is identical

### Requirement: A layer-effect matte is blurred once

The matte a layer effect blurs SHALL be blurred as a single plane, and the
result MUST equal the three-plane blur the effect used before. The mask
multiply applied to that matte SHALL be evaluated in parallel and MUST produce
the same matte values as the serial loop.

#### Scenario: One-plane blur equals the three-plane result [pk_matte]

- **WHEN** a matte of any size is blurred as one plane and as three identical
  planes
- **THEN** the two results are byte-identical over random matte content

#### Scenario: Every layer effect keeps its bytes [pk_effects]

- **WHEN** a drop shadow, inner shadow, outer glow, inner glow, bevel, satin,
  overlay and stroke are rendered through the parallel path and through the
  serial oracle
- **THEN** the rendered effects are byte-identical

### Requirement: Parallelism does not change composed or saved output

Compositing and saving the same document MUST return the bytes they returned
before this change. Parallel execution MAY change only how long a kernel takes.

#### Scenario: The compositor oracle's output is unchanged [pk_unchanged]

- **WHEN** the full composite and the document oracles run on their existing
  documents after the change
- **THEN** every output buffer and encoded file is byte-identical to the
  recorded result

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
