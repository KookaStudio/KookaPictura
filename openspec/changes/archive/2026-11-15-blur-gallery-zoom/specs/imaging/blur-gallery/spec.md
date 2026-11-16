# Spec Delta — imaging/blur-gallery

## ADDED Requirements

### Requirement: Blur Gallery preview zoom

The Blur Gallery workspace SHALL show an editable zoom field at the bottom-left
of its options bar. The field SHALL display the preview's live zoom percentage,
list the preset levels plus **Fit in View** and **Actual Pixels**, and apply a
typed percentage. The mouse wheel over the preview SHALL zoom in and out about
the pointer, and `Ctrl++` / `Ctrl+-` (and the `+` / `−` buttons beside the
field) SHALL step the zoom about the preview's centre. The zoom SHALL only
place the preview; the pin fractions and blur amounts committed by OK SHALL be
unchanged, and **Fit in View** SHALL restore the fitted opening view.

#### Scenario: The zoom field sets the preview zoom [blur_zoom_field]

- **WHEN** the workspace opens and the zoom field is typed to `200%`
- **THEN** the field opens on the fitted percentage, typing `200%` sets the
  preview to 200 %, and Fit in View restores the fitted percentage

#### Scenario: The wheel zooms about the pointer [blur_zoom_wheel]

- **WHEN** the mouse wheel is scrolled up and then down over the preview
- **THEN** the preview zooms in and then out

#### Scenario: The zoom shortcuts step the preview [blur_zoom_shortcuts]

- **WHEN** `Ctrl+=` and then `Ctrl+-` are pressed in the workspace
- **THEN** the preview zooms in and then out
