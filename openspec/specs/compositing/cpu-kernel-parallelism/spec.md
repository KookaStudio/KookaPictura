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
