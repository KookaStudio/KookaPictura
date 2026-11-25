# Proposal

## Why

Review of the first Debug Toolset (#1589) found the panel hard to read and
awkward to place: rows run together so it is unclear which belong to which
metric, it cannot leave the main window, it has no close control or resize cue,
and spikes in RAM and CPU are invisible between 2-second readings.

## What Changes

- **Look.** Light-yellow background, dark text.
- **Table.** Each group is a bordered table: a titled header band, then one
  `metric | value` row per metric with ruled cells.
- **Rename.** "Performance" becomes "Pictura Resource Usage".
- **History charts.** Under "Busiest", two line charts (RAM, CPU) of the last 60
  samples (two minutes), newest at the right, kept only while the toolset is on.
- **Close.** A close button at the top right turns the toolset off: it stops
  every metric collection, drops the history, and unchecks the menu item.
- **Drag icon.** A grip icon at the top centre marks the drag handle.
- **Floats outside.** The panel becomes its own tool window (like a floating
  toolbar), so it can be dragged anywhere on screen, outside the main window; it
  stays above the main window and closes with it.
- **Resize.** A size grip in the bottom-right corner shows, and provides, resizing.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `ui/debug-toolset`: panel placement and chrome, the Performance group.

## Impact

- `crates/pictura-app/cpp/debug_toolset_panel.{h,cpp}` rewritten for the layout;
  `frame_system_info.cpp` keeps the menu check in step with the panel.
- `tst_debug_toolset.cpp` updated and extended.
- No engine, bridge, or dependency change; charts are painted with QPainter.
