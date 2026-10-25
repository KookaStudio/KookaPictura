# Spec Delta

## ADDED Requirements

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

The Window menu SHALL show a disabled readout of the current percentage and
SHALL provide Enlarge UI Size, Decrease UI Size, and Reset UI Size. Enlarge and
Decrease SHALL move one step through the choice list. Enlarge SHALL be disabled
at 200 percent, Decrease at 75 percent, and Reset at 100 percent.

#### Scenario: Enlarge and decrease step through the choices [uis_menu_step]

- **WHEN** Enlarge UI Size runs at 100 percent and Decrease runs at 90 percent
- **THEN** the scale becomes 110 and 80 percent respectively

#### Scenario: The ends and reset disable appropriately [uis_menu_bounds]

- **WHEN** the scale is 75, 200, or 100 percent
- **THEN** Decrease, Enlarge, and Reset are disabled respectively

#### Scenario: The menu shows the current percentage [uis_menu_readout]

- **WHEN** the scale is 130 percent
- **THEN** the Window menu readout says `UI Size: 130%`

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
