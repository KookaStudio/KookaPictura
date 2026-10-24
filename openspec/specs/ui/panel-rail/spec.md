# panel-rail Specification

## Purpose
The panel set and the right icon rail that shares panel toggles with the Window menu.

## Requirements

### Requirement: Panel set

The system SHALL provide panels for Gradients, Patterns, Properties,
Adjustments, Channels, Paths, and Actions, each with a stable `objectName` and a
CS6-appropriate empty state or, for Actions, its implemented action-set content,
in addition to the existing Layers, History, Navigator, Color, Swatches, Info,
and Histogram panels. (The `Libraries` panel named by the previous revision is not
provided by this application; this change records its absence rather than
requiring it.) The panels SHALL be plain `QWidget` content widgets hosted by the
`PanelColumn` rather than `QDockWidget`s. The default CS6 Essentials set SHALL
place `Styles` as a tab of the `Adjustments` group and SHALL show `Properties` as
a standalone panel in the secondary iconic column; the Gradients and Patterns
panels SHALL remain registered and reachable from `Window > Panels` but SHALL
NOT be in a default visible group. The remaining placeholder panels SHALL show
an explicit empty state rather than fabricated content.

#### Scenario: Every panel exists as a content widget

- **WHEN** the frame is shown
- **THEN** each of the listed panels exists as a `QWidget` with its documented `objectName` and is hosted by the `PanelColumn`

#### Scenario: Properties shows its empty state

- **WHEN** the Properties panel is shown with nothing selected
- **THEN** it displays a "No Properties" empty state

#### Scenario: Styles takes the former placeholder slot

- **WHEN** the frame starts with a fresh session
- **THEN** `Styles` is a default-visible tab of the `Adjustments` group, not the third tab of the Color/Swatches group

#### Scenario: Actions is implemented content [pr_actions_content]

- **WHEN** the Actions panel is shown
- **THEN** it presents the action-set tree and bottom toolbar described by `ui/actions-panel`, not a placeholder empty state

### Requirement: Rail and Window menu share panel toggles

The `Window > Panels > <name>` command SHALL toggle its panel's visibility and
SHALL be the single command path for panel visibility. The panel's registry
visibility SHALL reflect the command's state, and SHALL track visibility changes
made from the column. The former rail buttons SHALL no longer exist.

#### Scenario: Window menu entry toggles the panel

- **WHEN** `Window > Panels > <name>` is invoked
- **THEN** the panel's visibility toggles and the column reflects the new state

#### Scenario: Grouped panels have menu toggles

- **WHEN** `Window > Panels > Gradients` (or Patterns, Properties, Adjustments, Libraries, Channels, Paths, Actions) is invoked
- **THEN** that panel is shown or hidden
