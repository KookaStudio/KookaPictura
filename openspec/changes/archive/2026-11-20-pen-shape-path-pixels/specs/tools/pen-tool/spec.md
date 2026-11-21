## ADDED Requirements

### Requirement: Pen output modes

The Pen and Freeform Pen options bar SHALL offer Mode: Shape, Path, or Pixels,
defaulting to Shape. Shape and Pixels SHALL land when a drawing session ends —
by closing the path, Enter, Esc, Ctrl-click, a Freeform release, or switching
tools. Shape SHALL create a new shape layer from the drawn components and clear
the Work Path; Pixels SHALL stroke the path onto the active pixel layer with the
current brush and foreground colour. Path SHALL save the drawn path as the next
`Path <n>` saved path when it is closed or explicitly finished (Enter, Esc,
Ctrl-click, or a Freeform release) and leave the Work Path in place; switching
tools only ends the session. The Add, Delete, and Convert anchor tools SHALL NOT
change the output mode or land a path.

#### Scenario: The mode control is present and shared [pen_output_mode_bar]

- **WHEN** the `tst_pen_tools` mode test selects the Pen and then the Freeform Pen
- **THEN** both options bars show Shape, Path, and Pixels with Shape selected, and changing one page's mode changes the other's

#### Scenario: Shape mode creates a shape layer [pen_output_shape]

- **WHEN** the `tst_pen_tools` shape test closes a Pen square with Mode Shape
- **THEN** one shape layer is added, the Work Path is empty, the layer shows the foreground fill inside the square, and one "Pen Tool" state is recorded

#### Scenario: Path mode saves the path [pen_output_path]

- **WHEN** the `tst_pen_tools` path test closes a Pen square with Mode Path
- **THEN** a saved path named "Path 1" is added, and the Work Path still holds the four closed anchors

#### Scenario: Pixels mode strokes the layer [pen_output_pixels]

- **WHEN** the `tst_pen_tools` pixels test closes a Pen square with Mode Pixels
- **THEN** no layer is added, the square's edge is painted with the foreground colour, and one "Stroke Path" state is recorded
