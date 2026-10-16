# export-as-dialog Specification

## Purpose

The CC-style Export As dialog: a layer list, a live preview, File Settings
(format and its controls), Image Size with a Resample menu, Canvas Size with a
reset, and a bounded output-size estimate. It is a CC-only surface (CS6 has no
Export As dialog) recorded as a scope extension.

## ADDED Requirements

### Requirement: The Export As dialog surface

`File ▸ Export As…` SHALL open one modal dialog containing a layer list on the
left, a preview with zoom controls in the center, and a settings column on the
right with a **File Settings** group, an **Image Size** group, and a **Canvas
Size** group, closed by `Cancel` and `Export…`. File Settings SHALL hold the
Format list (PNG, JPG, PNG-8, GIF) and the controls that apply to the chosen
format. Selecting a list row SHALL redraw the preview from that row's image
after sizing and canvasing. The dialog SHALL NOT mutate the document and SHALL
NOT add a history state.

#### Scenario: The dialog presents the three sections

- **WHEN** Export As is opened on a document
- **THEN** the dialog shows a layer list, a preview, a File Settings group, an Image Size group, and a Canvas Size group, with Cancel and Export buttons

#### Scenario: Selecting a row previews it

- **WHEN** the user selects a different layer row
- **THEN** the preview redraws from that row's image

#### Scenario: Opening Export As changes nothing

- **WHEN** the user opens Export As and cancels
- **THEN** the document's pixels, path, modified state, and history are unchanged

### Requirement: Image Size and Canvas Size controls

The Image Size group SHALL hold Width, Height, Scale (percent), and a Resample
menu, with Width and Height linked by a chain toggle (default on) so editing one
updates the other and the scale, and editing the scale updates both. The Resample
menu SHALL offer Bicubic, Bicubic Automatic, Bicubic Sharper, Bicubic Smoother,
Bilinear, Nearest Neighbor, and Preserve Details. The Canvas Size group SHALL
hold Canvas Width and Canvas Height and a `Reset` button; the output SHALL be
canvas-sized with the resampled image placed at the top-left, padded with
transparency or cropped. `Reset` SHALL set the canvas back to the current image
size.

#### Scenario: Width and height stay linked

- **WHEN** Width is edited with the chain on
- **THEN** Height and Scale update to keep the document's aspect ratio

#### Scenario: The Resample menu offers the CC options

- **WHEN** the Resample menu is opened
- **THEN** it lists Bicubic, Bicubic Automatic, Bicubic Sharper, Bicubic Smoother, Bilinear, Nearest Neighbor, and Preserve Details

#### Scenario: Canvas Size crops the output

- **WHEN** Canvas Width and Height are set smaller than the image size
- **THEN** the encoded output is the canvas size

#### Scenario: Reset restores the canvas

- **WHEN** the canvas has been changed and Reset is pressed
- **THEN** Canvas Width and Height return to the current image size

### Requirement: The output-size estimate is bounded

The dialog SHALL show an output-size estimate for the selected row and format.
The estimate SHALL be computed only after a short debounce rather than on every
change, and SHALL be omitted for a target above a pixel cap, where the label
SHALL show only the output dimensions. A large target SHALL NOT trigger a
full-resolution encode.

#### Scenario: A small target shows a size

- **WHEN** the selected target is below the pixel cap
- **THEN** the dialog shows an estimated output size for the chosen format

#### Scenario: A large target omits the size

- **WHEN** the selected target is above the pixel cap
- **THEN** the dialog shows the output dimensions and no size estimate
