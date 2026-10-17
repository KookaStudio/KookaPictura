## ADDED Requirements

### Requirement: Photoshop's own PDFs reopen with their layers

A PDF whose first page carries Photoshop's editing data
(`/PieceInfo /AdobePhotoshop /Private` with `/ImageResources`,
`/StandardImageFileData`, and `/CompositeImage`) SHALL open with its layers,
without the Import PDF dialog: the data SHALL be rebuilt into a PSD and read
as one, so pixel layers keep their pixels and type layers stay type layers.
A PDF without that data, or one whose editing data cannot be rebuilt, SHALL
fall back to the Import PDF dialog.

#### Scenario: A Photoshop PDF keeps its type layer [pdf_photoshop_layers]

- **WHEN** a PDF holding Photoshop editing data for a background and a type layer `PDF FROM PHOTOSHOP` is opened
- **THEN** a document opens without the Import PDF dialog with the same two layers, the type layer is a type layer with that text, and the layer pixels and composite match

#### Scenario: Other PDFs still import [pdf_photoshop_absent]

- **WHEN** a PDF without Photoshop editing data is opened
- **THEN** no PSD is rebuilt and the Import PDF dialog is shown
