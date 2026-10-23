# Spec Delta

## MODIFIED Requirements

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
