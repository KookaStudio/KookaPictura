# ui/workspaces Specification

## Purpose
Named and built-in panel-arrangement workspaces that can be switched, saved,
deleted, reset, and remembered across restarts, exposed through the
`Window > Workspace` menu.

## Requirements

### Requirement: Built-in preset workspaces

The system SHALL expose the CS6 `Window > Workspace` presets as menu commands.
Essentials, Painting, Photography, and Typography SHALL be implemented built-in
workspaces with factory panel layouts built from the panels the application
provides. The presets that the application cannot express (3D, Advanced 3D,
Motion, and New in CS6) SHALL NOT appear in the menu at all; the menu SHALL NOT
show disabled placeholder workspaces.

#### Scenario: Selecting a preset applies its layout

- **WHEN** the user selects an implemented preset from `Window > Workspace`
- **THEN** the widget panel columns are rearranged to that preset's remembered
  arrangement, or its factory layout when none has been remembered, and it
  becomes the active workspace

#### Scenario: No placeholder presets are shown

- **WHEN** the `Window > Workspace` menu is shown
- **THEN** 3D, Advanced 3D, Motion, and New in CS6 are absent

### Requirement: Essentials factory layout

The Essentials built-in workspace SHALL have the factory arrangement of the CS6
default: a wider main right-hand column holding the groups `Color | Swatches`,
`Adjustments | Styles`, and `Layers | Channels | Paths`, and a narrower
secondary right-hand column collapsed to icons holding `History` and
`Properties`. The arrangement is a best-effort reconstruction of the CS6
default (there is no oracle for panel arrangements); its structure is normative,
not its pixel geometry.

#### Scenario: The Essentials main column holds three groups

- **WHEN** the Essentials workspace is applied
- **THEN** the main right-hand column contains the groups `Color | Swatches`,
  `Adjustments | Styles`, and `Layers | Channels | Paths`, in that order

#### Scenario: The Essentials secondary column is iconic

- **WHEN** the Essentials workspace is applied
- **THEN** a second right-hand column to the left of the main column is in
  iconic mode and holds `History` and `Properties`

### Requirement: Active workspace indicator

Exactly one workspace SHALL be active at a time. The `Window > Workspace` menu
SHALL show a checkmark beside the active workspace, the Options-bar switcher
SHALL show the active workspace as its value, both SHALL update when the active
workspace changes, and the same active workspace SHALL be restored on the next
launch.

#### Scenario: The active workspace is checked

- **WHEN** the `Window > Workspace` menu is shown
- **THEN** the active workspace is checked and no other workspace is

#### Scenario: The switcher shows the active workspace

- **WHEN** the frame is shown
- **THEN** the Options-bar switcher's value is the active workspace's name

#### Scenario: The checkmark and switcher follow a switch

- **WHEN** the user selects a different workspace
- **THEN** the checkmark moves to the newly active workspace and the switcher's
  value updates to it

#### Scenario: The active workspace survives a restart

- **WHEN** a workspace is active and the application is restarted
- **THEN** that workspace is active, checked, and shown in the switcher

### Requirement: Options-bar workspace switcher

The Options bar SHALL carry a workspace switcher at its right end that shows the
active workspace's name as its value and opens the `Window > Workspace` menu when
activated. The switcher SHALL be visible for every active tool, not only some
tools.

#### Scenario: The switcher opens the workspace menu

- **WHEN** the switcher is activated
- **THEN** the `Window > Workspace` menu is shown

#### Scenario: The switcher is always present

- **WHEN** the active tool changes
- **THEN** the Options-bar workspace switcher remains visible

### Requirement: Workspace menu grouping

The `Window > Workspace` menu SHALL be grouped into three sections separated by
separators: the selectable workspaces (built-in presets and user workspaces)
first, then the workspace actions (`New Workspace…`, `Delete Workspace…`,
`Reset [Workspace]`), then the additional items (`Keyboard Shortcuts & Menus…`).
A newly created user workspace SHALL appear in the workspace section, not the
actions section.

#### Scenario: The menu has three separated sections

- **WHEN** the `Window > Workspace` menu is shown
- **THEN** a separator follows the workspace list and another follows the
  workspace actions, before `Keyboard Shortcuts & Menus…`

#### Scenario: A new user workspace joins the workspace section

- **WHEN** a new user workspace is created
- **THEN** it is listed with the presets, above the first separator

### Requirement: New Workspace

`Window > Workspace > New Workspace…` SHALL prompt for a name and save the
current arrangement as a user workspace, make it the active workspace, and list
it in the `Window > Workspace` menu. The saved workspace SHALL persist across a
restart. The name SHALL be rejected with a message when it is empty or
whitespace-only, when it collides with a built-in workspace or an existing user
workspace, or when it exceeds the workspace-name length cap; a rejected name
SHALL NOT replace or overwrite any workspace.

#### Scenario: The current arrangement is saved

- **WHEN** the user chooses `New Workspace…` and enters a valid name
- **THEN** a user workspace with that name is created from the current layout,
  becomes active, and appears in the menu

#### Scenario: A user workspace survives a restart

- **WHEN** a user workspace is created, and the application is restarted
- **THEN** the workspace is listed and can be selected

#### Scenario: A built-in name is rejected

- **WHEN** the user enters the name of a built-in workspace
- **THEN** the creation is rejected and no workspace is replaced

#### Scenario: An empty or duplicate user name is rejected

- **WHEN** the user enters an empty name or the name of an existing user
  workspace
- **THEN** the creation is rejected and no workspace is replaced

### Requirement: Delete Workspace

`Window > Workspace > Delete Workspace…` SHALL always be available and SHALL
open a chooser listing every workspace except the active one. Choosing one SHALL
remove it permanently. The active workspace SHALL NOT be deletable. Built-in
preset workspaces SHALL be deletable like any other, and a preset deleted in one
session SHALL NOT reappear after a restart.

#### Scenario: A non-active workspace is deleted

- **WHEN** the user chooses `Delete Workspace…` and selects a non-active
  workspace
- **THEN** it is removed from the menu and does not reappear after a restart

#### Scenario: The chooser excludes the active workspace

- **WHEN** the delete chooser is shown
- **THEN** the active workspace is not among the choices

#### Scenario: A preset workspace is deletable

- **WHEN** the user deletes a built-in preset that is not active
- **THEN** it is removed and stays removed on the next launch

### Requirement: Reset workspace

`Window > Workspace > Reset [Workspace]` SHALL restore the active workspace to
its factory arrangement: the CS6 factory layout for a built-in workspace, or the
arrangement captured when a user workspace was created. The menu label SHALL name
the active workspace (for example `Reset Essentials`).

#### Scenario: Reset restores the factory layout

- **WHEN** the active workspace's layout has been changed and `Reset
  [Workspace]` is chosen
- **THEN** the factory arrangement for that workspace is applied

#### Scenario: The Reset label names the active workspace

- **WHEN** the menu is shown while `Photography` is active
- **THEN** the reset entry reads `Reset Photography`

### Requirement: Auto-remember of the last arrangement

Switching away from a workspace SHALL remember that workspace's current
arrangement. Switching back SHALL restore the remembered arrangement, not the
factory one, until the workspace is explicitly reset.

#### Scenario: A modified arrangement is remembered

- **WHEN** a workspace's layout is changed, another workspace is selected, and
  the first workspace is selected again
- **THEN** the changed arrangement is restored

#### Scenario: Reset re-arms the factory arrangement

- **WHEN** a workspace with a remembered modified arrangement is reset and then
  switched away from and back
- **THEN** the factory arrangement is restored

### Requirement: Workspace apply can re-group panels

Applying a workspace SHALL be able to move panels between groups so a preset can
define its own panel tab groupings, not merely reorder or hide the groups that
already exist. Applying a workspace SHALL cover the widget panel columns only and
SHALL leave the Tools column unchanged. Applying a workspace SHALL update the
same panel registry the `Window > Panels` toggles drive, so those checks reflect
the applied arrangement rather than a stale state.

#### Scenario: A preset re-groups a panel

- **WHEN** a preset whose factory layout places a panel in a different group is
  applied
- **THEN** that panel joins the group named by the preset

#### Scenario: The Window menu reflects an applied workspace

- **WHEN** a workspace is applied that shows or hides a panel
- **THEN** the corresponding `Window > Panels` checkmark reflects the applied
  visibility

#### Scenario: The Tools column is untouched

- **WHEN** any workspace is applied
- **THEN** the Tools column keeps its position and content

### Requirement: Workspace store

The system SHALL persist named workspaces and the active workspace under
`$XDG_STATE_HOME/kooka-pictura/workspaces/`, one atomically written file per
workspace plus an index that carries the schema version, the active workspace,
and the workspace order. Each workspace's remembered arrangement SHALL be durable
independently of the session state store, which only mirrors the active
arrangement. The store SHALL preserve unknown keys across a load-then-write. A
missing store, or an unparseable index, SHALL fall back to the built-in
workspaces without a crash, and a single unparseable workspace file SHALL drop
only that workspace. The built-in presets SHALL be seeded only when no store
exists, so a preset the user deletes stays deleted.

#### Scenario: Named workspaces round-trip

- **WHEN** workspaces and the active workspace are saved and the store is
  reloaded
- **THEN** the same workspaces and active workspace are restored

#### Scenario: The active workspace survives a lost session state

- **WHEN** the session state is discarded but the workspace store is intact
- **THEN** the active workspace and every named workspace's arrangement are still
  available

#### Scenario: Unknown keys survive a rewrite

- **WHEN** a workspace file or the index carries a key this build does not know
  and it is saved again
- **THEN** the unknown key is still present

#### Scenario: A corrupt store loads the built-ins

- **WHEN** the index cannot be parsed
- **THEN** only the built-in workspaces are available and the application does
  not crash

#### Scenario: A missing store loads the built-ins

- **WHEN** no store exists
- **THEN** Essentials is active and only the built-in workspaces are listed

### Requirement: Floating panel state in a workspace arrangement

A workspace's remembered arrangement SHALL include which widget groups and whole
widget columns are floating, each overlay's position and size, and the column
that owns each floating group. Switching away from a workspace and back,
applying a workspace that carries float markers, and loading such a workspace
after a store reload SHALL restore those floats at their remembered positions.
A workspace arrangement without float markers SHALL open with every widget panel
docked. Applying a workspace SHALL continue to leave the Tools column's docked
or floating state and position unchanged.

#### Scenario: Switching away and back remembers floats [wsp_floats_remembered]

- **WHEN** a workspace has a floating group or whole column, another workspace
  is selected, and the first workspace is selected again
- **THEN** the float is restored floating at its remembered position and size

#### Scenario: A saved workspace restores its floats [wsp_floats_roundtrip]

- **WHEN** a user workspace created from a floating arrangement is reloaded from
  the workspace store
- **THEN** its floating groups and columns are restored floating with their
  remembered geometry

#### Scenario: A workspace without float markers docks everything [wsp_floats_legacy]

- **WHEN** a workspace arrangement carries no floating state
- **THEN** every widget panel opens docked, including panels that are floating in
  the current arrangement

#### Scenario: A workspace apply leaves the Tools column alone [wsp_floats_tools]

- **WHEN** a workspace is applied while the Tools column is docked or floating
- **THEN** the Tools column keeps its current docked or floating state and
  position

### Requirement: The first-run layout is built once

When no panel arrangement is stored, constructing the main window SHALL arrange
the panels in the Essentials factory layout directly, without first building a
different arrangement and then regrouping it. The result SHALL equal the
arrangement that applying the Essentials workspace produces: the same columns,
the same groups in the same order, the same panel order within each group, the
same iconic and visible state, and the same active tab in each group.

#### Scenario: the first-run layout equals the applied Essentials layout [wsp_first_run_layout]

- **WHEN** a main window is constructed from an empty session and its arrangement is recorded, and the Essentials workspace is then applied to it
- **THEN** the arrangement after the application equals the recorded one: columns, groups and their order, panel order within each group, iconic and visible state, and each group's active tab
