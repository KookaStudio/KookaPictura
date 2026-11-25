# Design

## Window

`DebugToolsetPanel` is a top-level `Qt::Tool | Qt::FramelessWindowHint |
Qt::WindowStaysOnTopHint` window whose QObject parent is the main window, so it
stays above it and is destroyed with it. It is shown without activation. Its
default position is the top-right of the main window in global coordinates and is
not saved.

Dragging starts on the header: `QWindow::startSystemMove()` first (works on
Wayland), else the panel follows `globalPosition()` by hand. The panel is not
clamped to the main window or the screen. A `QSizeGrip` fills the bottom-right
corner and resizes the frameless window.

## Table

Each group is a `QFrame` (`debugToolsetGroup_<key>`) with a `QGridLayout`: a title
cell spanning both columns, then label and value `QLabel`s per row. Cells are
ruled by a stylesheet scoped to `#debugToolsetPanel`, so the colours do not touch
the global theme. ponytail: the yellow is fixed, not theme-derived, by request.

## Charts

`HistoryChart` (a paint-only widget in `debug_toolset_panel.cpp`) draws up to 60
samples, newest right. The vertical range is the series' own min to max, padded and
no narrower than a floor (8 MiB, 10 %), so a flat series reads as flat and a spike
reads as a spike. A caption shows now, low and peak. The panel owns the series and
clears them when it deactivates.

## Close

The close button calls `setActive(false)` then emits `deactivated()`. The main
window keeps the `Debug Toolset` check in step from that signal and after every
toggle.
