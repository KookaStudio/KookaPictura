# Spec Delta: document/document-lifecycle

## ADDED Requirements

### Requirement: Raster Open records the file path

A raster opened as a document through File > Open, Open Recent, or a drop on the
workspace SHALL record the opened file as the document's current path. Save
SHALL then write that file in place without a Save As dialog, and Revert SHALL be
available. A raster reload performed by Revert SHALL record the same path again.
The path-less imports (`Open As Smart Object` and `Place`) SHALL remain
untitled.

#### Scenario: A raster open keeps its path

- **WHEN** a PNG is opened through `openDocumentAtPath`, edited, and saved with `saveActive()`
- **THEN** `documentPath(0)` equals the PNG path, `saveActive()` returns true without a Save As dialog, the file is rewritten in place, and Revert reloads the same path

### Requirement: Save As format choice drives the codec

When Save As is used and a single-format filter is selected, the application
SHALL write that filter's format. A conflicting suffix typed in the file name
SHALL be replaced with the filter's canonical suffix (`.psd`, `.psb`, `.png`,
`.jpg`, `.tif`, `.webp`, `.bmp`, or `.pdf`); the typed suffix SHALL NOT silently
choose a different codec. The `All Formats` entry SHALL keep the typed suffix
and choose the codec from it. The document's recorded path SHALL be the path
actually written.

#### Scenario: The chosen format wins over a typed suffix

- **WHEN** Save As has the PNG filter selected and the user types `report.jpg`
- **THEN** the written file is `report.png`, it decodes as PNG, and `report.jpg` is not created
