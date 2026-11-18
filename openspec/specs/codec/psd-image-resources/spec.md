# psd-image-resources Specification

## Purpose
Decodes and re-encodes PSD image-resource blocks into typed records via the shared codec reader.
## Requirements

### Requirement: Image resources are decoded into typed records

`pictura-codec` SHALL expose `decode_image_resources(document: &Document) ->
Vec<ImageResource>` that parses the document's preserved image-resource section
into typed records in stored order. Each record SHALL carry the resource `id`
(`u16`), its `name` (the Pascal string with the length byte removed and a
trailing NUL stripped, decoded lossily as text), its `data` bytes verbatim, and
the `raw` bytes of the whole block so it can be re-emitted. A block SHALL be
read as a 4-byte signature, a `u16` id, a Pascal name padded to an even length, a
`u32` data length, and that many data bytes padded to an even length. The
signatures `8BIM`, `8B64`, `MeSa`, `AgHg`, `PHUT`, and `DCSR` SHALL be accepted;
any other signature, a truncated block, or trailing bytes too short to frame a
block SHALL end parsing and the records decoded so far SHALL be returned. The
parser SHALL NOT panic and SHALL NOT allocate without bound on a malformed
section. When the section is written by the default raw codec write, `write_psd` SHALL reproduce it
byte-for-byte except that a resource `1039` the parser framed whose ICC
data-space signature does not match the output header color mode SHALL be
dropped.

#### Scenario: A resource section decodes to typed records

- **WHEN** the document carries an image-resource section holding an EXIF resource and an XMP packet
- **THEN** `decode_image_resources` returns records carrying those ids, names, exact data bytes, and raw block bytes in stored order

#### Scenario: The ICC profile is exposed by id

- **WHEN** the decoded records include resource id `1039`
- **THEN** the record's `data` is the embedded ICC profile bytes and the well-known id constant equals `1039`

#### Scenario: A malformed block is skipped without panicking

- **WHEN** the section ends with a truncated block or an unrecognized signature
- **THEN** `decode_image_resources` returns the records decoded before it and does not panic

#### Scenario: Byte preservation is unchanged for unnormalized documents

- **WHEN** a document read from a file with a non-empty image-resource section that was not ICC-normalized is written back using the default raw codec writer
- **THEN** the output's image-resource bytes are identical to the input's, except that a resource `1039` whose ICC data-space signature does not match the output header color mode is dropped

### Requirement: Image-resource data is parsed with the shared codec reader

The image-resource parser SHALL use the existing bounds-checked `pictura-codec`
`Reader`, so every read is length-checked and a truncated section yields a
recoverable end-of-parse rather than an index panic. It SHALL NOT introduce a
second byte cursor.

#### Scenario: Truncation is recoverable

- **WHEN** a resource block's declared data length exceeds the remaining bytes
- **THEN** parsing stops, the prior records are returned, and no panic occurs

### Requirement: Image resources are re-encoded from typed records

`pictura-codec` SHALL expose `encode_image_resources(resources: &[ImageResource])
-> Vec<u8>` that concatenates each record's raw block bytes in order, so that
`encode_image_resources(&decode_image_resources(doc))` reproduces the document's
image-resource section byte-for-byte. `ImageResource` SHALL carry the raw block
bytes it was decoded from.

#### Scenario: Decode then encode is byte-identical

- **WHEN** any committed fixture's image-resource section is decoded and re-encoded
- **THEN** the output equals the original section bytes

### Requirement: An image-resource block is framed from typed fields

`pictura-codec` SHALL expose `frame_image_resource(id: u16, name: &str, data:
&[u8]) -> ImageResource` that produces a block's `raw` bytes as a 4-byte `8BIM`
signature, the big-endian `u16` id, a Pascal name padded to an even length, the
big-endian `u32` data length, and the data padded to an even length, and that
sets the record's `id`, `name`, and `data`. A name longer than 255 bytes SHALL be
truncated to 255 bytes (the Pascal length is a `u8`), and the record's `name`
SHALL be the truncated form. Framing a record and then decoding the resulting
block SHALL recover the same `id`, `name`, and `data`.

#### Scenario: A framed odd-length value round-trips

- **WHEN** a record is framed from an id, a name, and odd-length data and the resulting block is decoded
- **THEN** the decoded record has the same id, name, and data

#### Scenario: A long name is truncated to the length byte

- **WHEN** a record is framed with a 300-byte name
- **THEN** framing and decoding recover the first 255 bytes as the name

### Requirement: Version Info 1057 reports native file metadata

The codec SHALL expose the Version Info resource version, merged-image flag, Unicode writer and reader names, and file version when 1057 uses the supported payload layout. Absence, unsupported payload versions, and malformed payloads SHALL be recoverable. An unknown or malformed 1057 SHALL be preserved as opaque data. The application SHALL show the typed values in File Info when available.

A version-aware native write SHALL create 1057 when absent. When a supported record exists, it SHALL retain its version, names, and file version. For a uniquely framed supported record the writer SHALL set its merged-image flag to reflect the real composite emitted by that save. If 1057 is malformed, unsupported, duplicated, or hidden by incomplete resource framing, version-aware writing SHALL refuse the save because consistency cannot be established; the raw bytes SHALL remain untouched. Opening SHALL remain recoverable and inspectable read-only with a reason until the ambiguity is resolved. This restriction is a save-safety decision and SHALL NOT change the private reader floor. It SHALL retain the original bytes when no field changes.

#### Scenario: Supported Version Info is readable and retains its fields [dvp_1057_fields]
- **WHEN** a PSD or PSB contains a supported 1057 with Unicode names and Pictura rewrites it
- **THEN** the parsed fields are exposed and the writer retains the original version, names, and file version

#### Scenario: Missing or malformed Version Info is recoverable [dvp_1057_recovery]
- **WHEN** 1057 is absent, malformed, or uses an unsupported payload version
- **THEN** typed fields are unavailable, a version-aware save creates a record only when absence is confirmed, and refuses malformed or unsupported records without changing their bytes

#### Scenario: Version Info reflects the composite that is written [dvp_1057_merged]
- **WHEN** version-aware PSD and PSB saves run with merged output enabled and disabled
- **THEN** 1057 reports real merged data only for outputs containing the real composite

#### Scenario: File Info shows supported Version Info [dvp_1057_file_info]
- **WHEN** a document with a supported 1057 opens and the user opens the Photoshop tab in File Info
- **THEN** the Version Info fields are visible

#### Scenario: Ambiguous Version Info prevents inconsistent writes [dvp_1057_ambiguous]
- **WHEN** a save encounters duplicate, malformed, or unsupported 1057, including a stale true flag while merged output is disabled
- **THEN** opening is inspectable read-only with a reason, version-aware writing fails without modifying the destination or opaque records, and raw codec preservation remains available

### Requirement: Recovery checkpoints carry autosave resources

New Pictura recovery checkpoints SHALL contain exactly one Auto Save File
Path resource (1086) and one Auto Save Format resource (1087). The path SHALL
name the checkpoint's final path and the format SHALL identify its actual PSD
or PSB container. Both values SHALL use PSD Unicode strings. Checkpoint
generation SHALL leave all other parseable resource records intact subject to the
existing writer's normalization rules; an unparsed suffix SHALL be preserved in
the recovery chain rather than placed before generated records in the checkpoint.

Ordinary native saves SHALL preserve imported 1086 and 1087 blocks verbatim
and SHALL NOT generate new ones. Recovery SHALL discover checkpoints through
the session store and SHALL NOT use these values as a path or format authority.
The recovery chain SHALL durably preserve the original complete raw image-resource
section, including imported 1086/1087 records, their order, duplicates, bytes,
and any unparsed suffix. This bounded, validated preservation data SHALL be
published and retired with its checkpoint. Recovery SHALL replace the entire
checkpoint resource section with the original bytes before replaying changes;
it SHALL NOT subsequently strip imported records by ID. If preservation data
cannot be read or validated, recovery SHALL retain the chain and report failure
without exposing an incomplete document or retiring its source. A later user
save SHALL NOT publish the generated local recovery path.

#### Scenario: Generated resources identify the checkpoint [ir_recovery_resources]

- **WHEN** Pictura writes PSD and PSB checkpoints to paths containing non-ASCII characters with existing 1086 and 1087 blocks present
- **THEN** each output has one Unicode path naming its final checkpoint and one Unicode format matching the container, and unrelated blocks keep their existing bytes

#### Scenario: Ordinary saves preserve imported autosave blocks [ir_foreign_recovery_resources]

- **WHEN** ordinary saves round-trip foreign PSD and PSB files with and without 1086 and 1087
- **THEN** existing blocks survive byte-for-byte and absent blocks remain absent

#### Scenario: The recovery manifest controls file discovery [ir_recovery_discovery]

- **WHEN** a checkpoint's stored 1086 and 1087 values disagree with its manifest and actual container, and the recovered document is then saved to a user path
- **THEN** recovery uses the manifest and actual file and the user output contains neither generated block

#### Scenario: Imported resource bytes survive recovery [ir_recovery_preserves_imports]

- **WHEN** an editable document with ordered duplicate imported 1086/1087 blocks is checkpointed, recovered after a crash, and saved normally
- **THEN** the imported blocks retain their order, duplicates, and bytes, and no generated local recovery path leaks into the save

#### Scenario: Recovery preserves an unparsed resource suffix [ir_recovery_unparsed]

- **WHEN** a recovery chain contains an unparsed raw resource suffix
- **THEN** recovery restores the complete original resource section byte-for-byte and the native document version policy leaves the recovered document inspectable read-only
