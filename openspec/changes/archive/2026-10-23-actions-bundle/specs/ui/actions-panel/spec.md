# Spec Delta

## Purpose

The CS6 Actions panel: a dense action-set tree, Button mode, recording and
playback controls, the full panel menu, and the dialogs that edit actions and
action sets, presented identically when docked, in the iconic flyout, and in a
floating overlay.

## ADDED Requirements

### Requirement: Panel content and placement

The system SHALL provide `Window > Panels > Actions` (F9) as a plain `QWidget`
content panel with the stable object name `actionsPanel`, hosted by a
`PanelGroup` in a `PanelColumn` rather than by a `QDockWidget`. On a fresh
session the panel SHALL be in its own hidden group and SHALL NOT be visible in the
default Essentials workspace. The same widget SHALL be presented in the docked
group, the iconic flyout, and the floating overlay with no host-specific
behavior. The panel content SHALL fit the shared normal-mode minimum width.

#### Scenario: The panel exists as a content widget [ap_exists]

- **WHEN** the frame is shown
- **THEN** an `Actions` content widget with object name `actionsPanel` exists and is hosted by a `PanelGroup`

#### Scenario: The panel starts in its own hidden group [ap_default_hidden]

- **WHEN** a fresh session starts
- **THEN** the Actions group is registered, is not visible, and is reachable from `Window > Panels > Actions`

#### Scenario: Docked, flyout, and floating hosts match [ap_hosts]

- **WHEN** the Actions group is shown docked, opened from the iconic flyout, and floated
- **THEN** the tree, bottom toolbar, and panel menu are present and behave the same in all three hosts

#### Scenario: The panel fits the flyout width [ap_flyout_width]

- **WHEN** the panel is shown in the 300 px flyout
- **THEN** all columns, row controls, and bottom toolbar buttons are fully visible without horizontal scrolling

### Requirement: Tree content and state

The system SHALL present a three-level tree: action set, action, and recorded
step. The tree SHALL use three columns in this order: inclusion check, modal
control, and name. Set, action, and step rows SHALL show their expansion state
and a disclosure control. A step row SHALL show its name and whether it is a
recorded command or an inserted stop, menu-item, path, or conditional step;
expanding it SHALL show its recorded parameters when the step carries them. The
tree SHALL use `uniformRowHeights` and SHALL NOT expand every node by default. Expanded state SHALL be preserved across save and reload.

#### Scenario: Three levels and columns render [ap_tree_shape]

- **WHEN** the library contains a set, an action, and steps
- **THEN** the set, action, and steps render in that order with check, modal, and name columns

#### Scenario: Expansion follows stored state [ap_tree_expansion]

- **WHEN** a set, action, or step is expanded or collapsed
- **THEN** the new state is reflected immediately and is preserved when the panel is reopened

#### Scenario: Alt-click expands a subtree [ap_tree_alt_expand]

- **WHEN** Alt/Option is held while a set or action disclosure control is clicked
- **THEN** all actions in the set or all steps in the action expand or collapse together

### Requirement: Inclusion check state

The system SHALL show a check mark for an included step and no check mark for an
excluded step. Clearing a step's check SHALL exclude it from playback; restoring
it SHALL include it. Clicking an action's or set's check SHALL include or exclude
all of its children. Alt/Option-clicking a step's check SHALL exclude or include
all sibling steps except the clicked step. An action or set whose children are
partly excluded SHALL show a partial state and a red check mark. The partial and
parent states SHALL be derived from the children, not stored separately.

#### Scenario: Excluding a step skips it [ap_check_step]

- **WHEN** a step's check mark is cleared and its action is played
- **THEN** that step is skipped and the remaining steps run

#### Scenario: Parent check toggles all children [ap_check_parent]

- **WHEN** an action's or set's check mark is clicked
- **THEN** every step or action below it is included or excluded to match

#### Scenario: Alt-click excludes all but one [ap_check_alt]

- **WHEN** Alt/Option-clicking a step's check mark
- **THEN** every sibling step is excluded if the clicked step is included, or included if it is excluded

#### Scenario: Partial state is red [ap_check_partial]

- **WHEN** some but not all steps in an action or actions in a set are excluded
- **THEN** the parent shows a partial state and a red check mark

### Requirement: Modal controls

The system SHALL show a modal-control icon for a step whose dialog state is on
and no icon for a step whose dialog state is off. Dialog options SHALL determine
the control: 0 optional SHALL be editable and follow the with-dialog flag; 1
required SHALL show on and SHALL NOT be turned off; 2 none SHALL show off and
SHALL NOT be turned on; 3 specific (Insert Menu Item and similar) SHALL show off
and SHALL NOT be editable, but playback SHALL still open the command's dialog; 4
stop/continue is an in-memory recording value that is written as 0. An action or
set SHALL show modal when all of its children are modal, partial (red) when some
are modal, and off when none are, based on the displayed controls. Modal controls
SHALL NOT be editable in Button mode.

#### Scenario: Command modal toggles [ap_modal_toggle]

- **WHEN** a step with editable dialog options has its modal control clicked
- **THEN** its with-dialog state flips and playback follows the new state

#### Scenario: Required dialog cannot be disabled [ap_modal_required]

- **WHEN** a step was recorded with dialog options 1 (required) or 2 (none)
- **THEN** its modal control is shown on for required and off for none, and neither can be toggled

#### Scenario: Insert Menu Item stays specific [ap_modal_menu_item]

- **WHEN** an Insert Menu Item step references a command that opens a dialog
- **THEN** its modal control is shown off and cannot be enabled, while playback still opens the dialog

#### Scenario: Parent modal is derived [ap_modal_parent]

- **WHEN** an action has some modal and some non-modal steps
- **THEN** the action shows a partial modal state with a red dialog icon

### Requirement: Selection, renaming, and overwriting

The system SHALL allow a set, action, or step to be selected. Shift-click SHALL
select a contiguous range and Ctrl/Cmd-click SHALL add or remove a discontiguous
item. Double-clicking an action or set name SHALL begin an inline rename.
Double-clicking a step SHALL open the command's dialog prefilled with the
recorded values and SHALL replace those values on OK.

#### Scenario: Range and discontiguous selection [ap_select]

- **WHEN** the user Shift-clicks and Ctrl/Cmd-clicks rows
- **THEN** the selection follows the standard contiguous and discontiguous rules

#### Scenario: Inline rename [ap_rename]

- **WHEN** an action or set name is double-clicked and a new name is entered
- **THEN** the name changes in the library and persists across save and reload

#### Scenario: Overwrite a step [ap_overwrite]

- **WHEN** a step is double-clicked, its dialog is changed, and OK is pressed
- **THEN** the step's recorded values are replaced and playback uses the new values

### Requirement: Reordering, duplication, and deletion

The system SHALL allow a step to be dragged within its action or into another
action, an action to be dragged into another set or before/after another action,
and a set to be reordered. A plain drag SHALL move the item; Alt/Option-dragging, dropping an action or
step onto the Create New Action button, or dropping a set onto the Create New
Set button SHALL duplicate it. Delete SHALL
remove the selected items after confirmation; Alt/Option-clicking Delete or
dragging the selection to the Delete button SHALL delete without confirmation. A
set deleted while it is the last set SHALL leave the panel empty rather than
invalid.

#### Scenario: Reorder steps and actions [ap_reorder]

- **WHEN** a step is dragged to a new position or an action is dragged to another set
- **THEN** the new order and ownership are stored and persist

#### Scenario: Alt-drag duplicates [ap_duplicate]

- **WHEN** Alt/Option-dragging a set, action, or step to another position
- **THEN** a copy is inserted at the drop position and the source is unchanged

#### Scenario: Delete without confirmation [ap_delete_fast]

- **WHEN** Alt/Option-clicking Delete or dragging the selection to the Delete button
- **THEN** the selected items are removed without a confirmation dialog

### Requirement: Button mode

The system SHALL provide a checkable Button Mode command. When on, the panel
SHALL render actions as buttons with their assigned color and name and SHALL NOT
show sets, set boundaries, or individual steps. Buttons SHALL follow library
order: sets in order, then actions within each set in order. Clicking a button
SHALL play the whole action, skipping excluded steps. Modal controls and
inclusion checks SHALL NOT be editable in Button mode. Turning Button mode off
SHALL restore the tree with all hidden state unchanged.

#### Scenario: Buttons replace the tree [ap_button_mode]

- **WHEN** Button Mode is enabled
- **THEN** actions render as colored buttons in library order and sets, set boundaries, and steps are not shown

#### Scenario: Button plays the whole action [ap_button_play]

- **WHEN** an action button is clicked
- **THEN** all included steps play and excluded steps are skipped

#### Scenario: Hidden state survives the toggle [ap_button_restore]

- **WHEN** Button Mode is turned off
- **THEN** the tree returns with the same expansion, selection, check, and modal states

### Requirement: Panel menu

The system SHALL provide the Actions panel menu with these rows and separators
in order: Button Mode (checkable); separator; New Action…, New Set…, Duplicate,
Delete, Play; separator; Start Recording (Stop Recording while recording),
Record Again…; separator; Insert Menu Item…, Insert Stop…, Insert Conditional…,
Insert Path; separator; a context-sensitive `Action Options…`/`Set Options…`
row, Playback Options…, Allow Tool Recording (checkable); separator; Clear All
Actions, Reset Actions, Load Actions…, Replace Actions…, Save Actions…; separator;
the installed action sets; separator; the group-provided close rows. Button Mode,
New Set, Playback Options, Allow Tool Recording, Load,
Replace, Reset, and the close rows SHALL be enabled while idle. New Action SHALL
need a selected set or a non-empty library; Duplicate and Delete SHALL need any
selection; Play SHALL need an action, step, or set selection; Start Recording
SHALL need an action, step, or non-empty set; Record Again SHALL need an action; Clear All SHALL need a non-empty library; Load, Replace, Reset, and Save SHALL be disabled while
recording or playing. Insert Menu Item, Insert Stop, and Insert Conditional SHALL
need an action or step; Insert Path SHALL also need a selected path. Action
Options SHALL need an action; Set Options SHALL need a set; Save Actions SHALL
need a set. Editing rows SHALL be disabled while recording or playing, except
Stop Recording; Play SHALL be disabled while recording and Start Recording SHALL
be disabled while playing. The installed-set rows SHALL be the `.atn` files discovered in the
application's `Presets/Actions` directory. The application SHALL ship no Adobe
preset action there, and the compiled-in default set SHALL NOT be discovered as
an installed set.

#### Scenario: Menu order matches the reference [ap_menu_order]

- **WHEN** the Actions panel menu is opened
- **THEN** the rows and separators appear in the documented order

#### Scenario: Menu enablement follows selection [ap_menu_enable]

- **WHEN** nothing is selected, then an action, then a set, then a step is selected
- **THEN** Duplicate/Delete/Play/Start Recording/Record Again/insert rows/options/Save Actions enable and disable as documented

#### Scenario: Checkable rows reflect state [ap_menu_checks]

- **WHEN** Button Mode or Allow Tool Recording is on
- **THEN** the corresponding menu row is checked, and toggling it updates the panel

#### Scenario: Installed sets load [ap_menu_sets]

- **WHEN** the presets directory contains an `.atn` file
- **THEN** its set name appears in the installed-set section and choosing it loads that set into the library

#### Scenario: Group close rows are not duplicated [ap_menu_close]

- **WHEN** the Actions panel menu is opened
- **THEN** exactly one Close and one group-close row appear, appended by the panel group

### Requirement: Bottom toolbar

The system SHALL provide an icon-only bottom toolbar in this order: Stop, Record,
Play, Create New Set, Create New Action, Delete. Each button SHALL have a tooltip
and SHALL be enabled only when its operation applies. The Record button SHALL
show the recording state. During playback the record and edit buttons SHALL be
disabled and the Stop button SHALL be enabled.

#### Scenario: Toolbar order and tooltips [ap_toolbar_order]

- **WHEN** the panel is shown
- **THEN** the six buttons appear in the documented order with tooltips

#### Scenario: Recording state is visible [ap_toolbar_record]

- **WHEN** recording starts or stops
- **THEN** the Record button reflects the state and Stop is available

#### Scenario: Controls lock during playback [ap_toolbar_playback]

- **WHEN** an action is playing
- **THEN** record/edit controls are disabled and Stop is enabled

### Requirement: Action and set dialogs

The system SHALL provide dialogs for New Action, New Set, Action Options, Set
Options, and Playback Options. New Action SHALL edit name, target set, optional
function key, Shift and Ctrl/Command modifiers, button color, and a Record
confirmation. Action Options SHALL edit name, target set, optional function key,
the same modifiers, and button color. Set Options SHALL edit the set name.
Playback Options SHALL offer Accelerated, Step By Step, and Pause For N Seconds
with a non-negative duration and SHALL default to Accelerated with zero pause.
Validation SHALL refuse an empty name where a name is required and SHALL refuse
an invalid function key.

#### Scenario: New Action creates and records [ap_dialog_new_action]

- **WHEN** New Action is confirmed with a name, set, optional key, and color
- **THEN** the action is inserted into the chosen set and recording begins

#### Scenario: Playback Options persist [ap_dialog_playback]

- **WHEN** Playback Options is changed to Step By Step or Pause and confirmed
- **THEN** subsequent playback uses the new speed and the setting survives the session

#### Scenario: Action Options moves and renames [ap_dialog_action_options]

- **WHEN** Action Options is confirmed with a new name, a different target set, and a new key
- **THEN** the action moves to that set and the new name and key persist

#### Scenario: Set Options renames a set [ap_dialog_set_options]

- **WHEN** Set Options is confirmed with a new name
- **THEN** the set name changes in the panel and persists

### Requirement: Stop, menu-item, path, and conditional steps

The system SHALL provide Insert Stop with a message and an Allow Continue flag,
Insert Menu Item with a command picker, Insert Path using the selected path, and
Insert Conditional (a Kooka Pictura extension) with a condition, a Then action,
and an Else action. Insert Stop SHALL insert a stop step at the insertion point.
Insert Menu Item SHALL insert the chosen command without recorded values and
SHALL mark its modal control as specific (off and not editable, dialog still
opens). Insert Path SHALL insert the selected path as an app-native step. Insert
Conditional SHALL require at least one non-empty branch and SHALL require both
referenced actions to belong to the action's own set.

#### Scenario: Stop message and allow-continue [ap_insert_stop]

- **WHEN** Insert Stop is confirmed with a message and Allow Continue
- **THEN** a stop step is inserted and playback shows the message with Continue

#### Scenario: Menu item inserts without values [ap_insert_menu]

- **WHEN** Insert Menu Item is confirmed with a command
- **THEN** a menu-item step is inserted with its modal control off and not editable, and playback runs that command with its dialog

#### Scenario: Conditional validation [ap_insert_conditional]

- **WHEN** both Then and Else are None or reference actions outside the action's set
- **THEN** the dialog refuses the step

### Requirement: Recording controls

The system SHALL provide Start Recording, Stop Recording, Record Again, and
Allow Tool Recording. Starting recording SHALL begin at the selected action or
step insertion point, or append to the last action of a selected non-empty set;
an empty or absent selection SHALL be refused. Stopping SHALL retain the recorded
steps, and Escape SHALL also stop recording. Record Again SHALL step through an
existing action's modal steps so their values can be re-captured. Allow Tool
Recording SHALL be a checkable panel option and SHALL be off by default. While
recording, the Record button and the panel state SHALL show that recording is
active. Recording and playback SHALL be mutually exclusive.

#### Scenario: Record Again updates modal values [ap_record_again]

- **WHEN** Record Again is used on an action and a modal step's dialog is confirmed with new values
- **THEN** only that step's recorded values are replaced

#### Scenario: Allow Tool Recording gates tool steps [ap_allow_tool]

- **WHEN** Allow Tool Recording is on and a tool stroke completes while recording
- **THEN** one tool step is appended; when the option is off no tool step is appended

#### Scenario: Escape stops recording [ap_record_escape]

- **WHEN** Escape is pressed while recording
- **THEN** recording stops and the steps already recorded are retained

### Requirement: Playback controls and modal pauses

The system SHALL play the selected action, a selected set's included actions in
order (an explicit app decision), a step to the end of its action, or a single
step. Ctrl/Cmd-clicking Play or Ctrl/Cmd-double-clicking a step SHALL play only
that step. A function key assigned to an action SHALL play it. Playback SHALL
show the current step, SHALL honor the playback speed, SHALL pause for modal
steps, SHALL open a specific-dialog step's dialog, SHALL show Stop messages, and
SHALL offer cancellation. Only one playback SHALL run at a time, the panel
controls SHALL prevent a second start while one is running, and recording SHALL
be refused during playback.

#### Scenario: Play from a step [ap_play_from_step]

- **WHEN** a step is selected and Play is used
- **THEN** playback starts at that step and runs to the end of its action

#### Scenario: Single-step playback [ap_play_single]

- **WHEN** Ctrl/Cmd-clicking Play with one step selected
- **THEN** only that step plays

#### Scenario: Function-key playback [ap_play_key]

- **WHEN** an action's assigned function key is pressed
- **THEN** that action plays

#### Scenario: One playback at a time [ap_play_concurrent]

- **WHEN** playback is running and Play is activated again
- **THEN** the second start is refused and the running playback continues

### Requirement: Saving, loading, and resetting sets

The system SHALL save only a whole set to a `.atn` file, load an additional set,
replace all sets, reset to the default set with OK replacing and Append adding,
clear all sets, and load/save the library to the application's palette state.
Save Actions SHALL be enabled only when a set is selected. Replace SHALL
confirm destructive replacement; Load appends without a confirmation. The
palette write SHALL be serialized and
atomic, SHALL happen on a debounced change and at shutdown, and SHALL leave the
previous palette intact when a write fails. A text listing written with
Ctrl+Alt/Cmd+Option at Save time SHALL be human-readable and SHALL NOT be
reloadable; its exact Adobe layout is undocumented, so the listing is a Kooka
format and is not claimed as Adobe-compatible.

#### Scenario: Save and load a set [ap_save_load]

- **WHEN** a set is saved to `.atn` and loaded again
- **THEN** the set appears with its actions and steps intact

#### Scenario: Replace warns and replaces [ap_replace]

- **WHEN** Replace Actions is confirmed
- **THEN** every current set is replaced by the loaded set

#### Scenario: Text listing is not reloadable [ap_text_dump]

- **WHEN** Save Actions is used with Ctrl+Alt/Cmd+Option
- **THEN** a text listing is written and attempting to load it reports that it is not an action set

### Requirement: Panel state persistence

The system SHALL persist Button mode, expanded set, action, and step identities,
the current selection, Allow Tool Recording, and Playback Options in the session
store, and SHALL persist the action library in the application's palette state.
Expansion and selection SHALL be stored by set/action name path, the ordinal
among same-named siblings, and step index, and SHALL ignore a missing name. The
keys SHALL be additive under the existing session schema 10: missing keys SHALL load defaults and unknown keys SHALL be
preserved. The panel SHALL restore those values on the next launch. The library
SHALL be loaded at launch and written atomically on change or shutdown without
losing a successfully parsed set.

#### Scenario: Panel state restores [ap_state_restore]

- **WHEN** Button mode, expansion, Allow Tool Recording, and Playback Options are changed and the application restarts
- **THEN** the panel restores those values

#### Scenario: Library restores [ap_library_restore]

- **WHEN** sets are created or loaded and the application restarts
- **THEN** the library is restored from the palette state

### Requirement: Function-key assignment

The system SHALL assign a function key F1–F12 with optional Shift and
Ctrl/Command from the New Action and Action Options dialogs, SHALL display it in
the panel and in Button mode, and SHALL refuse a combination already owned by
another action. On Windows it SHALL refuse F1 and Ctrl+F4/Ctrl+F6; the check
SHALL use an injectable platform seam. The panel SHALL warn, not block, when the
combination shadows a command shortcut. Clearing the key or removing the action
SHALL restore the command's shortcut. The keyboard-shortcuts editor SHALL NOT be
required to list action keys.

#### Scenario: Assign and display a key [ap_key_assign]

- **WHEN** an action is given F5 with Shift
- **THEN** the panel shows Shift+F5 and pressing it plays the action

#### Scenario: Shadowed command is warned [ap_key_warn]

- **WHEN** an action is assigned a combination already used by a menu command
- **THEN** the dialog warns that the action will take precedence

#### Scenario: Windows restrictions [ap_key_windows]

- **WHEN** the application runs on Windows
- **THEN** F1, Ctrl+F4, and Ctrl+F6 are refused as action function keys

### Requirement: Error and edge behavior

The system SHALL report `.atn`/`.psp` parse failures without crashing, load the
sets that parsed, and leave the existing library unchanged when nothing parsed.
It SHALL report a missing command target, an unsupported event, a wrong-mode
document, and a failing step with a named diagnostic and SHALL stop playback on
the first error; this release has no continue-after-error path. It SHALL allow an
empty action or set to play as a no-op success. It SHALL virtualize the tree for
large sets and SHALL NOT expand every node by default. The panel SHALL be
unavailable without a GUI, and the engine SHALL refuse concurrent playback and
recording during playback.

#### Scenario: Parse failure reports and keeps parsed sets [ap_error_parse]

- **WHEN** an action file contains a valid prefix and a malformed suffix
- **THEN** the parsed sets are loaded and a named parse error is reported

#### Scenario: Unsupported target stops with a name [ap_error_target]

- **WHEN** a step references a command or action that is not available
- **THEN** playback stops and names the step and the missing target

#### Scenario: Empty action is a no-op [ap_error_empty]

- **WHEN** an empty action is played
- **THEN** no command runs, no error is shown, and no history state is created

### Requirement: Accessibility

The system SHALL expose inclusion and modal states through standard item roles
so assistive technology can read them, and SHALL keep keyboard navigation for
the tree and Button mode. Space SHALL toggle the focused row's inclusion state
where editable; Ctrl/Cmd+Space SHALL toggle its modal state where editable.
Focusing a row SHALL announce both states when they apply.

#### Scenario: Screen reader reads states [ap_a11y_state]

- **WHEN** a screen reader focuses a row
- **THEN** its included/partial and modal/partial states are available through standard roles

#### Scenario: Keyboard toggles state [ap_a11y_key]

- **WHEN** a row is focused and Space or Ctrl/Cmd+Space is pressed
- **THEN** Space toggles inclusion and Ctrl/Cmd+Space toggles modal when each is editable
