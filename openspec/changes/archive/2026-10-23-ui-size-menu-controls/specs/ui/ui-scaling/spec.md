## MODIFIED Requirements

### Requirement: Window menu scale commands

The Window menu SHALL present one UI Size row stacked in three parts: a caption
row reading "UI size", a step row holding the minus button, the current
percentage readout, and the plus button, and a reset button on its own row below
the step row. The row SHALL remain open while its buttons are pressed, so
repeated steps require no new menu dropdown. The minus and plus buttons SHALL
move exactly one step through the choice list. The minus button SHALL be
disabled at 75 percent, the plus button at 200 percent, and reset at 100
percent. When the scale changes while the menu is open the menu SHALL resize to
fit its rows, so the caption and buttons are never clipped. The UI Size row
SHALL be the last row in the Window menu, placed below the Options item and
separated from it by a separator. The Enlarge UI Size, Decrease UI Size, and
Reset UI Size commands SHALL remain available for shortcuts and the shortcut
editor even though their menu rows are hidden.

#### Scenario: One stacked row replaces the separate step items [uis_menu_single_row]

- **WHEN** the Window menu is opened
- **THEN** it shows one UI Size row whose caption reads "UI size", with the minus
  button, percentage readout, and plus button on the step row and the reset
  button on the row below, and no separate Enlarge or Decrease rows

#### Scenario: The row follows Options [uis_menu_below_options]

- **WHEN** the Window menu is opened
- **THEN** the UI Size row is the last row, below the Options item and separated
  from it

#### Scenario: The open menu resizes as the scale changes [uis_menu_resize]

- **WHEN** a scale button is pressed while the Window menu is open
- **THEN** the menu grows to fit its rows, so the caption and buttons stay inside
  the menu rather than being clipped

#### Scenario: The row steps without reopening the menu [uis_menu_buttons]

- **WHEN** the plus button is pressed twice while the Window menu is open
- **THEN** the scale advances two steps and the menu stays open

#### Scenario: Enlarge and decrease step through the choices [uis_menu_step]

- **WHEN** Enlarge UI Size runs at 100 percent and Decrease runs at 90 percent
- **THEN** the scale becomes 110 and 80 percent respectively

#### Scenario: The ends and reset disable appropriately [uis_menu_bounds]

- **WHEN** the scale is 75, 200, or 100 percent
- **THEN** Decrease, Enlarge, and Reset are disabled respectively

#### Scenario: The menu shows the current percentage [uis_menu_readout]

- **WHEN** the scale is 130 percent
- **THEN** the Window menu readout says `UI Size: 130%` and the row's label
  shows 130 percent

## ADDED Requirements

### Requirement: UI scale shortcuts

The application SHALL bind `Ctrl+Alt++` to Enlarge UI Size, `Ctrl+Alt+-` to
Decrease UI Size, and `Ctrl+Alt+0` to Reset UI Size. These bindings MUST NOT
replace the unmodified Zoom In (`Ctrl++`), Zoom Out (`Ctrl+-`), or Fit On
Screen (`Ctrl+0`) shortcuts. The bindings SHALL be visible to the keyboard
shortcut editor and SHALL step or reset the scale through the same commands as
the menu row.

#### Scenario: Scale shortcuts are registered [uis_shortcut_bindings]

- **WHEN** the command registry is inspected
- **THEN** the three UI scale commands report the Ctrl+Alt bindings and the zoom
  commands keep their unmodified bindings

#### Scenario: A scale shortcut moves the scale [uis_shortcut_step]

- **WHEN** `Ctrl+Alt++` is pressed at 100 percent and `Ctrl+Alt+0` is pressed
  after a change
- **THEN** the scale becomes 110 percent and then returns to 100 percent

### Requirement: Window resizing at every scale

The frame SHALL remain resizable and maximizable at every UI scale. The tools
column SHALL scroll its content vertically rather than propagate the toolbox's
scaled minimum height to the frame. Raising the scale SHALL NOT make the frame's
minimum height exceed the available screen height.

#### Scenario: The tools column scrolls instead of pinning the window [uis_tools_scroll]

- **WHEN** the scale is raised to 200 percent
- **THEN** the tools column hosts a vertical scroll area and the frame's minimum
  height stays below the available screen height

#### Scenario: The frame remains resizable at a high scale [uis_window_resizable]

- **WHEN** the scale is 200 percent and the frame is resized smaller
- **THEN** the frame accepts the smaller size and stays resizable
