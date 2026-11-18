# PSD image resources delta

## MODIFIED Requirements

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

## ADDED Requirements

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
