# ui/ui-scaling Specification

## Purpose
The application-wide UI scale keeps the frame chrome legible on high-resolution
displays. It scales fonts, stylesheet metrics, panels and controls from one
normalized percentage, applies live, persists across sessions, and leaves the
document canvas untouched.

## Requirements

### Requirement: UI scale contract

The theme SHALL own one application-wide UI scale percentage with the choices
75, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, and 200. The
default SHALL be 100. Setting an arbitrary percentage SHALL normalize it to the
nearest choice. Scaling a pixel value SHALL round `value * percent / 100` and
SHALL return at least one pixel for a positive value. At 100 percent every
existing metric SHALL keep its previous value.

#### Scenario: The default is 100 percent [uis_default]

- **WHEN** the application starts without a stored UI scale
- **THEN** the UI scale is 100 percent and a 12 px value remains 12 px

#### Scenario: Arbitrary values normalize to a choice [uis_normalise]

- **WHEN** the scale is set to 99, 84, or 250
- **THEN** it becomes 100, 80, and 200 respectively

#### Scenario: Scaled pixels round with a positive floor [uis_scaled_pixels]

- **WHEN** a positive pixel value is scaled at 75 percent
- **THEN** the result is the rounded product and never below one

### Requirement: Live scale application

A change to the UI scale SHALL apply immediately without restarting the
application. The application font, the stylesheet's pixel metrics, tab fonts,
tool-slot geometry, status-bar chrome, menu-corner icons, panel-column bounds,
and panel-group icon controls SHALL follow the new scale. The document image,
its zoom, and its rendered pixels SHALL NOT change. The chosen percentage
SHALL persist in the session and be restored before the chrome is built.

#### Scenario: Font and stylesheet follow the scale [uis_live_font_qss]

- **WHEN** the scale changes from 100 to 200 percent
- **THEN** the application font pixel size doubles and stylesheet lengths such
  as the menu-bar padding double

#### Scenario: Shell chrome and panels follow the scale [uis_live_chrome]

- **WHEN** the scale changes while the frame is open
- **THEN** the status bar, tool slots, menu-corner icons, and panel-column
  width bounds use the scaled metrics

#### Scenario: Canvas pixels are unaffected [uis_canvas_unaffected]

- **WHEN** the scale changes with a document open
- **THEN** the canvas's zoom and displayed document pixels are unchanged

#### Scenario: The scale survives a restart [uis_persist]

- **WHEN** the scale is set and the session is reloaded
- **THEN** the restored percentage is the stored percentage

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

### Requirement: Interface preferences scale control

The Interface preferences page SHALL list the same percentage choices as the
theme and SHALL select the current one. Choosing a value SHALL apply it live
through the same setter the Window menu uses. Changing the scale elsewhere
while Preferences is open SHALL update its selected value.

#### Scenario: The Interface page lists the scale choices [uis_pref_list]

- **WHEN** the Interface preferences page is shown
- **THEN** its UI Size control contains 75, 80, 90, 100, 110 through 200 percent

#### Scenario: Preferences and the Window menu stay in step [uis_pref_sync]

- **WHEN** the scale changes from Preferences and then from the Window menu
- **THEN** both surfaces show the resulting percentage

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
