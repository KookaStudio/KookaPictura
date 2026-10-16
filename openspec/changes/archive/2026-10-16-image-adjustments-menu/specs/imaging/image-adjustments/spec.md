## ADDED Requirements

### Requirement: Image > Adjustments dialogs

Each Image > Adjustments dialog entry SHALL open a dialog on the adjustment's
CS6 defaults whose controls preview on the active pixel layer within the
selection; Cancel SHALL restore the layer exactly and record nothing, and OK
SHALL apply the adjustment as one state named for it. The entries SHALL be
enabled only when the active layer can take a destructive edit.

#### Scenario: Preview, cancel, apply on an opened image

- **WHEN** the `tst_image_adjustments` test opens an image, checks the menu, and in a Hue/Saturation dialog drags Lightness to -100, cancels, then repeats and presses OK and undoes
- **THEN** the dialog and direct entries are enabled (Levels… with Ctrl+L, Match Color still disabled), the canvas darkens during the preview, Cancel restores the pixels with no state, OK records one "Hue/Saturation" state, and undo restores the image

#### Scenario: Every dialog opens on its defaults

- **WHEN** the `adjustment_defaults` unit test builds each of the fourteen dialog blocks
- **THEN** each decodes under its CS6 title and every control starts at its default

### Requirement: Direct adjustment commands

Invert, Desaturate, Equalize, and Auto Tone / Auto Contrast / Auto Color SHALL
apply at once to the active pixel layer within the selection, as one state
named for the command, leaving transparency unchanged.

#### Scenario: Invert in a selection, Desaturate, Equalize

- **WHEN** the `tst_image_adjustments` test inverts the left half of a selection, then desaturates and equalizes the image, and the unit tests equalize three greys and invert a layer region
- **THEN** only the selected half inverts, the image turns neutral grey, Equalize spreads the levels present to the ends of the range (a flat image is unchanged), and alpha is never touched
