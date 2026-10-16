## ADDED Requirements

### Requirement: Snap Canvas To Workspace command in the command table

The system SHALL declare the `View > Snap Canvas To Workspace` command in the
declarative command table under the stable identifier `view.snapCanvasToWorkspace`,
with the label `Snap Canvas To Workspace`, no default shortcut, and a checked
state that defaults to unchecked. The command SHALL be checkable. Triggering it
SHALL flip the window's workspace-snap flag, apply the new state to every open
document's canvas immediately, and persist the state to the session store; the
command SHALL never alter document pixels or record a history state. The
command's checked provider SHALL report the current workspace-snap flag so the
checkmark tracks the state. Changing the label or shortcut SHALL NOT change the
identifier.

#### Scenario: The command is present and unchecked by default [lct_ws_snap_default]

- **WHEN** the View menu is opened with a default session
- **THEN** `View > Snap Canvas To Workspace` is present, checkable, and unchecked

#### Scenario: Triggering applies to every open canvas [lct_ws_snap_apply]

- **WHEN** `view.snapCanvasToWorkspace` is triggered while several documents are
  open
- **THEN** every document's canvas adopts the new workspace-snap state and no
  document pixels or history state change

#### Scenario: The toggle persists [lct_ws_snap_persist]

- **WHEN** Snap Canvas To Workspace is toggled and the session is saved and
  reloaded
- **THEN** the checked state is restored on the next launch
