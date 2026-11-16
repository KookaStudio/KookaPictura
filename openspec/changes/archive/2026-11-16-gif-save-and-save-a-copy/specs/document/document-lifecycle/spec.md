# Spec Delta

## MODIFIED Requirements

### Requirement: Save and Save As

The system SHALL write the active document in its current output format, chosen
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

### Requirement: Save As format choice drives the codec

When Save As is used and a single-format filter is selected, the application
SHALL write that filter's format. A conflicting suffix typed in the file name
SHALL be replaced with the filter's canonical suffix (`.psd`, `.psb`, `.png`,
`.jpg`, `.tif`, `.webp`, `.bmp`, `.gif`, or `.pdf`); the typed suffix SHALL NOT
silently choose a different codec. The `All Formats` entry SHALL keep the typed
suffix and choose the codec from it. The document's recorded path SHALL be the
path actually written.

#### Scenario: The chosen format wins over a typed suffix

- **WHEN** Save As has the PNG filter selected and the user types `report.jpg`
- **THEN** the written file is `report.png`, it decodes as PNG, and `report.jpg` is not created
