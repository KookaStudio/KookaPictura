# Spec Delta

## Purpose

A serializable, recordable action library that plays command steps through the
same command layer as the UI, so the Actions panel, future droplets, batch
processing, and scripting share one execution path.

## ADDED Requirements

### Requirement: Action library model

The system SHALL represent an action library as ordered action sets; each set
SHALL contain ordered actions, and each action SHALL contain ordered steps. A set
SHALL carry a Unicode name and an expanded state. An action SHALL carry a Unicode
name, an optional function key (F1–F12, with 0 meaning none) with its Shift and
Ctrl/Command modifiers, a button color, and an expanded state. Imported
function-key values outside F1–F12 SHALL be preserved and SHALL NOT be offered by
the assignment UI. A step SHALL carry
an expanded flag, an enabled flag, a with-dialog flag, a recorded dialog-options
value (0–4), and a display name. Step order SHALL be significant and SHALL be
preserved across save and load. Dialog options SHALL be interpreted as 0
optional (the with-dialog flag is user-editable), 1 required (always on), 2 none
(always off), 3 specific (shown off and not editable, but playback opens the
command dialog), and 4 stop/continue (written as 0).

#### Scenario: Nested structure round-trips [ae_model_roundtrip]

- **WHEN** a set containing actions and steps is saved and loaded again
- **THEN** the set, action, and step order, names, flags, function keys, and colors are unchanged

#### Scenario: Empty structures are valid [ae_model_empty]

- **WHEN** a set or action has no children
- **THEN** it loads, saves, and plays as a no-op success without creating a history state

### Requirement: Library load failure

A failed library load SHALL leave the in-memory library unchanged and SHALL
report a named error with the source and the failing offset when available.

#### Scenario: Failed load keeps the library [ae_load_failure_keeps_library]

- **WHEN** loading a malformed action file fails
- **THEN** the in-memory library is unchanged and the error names the source and offset

### Requirement: Records only recordable commands

While recording is active, the system SHALL append every recordable executed
command as a step containing the command identity and the parameter values in
effect at execution time. A parameterless command SHALL append a step with no
parameters. A command that is not recordable SHALL NOT be appended. The system
SHALL classify every implemented catalogue command that is recordable and every
recordable Brush/Pencil tool stroke as Adobe-verified, app-native, or
non-recordable, and a coverage check SHALL fail when any such executable has no
classification. Panel operations and native tool-specific strokes are outside
this release's recording surface. A handler SHALL report success before its step
is recorded; a refused or failed command SHALL NOT be appended.

#### Scenario: Dialog values are recorded [ae_record_dialog]

- **WHEN** a recorded dialog command is confirmed with changed values
- **THEN** one step is appended with those values, and playback can reuse them

#### Scenario: Parameterless command records an empty step [ae_record_empty]

- **WHEN** a recordable parameterless command executes while recording
- **THEN** one step with no parameters is appended

#### Scenario: Non-recordable command is never recorded [ae_record_excluded]

- **WHEN** a command classified non-recordable executes while recording
- **THEN** no step is appended and the recording continues

#### Scenario: Unclassified command fails coverage [ae_record_coverage]

- **WHEN** an implemented command has no recordable/app-native/non-recordable classification
- **THEN** the coverage check fails and names the command

### Requirement: Recording lifecycle

Recording SHALL begin at the selected action or step insertion point. With a
non-empty set selected, recording SHALL append to the last action in that set;
with an empty set or no selection, recording SHALL be refused with a named
diagnostic rather than invent a target. Recording SHALL be stoppable without
discarding already-recorded steps, and SHALL be resumable from the same
insertion point. The system SHALL expose the recording state to the panel. A
command that is refused or that fails SHALL NOT append a step. Recording and
playback SHALL be mutually exclusive.

#### Scenario: Start, stop, and resume [ae_record_lifecycle]

- **WHEN** recording is started, a step is recorded, recording is stopped, and recording is started again
- **THEN** the first step is retained and the next step is appended after it

#### Scenario: Refused command is not captured [ae_record_refused]

- **WHEN** a recordable command is refused by the engine while recording
- **THEN** no step is appended and the document and action are unchanged

### Requirement: Playback semantics

Playback SHALL execute enabled steps in order from the selected action, from a
selected step to the end, or as a single selected step. A selected set SHALL
play its included actions in order; this whole-set behavior is an explicit app
decision because CS6 Help does not document it. Disabled steps SHALL be skipped.
Dialog options 0 SHALL use the with-dialog flag (on pauses, off uses recorded
values); 1 SHALL always pause; 2 SHALL never pause; 3 SHALL open the command
dialog even though its modal control is off; 4 SHALL behave as 0. Playback SHALL
honor Accelerated, Step By Step, and Pause N Seconds speeds. A Stop step SHALL
show its message and, when allow-continue is set, SHALL offer Continue. An Insert
Menu Item step SHALL execute the referenced command and SHALL pause if that
command opens a dialog. An Insert Path step SHALL set the document's work path and
SHALL be stored as an app-native step. A Play step SHALL play the referenced
action, and recursion SHALL be limited to 64 nested Play/Conditional invocations.
A Conditional step SHALL evaluate its condition and play exactly one branch. Cancellation SHALL stop after the
current step and leave completed steps applied. An error SHALL stop playback and
report the failing step; this release does not continue after an error. Only one
playback SHALL run at a time.

#### Scenario: Playback order and exclusion [ae_play_order]

- **WHEN** an action contains three enabled steps and one disabled step between them
- **THEN** the three enabled steps execute in order and the disabled step is skipped

#### Scenario: A selected set plays in order [ap_play_set]

- **WHEN** a set is selected and Play is used
- **THEN** its included actions play in library order

#### Scenario: Modal pause uses entered values [ae_play_modal]

- **WHEN** a step has dialog state on and the user confirms changed values
- **THEN** playback pauses at that step and the command uses the entered values

#### Scenario: Dialog-off uses recorded values [ae_play_recorded]

- **WHEN** a step has dialog state off
- **THEN** the command executes with its recorded values without opening UI

#### Scenario: Specific dialog options still open the dialog [ae_play_specific]

- **WHEN** a step has dialog options 3 (specific) and its modal control is shown off
- **THEN** playback still opens the command's dialog and waits for input

#### Scenario: Step By Step redraws between steps [ae_play_stepwise]

- **WHEN** playback speed is Step By Step
- **THEN** the document is redrawn after each step before the next step starts

#### Scenario: Stop step honours Allow Continue [ae_play_stop]

- **WHEN** playback reaches a Stop step whose allow-continue flag is set
- **THEN** the message is shown with a Continue control, and playback resumes when it is used

#### Scenario: Insert Menu Item executes its command [ae_play_menu_item]

- **WHEN** playback reaches an Insert Menu Item step
- **THEN** the referenced menu command executes and the action pauses if its dialog opens

#### Scenario: Insert Path sets the work path [ae_play_path]

- **WHEN** playback reaches an Insert Path step
- **THEN** the document's work path becomes the recorded path

#### Scenario: Play recursion is bounded [ae_play_recursion]

- **WHEN** action A plays action B and action B plays action A
- **THEN** playback aborts within the documented depth limit and reports a recursion error

#### Scenario: Cancellation keeps completed steps [ae_play_cancel]

- **WHEN** playback is cancelled after two successful steps
- **THEN** those two steps remain applied and their history states remain

#### Scenario: A failing step stops playback [ae_play_error]

- **WHEN** a step fails because its target or document mode is invalid
- **THEN** playback stops, reports the failing step, and does not execute later steps, with no continue-on-error path

### Requirement: Unsupported steps are named

The system SHALL preserve an event whose command identity is not supported and
SHALL report a named diagnostic if playback reaches it. It SHALL NOT silently
skip the step, silently substitute a different command, or claim the event
played.

#### Scenario: Unsupported event reports at playback [ae_unsupported_play]

- **WHEN** playback reaches an event whose command identity is not supported
- **THEN** playback stops with a diagnostic naming the event and the step index

### Requirement: Function keys

An action may own a function key F1–F12 with optional Shift and Ctrl/Command
modifiers. At most one action SHALL own a given combination. While an action owns
a combination, pressing it SHALL play that action and SHALL take precedence over a
command with the same shortcut. Clearing the key or deleting the action SHALL
restore the command shortcut. On Windows the system SHALL refuse F1 and
Ctrl+F4/Ctrl+F6; the platform check SHALL be injectable for tests. The
keyboard-shortcuts editor is not required to list action keys; a shadowed
command keeps its stored binding.

#### Scenario: Action beats a command shortcut [ae_key_precedence]

- **WHEN** an action and a menu command share a function-key combination
- **THEN** pressing the combination plays the action and does not run the command

#### Scenario: Duplicate action keys are refused [ae_key_unique]

- **WHEN** the user assigns a combination already owned by another action
- **THEN** the assignment is refused and the existing owner keeps the key

#### Scenario: Removing the action restores the command [ae_key_restore]

- **WHEN** an action that shadowed a command shortcut is deleted
- **THEN** the command's shortcut works again

### Requirement: Default library and reset

The system SHALL ship one project-authored default set named `Default Actions`
containing Duplicate Layer, Flatten Image, Grayscale, and Resize to 1000 px, with
a distinct button color per action. Resize to 1000 px SHALL record Image Size
with width 1000 px, constrain-proportions on, and Bicubic resampling. The default
set SHALL contain no Adobe preset action and SHALL NOT be discovered as an
installed `.atn` set. Reset Actions SHALL replace the library with the default
set on OK and append it on Append; Append SHALL use a unique name
(`Default Actions 2`, …) when the default set already exists. Clear All Actions
SHALL empty the library.

#### Scenario: Reset replaces the library [ae_reset_replace]

- **WHEN** Reset Actions is confirmed with OK
- **THEN** the library contains exactly the default set

#### Scenario: Reset appends the default set [ae_reset_append]

- **WHEN** Reset Actions is confirmed with Append
- **THEN** the existing library is kept and the default set is appended

#### Scenario: Clear All empties the library [ae_clear_all]

- **WHEN** Clear All Actions is confirmed
- **THEN** the library has no sets and the panel shows its empty state

#### Scenario: Default set is project-authored [ae_default_provenance]

- **WHEN** the default set is inspected
- **THEN** it contains only the four documented actions and no Adobe preset name

### Requirement: Tool-stroke recording extension

When Allow Tool Recording is on, the system SHALL record a completed Brush or
Pencil stroke as one step containing the tool identity, its settings at stroke
start, and the ordered input samples. Playback SHALL replay that stroke through
the same tool engine. Native tool-specific strokes and mask strokes are outside
this requirement and SHALL be refused with a named diagnostic until their replay
paths land. The step SHALL be app-native and SHALL NOT be represented as an
Adobe-compatible `.atn` step. The sample count and payload size SHALL be bounded;
a stroke over the bound SHALL be refused with a named diagnostic and SHALL NOT
corrupt the action. The app-native payload SHALL carry a format version.

#### Scenario: Brush stroke replays [ae_tool_replay]

- **WHEN** a brush stroke is recorded with Allow Tool Recording on and the action is played
- **THEN** the same stroke geometry is painted and one tool history state is recorded

#### Scenario: Tool settings are captured [ae_tool_settings]

- **WHEN** a Brush stroke is recorded with non-default tool settings
- **THEN** playback uses the settings captured at stroke start, not the current ones

#### Scenario: Tool recording is off by default [ae_tool_off]

- **WHEN** an action is recorded with Allow Tool Recording off
- **THEN** tool strokes are not appended as steps

#### Scenario: An oversized stroke is refused [ae_tool_limit]

- **WHEN** a recorded stroke exceeds the sample or payload bound
- **THEN** the step is refused with a named diagnostic and the action is unchanged

### Requirement: Conditional steps extension

The system SHALL support Insert Conditional as a Kooka Pictura extension with a
document condition, a Then action, and an Else action. The first release SHALL
support the conditions document is square, document mode is a selected mode, and
document has a selection; the condition set is an extension and is listed in the
panel dialog. Both branches SHALL name actions in the same set as the action
containing the conditional; at least one branch SHALL be non-empty, and both
empty SHALL be refused at insert and playback. Evaluation SHALL use the active
document; with no active document playback SHALL stop with a named diagnostic.
Conditional and Play chains SHALL share the 64-invocation depth limit. Moving
or deleting an action that is referenced by a conditional SHALL be refused when
it would leave the conditional's same-set invariant unsatisfied or leave a
dangling reference.

#### Scenario: The matching branch plays [ae_conditional_branch]

- **WHEN** a conditional step's condition is true for the active document
- **THEN** only the Then action plays; when it is false only the Else action plays

#### Scenario: Both branches empty is refused [ae_conditional_invalid]

- **WHEN** both Then and Else are None
- **THEN** the dialog refuses the step and no conditional step is inserted

#### Scenario: A stored both-empty conditional fails playback [ae_conditional_invalid_play]

- **WHEN** a loaded conditional step has both Then and Else None
- **THEN** playback stops with a named error and no branch plays

#### Scenario: Actions must share the set [ae_conditional_same_set]

- **WHEN** a Then or Else action belongs to a different set than the action containing the conditional
- **THEN** the dialog refuses the selection

#### Scenario: A referenced action cannot leave the set [ae_conditional_move_guard]

- **WHEN** moving or deleting an action referenced by a conditional would break the same-set invariant or leave a dangling reference
- **THEN** the library refuses the operation and the conditional remains valid

#### Scenario: Conditional needs an active document [ae_conditional_no_document]

- **WHEN** a conditional step is played with no active document
- **THEN** playback stops with a named diagnostic and no branch plays

#### Scenario: Conditional recursion is bounded [ae_conditional_recursion]

- **WHEN** conditional actions form a cycle
- **THEN** playback aborts within the documented depth limit and reports an error

### Requirement: Playback history

Each command played by an action SHALL create its normal document history state.
A played action SHALL NOT be presented as one undo state by default, matching
Photoshop CS6. Cancelling playback SHALL leave completed commands and their
history states intact.

#### Scenario: Per-command history [ae_history_per_command]

- **WHEN** an action with two mutating steps plays successfully
- **THEN** the document history contains two states, one per step

#### Scenario: No single action undo [ae_history_no_group]

- **WHEN** an action with multiple steps finishes
- **THEN** no single history state represents the whole action by default
