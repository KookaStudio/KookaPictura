# Design

## Context

The dialog's outer layout is one horizontal row: the preview pane (with its own
zoom and Shadows/Highlights bottom row) beside the right panel (histogram, tab
strip, panel stack, buttons). The preview's bottom row carries a 4 px bottom
inset, while the right panel's buttons sit on the layout's bottom, so the two
rows differ by four pixels. The histogram owns one readout label set from the
whole preview's average. The raw open keeps a `PendingRaw` in the view until
the dialog commits; the document exists only for the filter path.

## Decisions

**One full-width status bar below the body.** A root `QVBoxLayout` holds the
preview/right `QHBoxLayout` and a status row. The status row carries the file
name left, the photo summary right, and a `QSizeGrip` at the far right. This
gives the grip a corner below the OK button, keeps the name and dimensions
visible without stealing width from the histogram, and lets the body's bottom
row align on one baseline once the preview's 4 px inset is removed.

**Photo information comes from one Rust function.** `photo_info(view)` returns
`key<TAB>value` rows. For a pending raw it reads the `RawPhotoInfo` captured at
decode time; otherwise it reads the document. The C++ dialog only formats the
rows, so the raw and document paths share one shape and one test seam. A raw
file's EXIF is parsed once, while its bytes are already in hand, and stored in
`PendingRaw`; the dialog never re-reads a 20 MB file.

**Defaults are explicit.** A document without a ResolutionInfo resource and a
raw without an XResolution tag report 72 ppi, the app's default. Missing
exposure tags produce no row, so the histogram labels stay empty rather than
showing a dash.

**The picker samples the rendered preview.** The preview stores the drawn image
rect during paint and maps a mouse position through it to `image_`. It emits the
pixel's RGB; the histogram shows it and falls back to the average readout when
the pointer leaves or the position is outside the image. A re-render while the
pointer is still over the image re-samples the last position, so dragging a
slider keeps the picker live.

## Risks

- The status bar shortens the preview by its height. The dialog's default
  1100x680 leaves room, and the preview already letterboxes.
- `QSizeGrip` in a layout resizes the top-level window; it must be a child of
  the dialog, which it is.
- The raw EXIF is read from the container's IFD0/Exif IFD. A container that
  keeps its exposure tags only in a SubIFD will show no exposure line; the
  requirement allows omission.
