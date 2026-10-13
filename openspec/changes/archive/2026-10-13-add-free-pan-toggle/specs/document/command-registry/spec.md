## ADDED Requirements

### Requirement: Free Pan command in the command table

The system SHALL declare the `View > Free Pan` command in the declarative
command table under the stable identifier `view.freePan`, with the label
`Free Pan`, no default shortcut, and a checked state that defaults to checked
(free pan on). The command SHALL be checkable. Triggering it SHALL flip the
window's free-pan flag, apply the new state to every open document's canvas
immediately, and persist the state to the session store; the command SHALL never
alter document pixels or record a history state. The command's checked provider
SHALL report the current free-pan flag so the checkmark tracks the state.
Changing the label or shortcut SHALL NOT change the identifier.

#### Scenario: The command is present and checked by default [lct_free_pan_default]

- **WHEN** the View menu is opened with a default session
- **THEN** `View > Free Pan` is present, checkable, and checked

#### Scenario: Triggering applies to every open canvas [lct_free_pan_apply]

- **WHEN** `view.freePan` is triggered while several documents are open
- **THEN** every document's canvas adopts the new free-pan state and no document
  pixels or history state change

#### Scenario: The toggle persists [lct_free_pan_persist]

- **WHEN** Free Pan is toggled and the session is saved and reloaded
- **THEN** the checked state is restored on the next launch
