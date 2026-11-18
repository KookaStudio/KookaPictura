## ADDED Requirements

### Requirement: The native writer streams into a seekable sink

The PSD and PSB writer SHALL write a document into a seekable byte sink, such
as the save's temp file. It SHALL write section lengths, layer-record channel
lengths and RLE scanline-count tables as placeholders and patch them once the
data they describe has been written. The bytes in the sink SHALL equal the
bytes of the in-memory encode of the same document, for every container
version, layer and composite compression, sample depth and color mode. The
in-memory encode SHALL be the same writer writing into a growable buffer.

#### Scenario: A streamed file equals the in-memory encode [sps_identical]

- **WHEN** a corpus of documents (RGB and grayscale, PSD and PSB; RLE, raw, ZIP and ZIP-with-prediction layers and composites; masks, extra channels, a 16-bit source) is written to a file through the sink and encoded in memory
- **THEN** every file's bytes equal the in-memory encode

#### Scenario: The writer's bytes are unchanged [sps_golden]

- **WHEN** a fixed matrix of documents and compressions is encoded by the streaming writer
- **THEN** the SHA-256 digest of each encode equals the digest the previous writer produced for the same input

### Requirement: The native writer holds a bounded working set

When it writes an 8-bit RGB or grayscale document, the writer SHALL hold at
most one band of rows for each channel being written and the scanline-count
tables. It SHALL NOT hold a whole plane, a whole encoded channel, the whole
encoded composite or the whole encoded file. This SHALL hold for RLE, raw, ZIP
and ZIP-with-prediction channels. 16/32-bit, CMYK, Lab, Indexed, Bitmap and
Duotone outputs, which re-emit or convert the retained native stores, are
exempt and are marked as deliberate shortcuts in the code.

#### Scenario: A large RLE save stays bounded [sps_bounded]

- **WHEN** a 4096 × 4096 RGB document with three noise layers is written RLE into a file with no encode cache
- **THEN** the writer's heap peak is below 64 MiB, while the file is larger than 100 MiB

#### Scenario: A large ZIP save stays bounded [sps_zip_bounded]

- **WHEN** the same document is written with ZIP and with ZIP-with-prediction layers and composite into a file
- **THEN** the writer's heap peak is below 64 MiB for each

### Requirement: The merged image is composited into tiles

When a save writes a merged image composited from the layers, the save view
SHALL composite it band by band into tiled planes of the channels it writes.
It SHALL NOT copy the composite into a flat buffer, and SHALL NOT materialize a
tiled plane. A stored composite whose alpha the save drops SHALL be copied band
by band into tiled planes.

#### Scenario: The save view keeps the merged image tiled [sps_merged_tiled]

- **WHEN** the save view of a layered RGB document with an extra channel is built, and the same for a layerless document whose stored composite carries alpha
- **THEN** each composite is a tiled 3-channel plane, its samples equal the RGB of the full composite, and no tiled plane was materialized

### Requirement: A failed streamed save leaves the target untouched

An encode error, or a failed integrity check, after the writer has started
writing into the temp file SHALL remove the temp file and SHALL leave the
target path as it was.

#### Scenario: An encode error leaves no file [sps_error_no_file]

- **WHEN** a save over an existing file fails partway through its layer channels
- **THEN** the existing file's bytes are unchanged and no temp file remains in its directory
