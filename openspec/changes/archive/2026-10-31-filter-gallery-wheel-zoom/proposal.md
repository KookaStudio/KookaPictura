# Proposal: the Filter Gallery zooms at the pointer

## Why

The Filter Gallery's preview is a `QScrollArea` over a canvas sized to the
document times the zoom. The wheel handler drops `QWheelEvent::position()`
(`filter_gallery_dialog.cpp`), and `setZoom` only stores the factor and resizes
the canvas. The scrollbars keep their old pixel values, which now address a
different document point, so a panned detail jumps away on every zoom step: the
zoom behaves as if it were anchored at the viewport's top-left (#840).

The other preview dialogs are not affected: Liquify, Vanishing Point, Adaptive
Wide Angle, and Lens Correction use offset-based canvases whose wheel handler
already calls `zoomAbout(zoom, event->position())`, and whose buttons and
drop-down call `zoomAbout` with the widget centre. The Filter Gallery is the
only preview that relies on a scroll area, so it is the only one that lost the
anchor.

## What changes

- The wheel keeps the document point under the pointer fixed when zooming in.
- Zooming out, the `−`/`+` buttons, the zoom shortcuts, the drop-down, and a
  typed percentage anchor at the viewport centre, matching `ImageView::zoomAt`.
- One helper takes the anchor, so every zoom entry point uses the same math; the
  scroll clamps to its range at an edge, where the point cannot be kept.
- The zoom ladder, the preview crop, and the committed result are unchanged.

## Capabilities

### Modified Capabilities

- `imaging/filter-gallery`: the "Filter Gallery zoom control" requirement gains
  the anchor rule — a wheel step keeps the point under the pointer (zoom-in) or
  the viewport centre (zoom-out) fixed, and the buttons, shortcuts, drop-down,
  and typed percentages anchor at the centre, within one screen pixel unless
  the scroll range ends first.

## Impact

- `crates/pictura-app/cpp/filter_gallery_dialog.{h,cpp}`: a document-point
  lookup and an anchored zoom helper; the wheel passes its position.
- Tests: `crates/pictura-app/cpp/tests/tst_filter_gallery.cpp` gains
  `wheelZoomAnchorsAtTheCursor` and `buttonsAndDropdownAnchorAtTheCentre`.
- No new dependencies. No engine, bridge, codec, or persisted-state change.
