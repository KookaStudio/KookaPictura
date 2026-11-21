# Proposal

## Why

The Pen and Freeform Pen always draw the Work Path. CS6 chooses Shape, Path, or
Pixels in the options bar, so the drawn path can become a shape layer, a saved
path, or pixels. Without that control a Pen path can only be turned into another
object afterwards (#810).

## What Changes

- The Pen and Freeform Pen options bar gains a Mode control with Shape, Path,
  and Pixels, defaulting to Shape as CS6 does.
- When a drawing session ends, the path is routed to the chosen mode:
  - **Shape** creates a shape layer from the drawn components and clears the
    Work Path. It lands on closing the path, Enter/Esc, Ctrl-click, a Freeform
    release, or a tool switch.
  - **Path** saves a copy of the drawn path as the next `Path <n>` saved path,
    leaving the Work Path in place for continued editing. It saves on closing
    the path, Enter/Esc, Ctrl-click, or a Freeform release; a tool switch only
    ends the session.
  - **Pixels** strokes the path onto the active pixel layer with the current
    brush and foreground colour, landing like Shape.
- The mode is shared by the Pen and Freeform Pen pages and persisted with the
  other Pen options.
- The Add/Delete/Convert anchor tools keep editing the current path; the mode
  applies only to the two drawing tools.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/pen-tool`: adds a requirement for the Pen/Freeform Pen Shape, Path, and
  Pixels modes and how a finished path lands.

## Impact

- `crates/pictura-app/cpp/tool_options.h` and `tool_options_json.cpp`:
  `PenOptions.mode`.
- `crates/pictura-app/cpp/options_bar_pen.cpp`: the Mode combo.
- `crates/pictura-app/cpp/tool_pen.cpp`: lands a finished path in the selected
  mode.
- `crates/pictura-app/src/cxxqt_object/paths.rs` and `shapes.rs`: the bridge
  calls that save a path copy and build a shape layer from the Work Path.
- `crates/pictura-app/cpp/tests/tst_pen_tools.cpp`: new mode cases; the
  existing Pen checks opt into Path mode. `tst_path_selection_tools.cpp` and
  `tst_type_on_path.cpp` do the same.
