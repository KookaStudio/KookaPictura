# Design

## Dialog control

`NewDocumentDialog` already keeps `background_` (the toolbox Background color)
and `custom_` (the last custom color). The new swatch reuses the shared
`ColorSwatchWidget`, as Canvas Size's extension color does: a fixed 24×22
button that paints its color and opens `ColorPickerDialog::getColor`.

## Choice model

`BackgroundChoice` becomes White, BackgroundColor, Transparent, CustomColor.
The combo carries the enum in item data so `spec()` and the swatch read one
source. White, Background Color, and Transparent are added at build time;
CustomColor is appended on the first accepted pick and stays available after
another choice is selected.

The combo's `currentIndexChanged` only updates the swatch state. The old
`activated` connection that opened the picker for the Custom… row is removed;
opening the picker is the swatch's job, so list selection never blocks.

## Swatch state

`updateBackgroundSwatch()` enables the swatch for every non-transparent choice,
shows white, `background_`, or `custom_`, and leaves the last shown color while
Transparent is selected. `pickBackgroundColor()` uses the current fill as the
picker's initial color, appends and selects Custom on acceptance, and leaves the
choice untouched on cancel.

`spec()` maps White to white, BackgroundColor to `background_`, CustomColor to
`custom_`, and Transparent to a transparent document whose fill is unused.

## Tests

`tst_new_document` drives the modal picker with a zero-delay timer, as
`tst_type_color_preview` does, and asserts the list, swatch state, and the
document fill.
