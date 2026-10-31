# psd-decode-parallelism Specification

## Purpose
PSD and PSB layer channels and merged-composite rows are independent streams, so
the codec decodes them in parallel and returns exactly the bytes, and the first
error, that a serial decode returns.

## Requirements

### Requirement: Layer channels decode in parallel

Reading a layered PSD or PSB SHALL decode the layer and mask channel planes of
independent channels in parallel, and the decoded document MUST be identical to
the document the same file produces through the serial decode. Channel data
MUST be located from the already-parsed layer records, so no channel is decoded
from a cursor that another channel also advances.

#### Scenario: Every layer encoding is byte-identical [pd_layers]

- **WHEN** a layered file is read through the parallel path and through the
  serial path, covering RLE, ZIP, ZIP-with-prediction, depth 1, 16-bit, 32-bit,
  PSB, user masks (-2), real masks (-3), extra color channels, and folder and
  divider records
- **THEN** the two `Document` values are equal in every field, including layer
  channels, mask data, retained source samples, preserved raw channels, and
  layer compression

#### Scenario: A channel slice is never read from a shared cursor [pd_layers_slice]

- **WHEN** a file's channels are decoded in parallel
- **THEN** each channel's declared length is read from the layer record and its
  compression word and payload are read from that channel's own slice of the
  layer section, so the result does not depend on the order the parallel tasks
  run in

### Requirement: Merged RLE rows decode in parallel

Reading a PackBits RLE image-data section SHALL decode independent scanlines in
parallel, using the byte-count table read up front to locate every scanline.
The decoded planes MUST be byte-identical to the serial decode, and the merged
ZIP stream SHALL stay serial because it is a single deflate stream.

#### Scenario: A merged RLE composite is byte-identical [pd_rle]

- **WHEN** a file with an RLE image-data section is read through the parallel
  path and through the serial path, at depth 8, depth 16, depth 32, and depth-1
  Bitmap, with one and with several channels
- **THEN** the decoded composite planes are byte-identical

#### Scenario: The merged ZIP stream is not split [pd_zip_serial]

- **WHEN** the image-data section uses ZIP or ZIP-with-prediction
- **THEN** the deflate stream is inflated as one stream and the decoded planes
  are byte-identical to the serial decode

### Requirement: ZIP-with-prediction inversion runs per row in parallel

Inverting a ZIP-with-prediction plane SHALL process independent rows in
parallel at depth 8, 16, and 32, including the depth-32 four-byte-plane
unshuffle, and the recovered plane MUST be byte-identical to the serial
inversion.

#### Scenario: A predicted plane is byte-identical [pd_prediction]

- **WHEN** a depth-8, depth-16, and depth-32 ZIP-with-prediction composite and
  layer channel are read through the parallel path and through the serial path
- **THEN** the recovered planes and the narrowed 8-bit samples are byte-identical

### Requirement: Parallel decoding returns the first error in order

A malformed file SHALL report the same typed error as the serial decode. When
more than one channel or row fails, the error returned MUST be the one the
serial decode would hit first, in layer-record order for channels and in
scanline order for rows. A failure MUST NOT panic, and MUST NOT depend on task
scheduling.

#### Scenario: Truncated and over-long files report the same error [pd_errors]

- **WHEN** a fixture declares a layer channel longer than the bytes that remain,
  truncates the RLE count table or a scanline, or carries an over-long or
  under-long ZIP payload, and is read through the parallel path and through the
  serial path
- **THEN** both return the same `PsdError` variant

#### Scenario: The first of several failures wins [pd_errors_order]

- **WHEN** two layer channels fail with distinguishable errors and the earlier
  one in record order is not the one a task scheduler completes first
- **THEN** the error reported is the earlier channel's error, not whichever task
  finished first

### Requirement: The decode budget is charged before any channel decodes

Reading a file SHALL charge every layer and mask channel plane it decodes
against the decode budget before any channel is decoded, in layer-record order,
and a refusal SHALL remain the typed error defined by
`codec/untrusted-input-limits` with no plane allocated. A depth-8 channel whose
bytes are preserved verbatim rather than decoded into a plane SHALL NOT be
charged, as today.

#### Scenario: An over-budget channel is refused before decoding [pd_budget]

- **WHEN** a later layer channel exceeds the per-allocation or document budget
  while an earlier channel is malformed
- **THEN** the read fails with the budget error and the earlier malformed
  channel is not decoded

#### Scenario: The existing budget scenarios stay green [pd_budget_existing]

- **WHEN** the `codec/untrusted-input-limits` budget scenarios run after this
  change
- **THEN** every one still reports its recorded result

### Requirement: A serial execution path stays available to tests

The codec SHALL keep a serial decode that a test can select, and each parallel
decode SHALL have a test that compares its output with the serial output. The
serial path MUST NOT be observable in a production build.

#### Scenario: Parallel and serial agree for every decode stage [pd_serial]

- **WHEN** the same input is decoded with the parallel path and with the decode
  confined to a single thread
- **THEN** every output byte and the returned error are identical
