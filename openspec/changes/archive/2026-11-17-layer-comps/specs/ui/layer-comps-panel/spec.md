# Spec Delta

## Purpose

A dedicated Layer Comps dock that creates, applies, updates, cycles, and deletes
document comps and surfaces cannot-be-restored warnings.

## ADDED Requirements

### Requirement: The panel is its own dock

The system SHALL register a Layer Comps panel as a plain content widget with a
stable object name, hosted by the panel column like every other panel. It SHALL
be hidden in a fresh session, SHALL NOT be part of the default visible groups,
and SHALL be reachable from a checkable `Window > Panels > Layer Comps` toggle
whose checked state follows the panel's visibility. The panel SHALL dock, float,
regroup, and restore through the session's panel-column layout. Opening or
closing it SHALL NOT modify the document, its history, or its modified state.
User-visible labels SHALL use the application's localization path.

#### Scenario: The panel starts hidden [lcp_dock_hidden]

- **WHEN** a fresh session starts
- **THEN** the Layer Comps panel is registered but not visible

#### Scenario: The Window menu toggles the panel [lcp_dock_toggle]

- **WHEN** `Window > Panels > Layer Comps` is invoked
- **THEN** the menu item is checkable, the panel becomes visible and the item is checked, and invoking it again hides the panel

#### Scenario: The panel restores with the session [lcp_dock_restore]

- **WHEN** the panel is moved to a group, the session is saved, and the app restarts
- **THEN** the panel is in that group with the same visibility

#### Scenario: Toggling changes nothing in the document [lcp_dock_unchanged]

- **WHEN** the panel is shown and hidden
- **THEN** the document, history, and modified state are unchanged

### Requirement: Rows show the comp list and Last Document State

The panel SHALL show Last Document State as the first row and the comps in
document order beneath it. A comp row SHALL show its name, an apply affordance,
and a caution affordance when its warning is active. The name SHALL be the
editable field; the row SHALL show the comp's comment as a tooltip when one is
present. A document with no comps SHALL show only Last Document State. The panel
SHALL support single selection and contiguous or discontiguous multi-selection.
The list SHALL follow the active document and SHALL refresh when the document
changes, a document is opened, or the active document switches.

#### Scenario: Rows follow the comp order [lcp_rows_order]

- **WHEN** a document has comps A, B, and C
- **THEN** the panel lists Last Document State, A, B, and C in that order

#### Scenario: No comps [lcp_rows_empty]

- **WHEN** a document has no comps
- **THEN** the panel shows only Last Document State

#### Scenario: Multi-selection [lcp_rows_multi]

- **WHEN** the user Shift-clicks or Ctrl-clicks comp rows
- **THEN** the selected rows form a contiguous or discontiguous selection

#### Scenario: The panel follows the document [lcp_rows_switch]

- **WHEN** the active document switches to another document
- **THEN** the panel lists the new document's comps and discards the previous selection

#### Scenario: Layer deletion updates warnings [lcp_rows_refresh]

- **WHEN** a captured layer is deleted in the Layers panel
- **THEN** the affected comp rows show the caution affordance without reopening the document

### Requirement: Create opens the comp dialog or skips it with Alt

A Create New Layer Comp action SHALL open one dialog with Name, Comment,
Visibility, Position, and Appearance controls. Visibility SHALL default on and
Position and Appearance SHALL default off; the dialog SHALL remember the
recorded-class choices for the next comp. Name SHALL default to `Layer Comp N`.
OK SHALL create the comp and close the dialog; Cancel SHALL change nothing.
Alt-clicking Create SHALL create the comp without opening the dialog, using the
remembered defaults and the default name.

#### Scenario: The dialog defaults [lcp_create_defaults]

- **WHEN** the New Layer Comp dialog opens for the first time
- **THEN** Visibility is checked and Position and Appearance are not

#### Scenario: The dialog remembers the last choices [lcp_create_remember]

- **WHEN** a comp is created with Position checked and the dialog is opened again
- **THEN** Position is still checked

#### Scenario: Alt-click creates without a dialog [lcp_create_alt]

- **WHEN** Create is Alt-clicked
- **THEN** a comp is created immediately with the default name and remembered classes and no dialog is shown

#### Scenario: Cancel changes nothing [lcp_create_cancel]

- **WHEN** the dialog is cancelled
- **THEN** no comp is added and the document is unchanged

### Requirement: Apply, Previous, and Next drive the comp state

The panel SHALL apply a comp when its row's apply affordance is clicked or the
Apply action is used, and SHALL record one history state per apply. Previous and
Next SHALL move the applied comp to the neighbouring row and apply it. When more
than one comp is selected, Previous and Next SHALL cycle only the selected
comps; otherwise they SHALL cycle every comp. There SHALL be no wrap: Previous
is unavailable on the first comp and Next on the last. Applying SHALL update the
Layers panel to the restored visibility, blend, and style state.

#### Scenario: The row affordance applies [lcp_apply_row]

- **WHEN** a comp row's apply affordance is clicked
- **THEN** that comp is applied and one history state is added

#### Scenario: Previous and Next cycle [lcp_apply_cycle]

- **WHEN** comp B is applied and Next is used repeatedly
- **THEN** C is applied, then Next is unavailable

#### Scenario: Selected comps cycle alone [lcp_apply_selected]

- **WHEN** comps A and C are selected and Next is used from A
- **THEN** C is applied and B is skipped

#### Scenario: The Layers panel follows [lcp_apply_layers]

- **WHEN** a comp with recorded visibility and blend is applied
- **THEN** the Layers panel shows the restored eye and blend states

### Requirement: Last Document State is a read-only pseudo-row

Last Document State SHALL appear as a row and MAY be selected, but SHALL NOT be
renamed, updated, deleted, or opened in Options. Its apply affordance SHALL be
unavailable until a change-making comp apply has happened in the session; after
an apply it SHALL restore the pre-apply layer state and record one history
state.

#### Scenario: Unavailable before an apply [lcp_last_disabled]

- **WHEN** no comp has been applied
- **THEN** Last Document State cannot be applied

#### Scenario: Restoring it [lcp_last_apply]

- **WHEN** a comp has been applied and Last Document State is applied
- **THEN** the pre-comp layer state returns and one history state is added

#### Scenario: It cannot be edited [lcp_last_readonly]

- **WHEN** Update, Delete, Rename, or Options is invoked with Last Document State selected
- **THEN** the action is disabled or refused and the document is unchanged

### Requirement: Update, Delete, Rename, Options, and Duplicate

The panel SHALL update the selected comp from the current state, delete it,
rename it inline, open its options dialog, and duplicate it. Each action SHALL
record at most one undoable metadata state. Renaming SHALL be available by
double-clicking the comp name and by the panel menu; double-clicking elsewhere
on the row SHALL open the options dialog. A new comp name that is empty SHALL be
refused. Delete SHALL be available from the Delete action, the panel menu, and
the Delete key when the list has focus. Duplicate SHALL create a comp with the
same comment, recorded classes, and captured state and a new identity.

#### Scenario: Update [lcp_edit_update]

- **WHEN** Update is invoked on a comp after a recorded change
- **THEN** the comp reflects the current state and one history state is added

#### Scenario: Delete [lcp_edit_delete]

- **WHEN** Delete is invoked on a comp
- **THEN** the comp is removed and one history state is added

#### Scenario: Inline rename [lcp_edit_rename]

- **WHEN** the comp name is double-clicked, a new name is typed, and editing ends
- **THEN** the comp is renamed and one history state is added

#### Scenario: Options dialog [lcp_edit_options]

- **WHEN** a comp row is double-clicked outside the name
- **THEN** the options dialog opens with its name, comment, and recorded classes

#### Scenario: Duplicate [lcp_edit_duplicate]

- **WHEN** Duplicate is invoked
- **THEN** a new comp with the same captured state and a new id appears after the source comp

#### Scenario: Empty name is refused [lcp_edit_empty_name]

- **WHEN** the name is cleared and committed
- **THEN** the old name remains and no history state is added

#### Scenario: Metadata edits record one state each [lcp_edit_history]

- **WHEN** Options is confirmed, Duplicate is invoked, or Delete is pressed with the list focused
- **THEN** each action adds exactly one history state and changes no pixels

### Requirement: The panel menu carries the comp actions

The panel's menu SHALL offer New Layer Comp, Layer Comp Options, Rename Layer
Comp, Duplicate Layer Comp, Delete Layer Comp, Clear Layer Comp Warning, and
Clear All Layer Comp Warnings. Actions that need a selected comp SHALL be
disabled without one; the warning actions SHALL be enabled only when a warning
exists.

#### Scenario: The menu is complete [lcp_menu]

- **WHEN** the panel menu is opened
- **THEN** it offers the seven listed actions

#### Scenario: Actions follow the selection [lcp_menu_enabled]

- **WHEN** no comp is selected
- **THEN** update, rename, duplicate, and delete are disabled while New remains enabled

### Requirement: Warnings are explained and clearable

A comp with a warning SHALL show a caution affordance. Clicking or
right-clicking it SHALL offer an explanation and Clear Layer Comp Warning; the
panel menu SHALL offer Clear All Layer Comp Warnings. Clearing SHALL remove the
affordance without changing the comp's captured state or the remaining layers
and SHALL be undoable. Clearing all warnings SHALL record exactly one metadata
state. Applying a warned comp SHALL still apply its resolvable entries.

#### Scenario: The caution affordance is shown [lcp_warn_icon]

- **WHEN** a captured layer is deleted
- **THEN** the comp shows a caution affordance

#### Scenario: Clear from the caution affordance [lcp_warn_clear]

- **WHEN** Clear Layer Comp Warning is chosen for a warned comp
- **THEN** the affordance disappears, the comp's captured state remains, and one history state is added

#### Scenario: Clear All records one state [lcp_warn_clear_all]

- **WHEN** Clear All Layer Comp Warnings is chosen for multiple warned comps
- **THEN** all warnings disappear and exactly one history state is added

#### Scenario: Applying a warned comp [lcp_warn_apply]

- **WHEN** a comp with one missing layer and one existing captured layer is applied
- **THEN** the existing layer is restored and the missing one is skipped

### Requirement: The panel respects the document and edit gates

With no document open the panel SHALL show no rows and every action SHALL be
disabled. While a modal transform session or another blocking edit session is
active, actions that mutate the document SHALL be unavailable or refused
without changing the document. The panel SHALL NOT crash or write to a document
that is closing or has been replaced.

#### Scenario: No document [lcp_gate_none]

- **WHEN** no document is open
- **THEN** the panel is empty and its actions are disabled

#### Scenario: A transform session blocks apply [lcp_gate_transform]

- **WHEN** a free-transform session is active and a comp row is applied
- **THEN** the apply is refused and the transform session and document are unchanged

#### Scenario: A closing document is not written [lcp_gate_close]

- **WHEN** the active document is closing or has been replaced while the panel refreshes
- **THEN** the panel clears its rows and writes nothing to the old document
