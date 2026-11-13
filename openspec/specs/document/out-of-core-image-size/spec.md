# document/out-of-core-image-size Specification

## Purpose
Image Size resamples every plane into spillable tiles a strip at a time, reading only the source windows its taps reach, and is gated on RAM for its working set and on RAM plus free scratch-disk space for its resampled planes.

## Requirements

### Requirement: Image Size resamples into spillable tiles

`Image > Image Size` SHALL resample every layer channel, every layer mask, every
document alpha channel, every plane of a layer's retained 16/32-bit store and
the document's retained store one strip of 256 destination rows at a time, into
tiled planes. Each block of destination pixels SHALL read only the source rows
and the column span that its samples' taps reach, and SHALL split until that
window is at most 1 Mpx. Tiles whose samples are all equal SHALL hold no
allocation. The output SHALL be byte-identical to the dense resamplers: the
8-bit resampler for 8-bit planes and the native-precision resampler for retained
stores.

#### Scenario: Tiled Image Size matches the dense resampler [ocr_resize_exact]

- **WHEN** a seeded 8-bit document with a full-canvas layer, an offset layer
  with a mask, and an alpha channel is resized to 150 %, to 50 %, to 7 % by
  31 %, and to 233 % by 100 %, each time with Nearest, Bilinear and Bicubic
- **THEN** every channel, mask and alpha channel is tiled and equals the
  dense resampler's output byte for byte

#### Scenario: Retained stores resample through tiles [ocr_resize_deep]

- **WHEN** a 16-bit and a 32-bit document, each with a retained layer store
  holding a `-2` mask plane and a retained document store, are resized to
  150 % with Bicubic
- **THEN** every retained plane is tiled and equals the native-precision
  dense resampler's output byte for byte

#### Scenario: Uniform tiles hold no allocation [ocr_uniform_tiles]

- **WHEN** a 2048 × 2048 document whose one layer is a single solid colour is
  resized to 150 %
- **THEN** each channel's allocated bytes are less than a quarter of the dense
  plane's size

#### Scenario: The resized tiles can be spilled [ocr_output_spills]

- **WHEN** a 1024 × 1024 seeded document is resized to 150 % with a scratch
  volume configured, and eviction passes are run that spill every live tile
- **THEN** some of the layer's tiles are spilled, and reading the layer back
  equals the dense resampler's output

#### Scenario: A large Image Size completes under a hard memory limit [ocr_memory_max]

- **WHEN** memory scenario M15 runs at 17000 × 17000 under `MemoryMax=2G` and
  again under `MemoryMax=4G`
- **THEN** each run resizes the document without an OOM kill or a refusal

### Requirement: An Image Size is gated on RAM plus scratch

An Image Size that resamples SHALL be admitted only when its working set fits
the governor's room. Its working set is one source window per worker thread, one
output strip of the widest plane, and the dense composite when the document's
composite is authoritative. Its resampled planes, at their new sizes and sample
widths, SHALL fit in that room plus the free space on the usable scratch
volumes. When the working set does not fit, the resize SHALL be refused as a
memory refusal. When the planes do not fit, it SHALL be refused with a
scratch-full refusal.

#### Scenario: A large Image Size spills instead of refusing [ocr_gate_spills]

- **WHEN** the cost of resizing a 17000 × 17000 document with one full-canvas
  layer to 150 % is gated with the governor's room pinned at 1.75 GiB and
  64 GiB of scratch available
- **THEN** the resize is admitted and is expected to spill

#### Scenario: Full scratch disks refuse an Image Size [ocr_gate_scratch_full]

- **WHEN** the same resize is gated with 1.75 GiB of room and no scratch
  available
- **THEN** it is refused, and the refusal is recorded as a scratch-full
  refusal
