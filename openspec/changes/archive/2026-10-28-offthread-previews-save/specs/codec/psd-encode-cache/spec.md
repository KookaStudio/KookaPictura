# Spec Delta

## Purpose

Reuse encoded PSD channel streams across saves and recovery snapshots, so an
unchanged layer is not encoded again while the writer produces the same bytes as
a fresh encode.

## ADDED Requirements

### Requirement: Encoded streams are keyed by source content

The PSD writer SHALL reuse an encoded layer channel stream only when the source
plane's content identity and every input to the encoding match: the source
plane's identity token, its pixel dimensions, the output sample depth, the
channel compression, the PSD or PSB container version, and whether the stream is
a color channel or a layer mask. A reused layer stream SHALL be byte-identical
to a fresh encode of the same plane and parameters. A plane whose bytes changed
SHALL NOT reuse a stream encoded from its earlier bytes.

The composite image-data section SHALL be cached as one block, not per channel:
RLE writes every plane's scanline counts before every row, and ZIP and
ZIP-with-prediction deflate the planes together. Its key SHALL be the composite
buffer's identity token, pixel dimensions, output mode, compression, output
sample depth, PSD or PSB container version, and channel count. A synthesized
Lab, CMYK, or Indexed composite block SHALL NOT be cached. A reused composite
block SHALL be byte-identical to a fresh encode of the same block.

#### Scenario: A cache hit equals a fresh encode [pec_hit_bytes]

- **WHEN** a document is encoded through a session cache, encoded again unchanged, and the same document is encoded with no cache
- **THEN** the cached second write and the uncached write are byte-identical

#### Scenario: A changed plane is re-encoded [pec_changed_plane]

- **WHEN** one pixel layer is edited after a cached encode and the document is encoded again through the same cache
- **THEN** that layer's channel bytes differ from the cached stream and the file still round-trips

### Requirement: Layer channels encode in parallel

The writer SHALL encode independent layer channels on multiple worker threads
while emitting records and channel data in the same order as a serial encode.
The parallel output SHALL be byte-identical to the serial output. A failing
channel SHALL fail the encode and produce no output.

#### Scenario: Parallel output equals serial output [pec_parallel_identical]

- **WHEN** a multi-layer document is encoded with parallel channel encoding and with a single-threaded encode
- **THEN** both outputs are byte-identical

#### Scenario: A channel error fails the encode [pec_parallel_error]

- **WHEN** a layer channel cannot be encoded, such as a channel length mismatch or an oversized synthetic RLE scanline
- **THEN** the encode returns an error and produces no output

### Requirement: The encode cache is session-scoped and pruned

The application SHALL keep one encode cache per document session and SHALL drop
it when a document is installed. After each successful encode the cache SHALL
discard entries that encode did not use. The cache SHALL expose counters for
channel hits, channel misses, layers with at least one encoded channel, and
resident bytes. The cache SHALL NOT change the bytes written.

#### Scenario: Entries unused by the latest encode are dropped [pec_pruned]

- **WHEN** a layer is deleted and the document is encoded again through the same cache
- **THEN** the deleted layer's encoded streams are no longer resident and a later encode cannot hit them

#### Scenario: The counters report the work [pec_counters]

- **WHEN** a 20-layer document is encoded, one layer is edited, and it is encoded again
- **THEN** the second encode reports one layer with encoded channels, and the unchanged layers count as channel hits

### Requirement: Unchanged non-RGB layer planes skip the color round trip

For a normalized CMYK or Lab document, the writer SHALL decide whether a
retained **layer** source plane is unchanged from the working layer plane's
content identity rather than by converting the retained plane back to RGB. When
it is unchanged the writer SHALL re-emit the retained plane; when it changed the
writer SHALL convert the working plane through the document's color profile.
When the retained store's working identity is unknown, the writer SHALL keep the
existing forward-conversion check. The composite's CMYK and Lab forward checks
are out of scope for this identity path.

The identity path is not byte-identical to the forward-conversion check in one
case: when a plane's identity is a known mismatch but its working RGB bytes
still forward-convert equal (for example a no-op edit after a profile
conversion), the identity path converts and may write different encoded bytes.
The file SHALL still reopen to the same working RGB within the documented
conversion tolerance.

#### Scenario: An unchanged CMYK layer reuses its retained planes [pec_cmyk_reuse]

- **WHEN** an unchanged 8-bit CMYK document is written
- **THEN** every unchanged layer's CMYK channel stream equals the retained source stream, and the writer performs no CMYK-to-RGB conversion to choose it

#### Scenario: An edited CMYK layer re-encodes [pec_cmyk_edited]

- **WHEN** a CMYK document's pixel layer is edited and the document is written
- **THEN** that layer's CMYK channel stream comes from the working RGB through the document profile, and the other unchanged layers still reuse their retained streams
