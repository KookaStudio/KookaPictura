# Spec Delta

## Purpose

Paste PDF clipboard artwork as embedded content or pixels, with editable destinations reserved for a fixture-backed follow-up.

## ADDED Requirements

### Requirement: Paste As follows the clipboard representations

Normal Edit > Paste SHALL be enabled and offer Paste As when the current clipboard contains valid `application/pdf` artwork, including when there is no internal clip or system image. Other paste commands SHALL retain their existing enablement gates. The dialog SHALL offer Smart Object, Pixels, Paths, and Shape Layer. Valid `application/pdf` artwork SHALL enable Smart Object and Pixels. Paths and Shape Layer SHALL remain disabled. Editable Illustrator clipboard support is a separate follow-up requiring a redistributable captured fixture and a verified supported representation. Normal raster and internal Pictura paste SHALL retain their current behavior for clipboards without a supported vector representation.

#### Scenario: Offer destinations for PDF-only clipboard data [vector_paste_pdf]
- **WHEN** the user pastes a clipboard containing valid PDF artwork without supported editable curves
- **THEN** Paste As offers Smart Object and Pixels and disables Paths and Shape Layer

#### Scenario: Editable destinations stay disabled pending a fixture [vector_paste_aicb]
- **WHEN** the user pastes PDF artwork with additional unverified Illustrator clipboard data
- **THEN** Smart Object and Pixels are enabled and Paths and Shape Layer are disabled

#### Scenario: Raster and native clipboard paste [vector_paste_raster]
- **WHEN** the user pastes a regular raster image or an internal Pictura clip without vector data
- **THEN** the normal paste path runs and no Paste As dialog opens

### Requirement: Paste As produces the selected document content

An accepted paste SHALL create the selected content and record one undo state. Smart Object SHALL use the vector placement path and preserve the PDF source bytes. Pixels SHALL create a raster layer with the artwork's transparency. Cancellation or conversion failure SHALL preserve the document and history. Failed vector conversion SHALL report failure and SHALL NOT paste stale image data.

#### Scenario: Paste a Smart Object or Pixels [vector_paste_layers]
- **WHEN** the user chooses Smart Object or Pixels for PDF artwork
- **THEN** one layer of the chosen kind with the expected appearance is added in one undoable step

#### Scenario: Cancel or reject a vector paste [vector_paste_refused]
- **WHEN** the user cancels the dialog or the selected conversion fails
- **THEN** the document and its history are unchanged, and a conversion failure reports an error
