# Document lifecycle delta

## MODIFIED Requirements

### Requirement: Save and Save As

For an editable document, the system SHALL write the active document in its current output format, chosen
by the save path's extension, writing atomically so a failed write never
truncates an existing file. A `psd` path SHALL write the document's composite
and layer tree through the PSD codec; a `psb` path SHALL force a version-2 PSB
container through the codec. A supported raster path (PNG, JPEG, TIFF, WebP,
BMP, or GIF) SHALL write the flattened composite: PNG, JPEG, TIFF, WebP, and BMP
through the Qt-boundary raster encode edge, and GIF through the engine GIF89a
encoder (`interop/gif-save`). Save As SHALL default its format to PSD unless the
document is a single pixel or background layer imported from a supported raster,
in which case it SHALL default to that source format. When the document's
recorded path is a raster format and the document has more than one layer, a
group, an adjustment, or a type layer, Save SHALL behave as Save As rather than
silently flattening into that raster file. Saving a document that has more than
one layer, a group, an adjustment, or a type layer to a format that cannot hold
them SHALL warn before writing and then write the flattened composite. The
`composite` serialized to PSD SHALL be the active document's current rendered
composite, so a save after any edit round-trips to the edited pixels rather than
to a pre-edit merged image. The `write_psd` byte layout and its use of
`doc.composite.channels` to derive the PSD colour-plane count MUST NOT change; an
RGB document's merged composite is RGBA (four planes) after a canvas rebuild,
while a grayscale document keeps its composite colour-plane count. Save on an
untitled document SHALL behave as Save As. On success the save SHALL record the
file path. It SHALL clear the modified state only when the document is the same
document that was snapshotted when the write started — the same history
identity, content revision, and document generation — and that snapshot is still
current; a document changed after the snapshot SHALL keep the recorded path and
remain modified. When the document generation changed during the save (the
document was replaced), the replacement document's path and modified state SHALL
be unchanged. On failure the path and modified state SHALL be unchanged and no
temporary file SHALL remain.

#### Scenario: Save As writes a file

- **WHEN** the user runs Save As and chooses a path
- **THEN** a valid file of the chosen path's format is written at that path and the document is no longer modified

#### Scenario: Save uses the recorded path

- **WHEN** a document has a path and the user runs Save
- **THEN** the file at that path is overwritten atomically, in that path's format

#### Scenario: Save on an untitled document prompts for a path

- **WHEN** Save is invoked on an untitled document
- **THEN** a Save As path is requested before any write

#### Scenario: Save after an edit writes the edited composite

- **WHEN** an edit changes the canvas and the document is saved and read back
- **THEN** the read-back document's composite equals the edited rendered composite, not the pre-edit merged image, and its colour-plane count matches the saved document's colour-plane count

#### Scenario: Save As defaults to the import source format

- **WHEN** a single-layer PNG imported through `File > Open` is saved with Save As
- **THEN** the Save As dialog preselects PNG as the format

#### Scenario: A raster save clears modified and re-saves the same format

- **WHEN** a document is saved to `photo.png` and then edited and saved again
- **THEN** the second save writes a PNG to `photo.png` and clears the modified state

#### Scenario: A layered document saved to a flat format warns and flattens

- **WHEN** the user saves a document with more than one layer to a PNG
- **THEN** a warning is shown that the format cannot hold the document's features, and on confirmation the flattened composite is written as a PNG

#### Scenario: A layered document defaults Save As to PSD

- **WHEN** a PNG imported through `File > Open` has a layer or an adjustment added and Save As is invoked
- **THEN** the Save As dialog preselects PSD, not PNG

#### Scenario: Save on a layered raster path behaves as Save As

- **WHEN** a layered document recorded at a `.png` path is saved
- **THEN** the format-aware Save As dialog opens (defaulting to PSD) instead of overwriting the PNG with a flattened composite

#### Scenario: A PSB path writes a PSB container

- **WHEN** a document is saved to a `.psb` path
- **THEN** the written file is a version-2 PSB

#### Scenario: A completed save writes the snapshot and clears modified [dl_save_current]

- **WHEN** a modified document is saved through the worker and the document does not change while it runs
- **THEN** the file reopens with the snapshot's layers and composite, the recorded path is the saved path, and the document is unmodified

#### Scenario: An edit during a save leaves the document modified [dl_save_stale]

- **WHEN** the document's history or content revision changes while the save worker runs
- **THEN** the file holds the saved snapshot, the recorded path is the saved path, and the document remains modified

#### Scenario: A failed write leaves the document untouched [dl_save_failed]

- **WHEN** the target cannot be written
- **THEN** the save returns false, the path and modified state are unchanged, and no temporary file remains

#### Scenario: A replaced document keeps its path and dirty state [dl_save_replaced]

- **WHEN** the document is replaced while the save worker runs
- **THEN** the file holds the original document's snapshot, and the replacement document keeps its own path and modified state

## ADDED Requirements

### Requirement: Compatibility read-only state protects the opened document

When the document version policy requires read-only access, the document SHALL remain inspectable and identify its read-only state and reason. It SHALL use the source merged image when the image is present, decodable, and carries real merged data. When a faithful preview is unavailable, the application SHALL explain that limitation. Browsing and viewport navigation SHALL remain available.

Read-only access restricts the ordinary editing, Save and Save As flows for that document. Document changes and ordinary saves SHALL be refused across menu, direct tool, action playback, and programmatic application entry points. A refused operation SHALL leave file bytes, document content, path, modified state, and history unchanged. Opening or reverting to an editable document SHALL restore the normal lifecycle for that document.

#### Scenario: Read-only documents reject mutations and ordinary saves [dvp_read_only_guards]
- **WHEN** a read-only document is targeted by a menu edit, a canvas edit, an action step, an app save API, or Save or Save As
- **THEN** nothing is edited or written and the file, path, modified state, and history are unchanged

#### Scenario: Compatibility view uses available original merged pixels [dvp_preview]
- **WHEN** read-only documents open with a real usable merge and without one
- **THEN** the first shows the original merged image, and the second reports that a faithful preview is unavailable

#### Scenario: Document replacement refreshes access [dvp_replace]
- **WHEN** an editable document replaces a read-only session through open or revert
- **THEN** editing and saves follow the replacement document's policy

#### Scenario: Every output protects the compatibility source [dvp_output_source_guard]
- **WHEN** any output operation, including control-server `export_png` or an operation from a different active tab, targets the source path or a filesystem alias of any open read-only compatibility document
- **THEN** the operation refuses before writing and leaves source bytes and session unchanged; unrelated output destinations retain their ordinary behavior subject to their own copy restrictions

#### Scenario: Source composite becomes unavailable [dvp_source_decode_failure]
- **WHEN** source-backed merged pixels cannot be decoded because data is corrupt or the source changes after open
- **THEN** viewing reports the limitation and a compatibility copy fails with a reason without creating or modifying its destination
