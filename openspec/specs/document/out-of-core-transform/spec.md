# document/out-of-core-transform Specification

## Purpose
A layer transform resamples into spillable tiles a strip at a time, and its commit is gated on RAM for its working set and on RAM plus free scratch-disk space for its output.

## Requirements

### Requirement: A transform resamples into spillable tiles

The layer transform SHALL resample each channel, the layer mask and every
retained deep plane one strip of 256 destination rows at a time, into a tiled
plane. Each block of destination pixels SHALL read only the source window its
samples' kernel taps reach, and SHALL split until that window is at most 4 Mpx.
Tiles whose samples are all equal SHALL hold no allocation. The output SHALL be
byte-identical to resampling the whole plane at once.

#### Scenario: Tiled output matches the dense resampler [oct_resample_exact]

- **WHEN** a seeded 8-bit layer with a mask is scaled by 0.5, scaled by 1.5 and rotated by 17°, downscaled by 0.02, and mapped through a perspective quad, with each interpolation
- **THEN** every channel and the mask are tiled, and they equal the dense resampler's output byte for byte

#### Scenario: Deep planes resample through tiles [oct_resample_deep]

- **WHEN** a 16-bit layer with a retained store is scaled by 1.5 and rotated by 10°
- **THEN** every retained plane is tiled and equals the dense typed resampler's output

#### Scenario: Uniform tiles hold no allocation [oct_uniform_tiles]

- **WHEN** a 2048 × 2048 layer of one solid colour is scaled by 1.5
- **THEN** each channel's allocated bytes are less than a quarter of the dense plane's size

#### Scenario: The output tiles can be spilled [oct_output_spills]

- **WHEN** a 1024 × 1024 seeded layer is rotated by 10° with a scratch volume configured, and an eviction pass is run that spills every live tile
- **THEN** some of the layer's tiles are spilled, and reading the layer back equals the dense resampler's output

#### Scenario: A cancelled tiled resample changes nothing [oct_cancel]

- **WHEN** a layer transform runs under a job whose cancel flag is already set
- **THEN** it reports cancelled, and the layer's rect and pixels are unchanged

### Requirement: A transform commit is gated on RAM plus scratch

A transform commit SHALL be admitted only when its dense working set (the
resample strips, the composite group buffers and one source window per worker
thread) fits the governor's room. Its spillable output (the new layer tiles and
the region composite) SHALL fit in that room plus the free space on the usable
scratch volumes. A whole-pixel move SHALL be charged no new layer tiles. When
the working set does not fit, the commit SHALL be refused as a memory refusal.
When the spillable output does not fit, the commit SHALL be refused with a
scratch-full refusal, and the message SHALL name the scratch disks.

#### Scenario: A large resize spills instead of refusing [oct_gate_spills]

- **WHEN** a 17000 × 17000 full-canvas layer is scaled by 1.5, with the governor's room pinned at 1.75 GiB and 64 GiB of scratch available
- **THEN** the commit is admitted

#### Scenario: Full scratch disks refuse with their own message [oct_gate_scratch_full]

- **WHEN** the same commit is gated with 1.75 GiB of room and no scratch available
- **THEN** it is refused, the scratch-full refusal is recorded, and the transform refusal message names the scratch disks

#### Scenario: A working set larger than RAM is a memory refusal [oct_gate_working]

- **WHEN** a commit is gated with less room than its working set and ample scratch
- **THEN** it is refused as a memory refusal and the scratch-full refusal is not recorded

### Requirement: A Move tool commit is gated on RAM plus scratch

A Move tool commit of pixel layers SHALL be gated as a whole-pixel transform
is. Its working set (the composite's strip buffers across the changed region's
width) SHALL fit the governor's room. The tiled composite of the changed region
(4 bytes a pixel) SHALL fit that room plus the free space on the usable scratch
volumes. It SHALL NOT be charged for the moved layers' planes. When the
composite does not fit, the move SHALL be refused with a scratch-full refusal,
the document and history SHALL be unchanged, and the move refusal message
SHALL name the scratch disks.

#### Scenario: A large move spills instead of refusing [oct_move_gate_spills]

- **WHEN** the commit cost of moving a 17000 × 17000 full-canvas layer by (5, 5) is gated with the governor's room pinned at 1.75 GiB and 64 GiB of scratch available
- **THEN** the commit is admitted, although 12 bytes a pixel of the changed region exceeds the room

#### Scenario: A move needs room only for its tiled composite [oct_move_commits]

- **WHEN** a 4100 × 4100 full-canvas layer is moved by (5, 5) with room for 4 bytes a pixel of the changed region but not 12, and no scratch available
- **THEN** the move is recorded in history and no memory refusal is recorded

#### Scenario: Full scratch disks refuse a move with their own message [oct_move_scratch_full]

- **WHEN** the same move is committed with room for its working set but not its composite, and no scratch available
- **THEN** it is refused, the scratch-full refusal is recorded, history is unchanged, and the move refusal message names the scratch disks
