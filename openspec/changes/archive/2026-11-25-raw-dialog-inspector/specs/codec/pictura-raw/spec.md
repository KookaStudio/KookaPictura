# Spec Delta

## ADDED Requirements

### Requirement: Pictura Raw bottom bar

The dialog SHALL align the zoom/Fit controls, the Shadows/Highlights toggles,
and the Done/Cancel/OK actions on one baseline, and SHALL show a resize grip in
the bottom-right corner below the OK action.

#### Scenario: The bottom controls share a baseline [rb_baseline]

- **WHEN** the dialog is shown
- **THEN** the Shadows/Highlights toggles and the OK button have the same bottom
  edge

#### Scenario: A resize grip sits below OK [rb_size_grip]

- **WHEN** the dialog is shown
- **THEN** a visible size grip lies below the OK button at the bottom-right
  corner

### Requirement: Pictura Raw photo information

The dialog SHALL show the photo's name, colour space, bit depth, pixel
dimensions with megapixels, and resolution in ppi for the pending raw file or
the open document, plus the file size when the file is on disk. It SHALL show
the camera's f-number, exposure time, ISO, and focal length in the histogram
area when the source metadata provides them, and SHALL omit a value that is
missing. A raw file's resolution SHALL come from its EXIF when present, else
72 ppi; a document without a resolution resource SHALL report 72 ppi.

#### Scenario: A raw open shows the raw file's information [rb_photo_raw]

- **WHEN** the raw open dialog is shown for a 16 by 12 DNG named `shot.dng`
- **THEN** the status bar names `shot.dng` and reports 16 by 12 and 16 bits

#### Scenario: A filter shows the document's information [rb_photo_document]

- **WHEN** the filter dialog is shown for a 16 by 16 8-bit RGB document
- **THEN** the status bar reports 16 by 16, 8 bits, sRGB, and 72 ppi

#### Scenario: Missing camera metadata is omitted [rb_photo_missing]

- **WHEN** the source carries no EXIF exposure tags
- **THEN** the histogram's exposure labels are empty

### Requirement: Pictura Raw preview picker

Hovering the preview SHALL set the histogram's RGB readout to the pixel under
the cursor. Leaving the image SHALL restore the readout the preview's average
provides.

#### Scenario: Hovering samples the pixel [rb_picker_hover]

- **WHEN** the cursor is over a red pixel of the preview
- **THEN** the RGB readout shows that pixel's red, green, and blue values

#### Scenario: Leaving restores the average [rb_picker_leave]

- **WHEN** the cursor leaves the preview after hovering
- **THEN** the RGB readout returns to the default average readout
