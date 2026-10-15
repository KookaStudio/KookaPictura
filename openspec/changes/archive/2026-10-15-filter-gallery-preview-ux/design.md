# Design

## Zoom control

The gallery already has `kZoomSteps` for the +/- buttons and the keyboard
shortcut. The zoom level becomes an editable `QComboBox` (`galleryZoom`) showing
the live percentage via `setEditText`, with the CS6 preset list as items and
separators between the low levels, 100 % and 200 %, and the named views. Item
data carries the factor; the named views use sentinels (`-1` Actual Pixels, `-2`
Fit in View, `-3` Fit on Screen) applied on `activated`. A typed percentage is
parsed on `editingFinished`. `setZoom`/`layoutPreview` update the box, so the +/-
buttons and the shortcut keep it in sync.

The wheel handler sits in the existing preview-viewport `eventFilter` and calls
`stepZoom`, the same helper the +/- buttons now use.

## Toggle icon

The gallery's thumbnails toggle already drives the pane's visibility; it now
shows `panel.rightClose` while the pane is shown and `panel.rightOpen` once
hidden, on the shared flat `QToolButton` surface (`setAutoRaise(true)`), like the
New/Delete effect buttons.

## Slider focus

`FilterParamControls` builds a slider with a `sliderPressed` handler that begins
a drag. That handler now also focuses the row's spin box, so a click on the
slider leaves the value box focused and Up/Down adjust the same parameter. The
`JumpSlider` overrides its press handler and does not take focus itself, so the
focus lands on the spin box.
