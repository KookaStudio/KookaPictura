## ADDED Requirements

### Requirement: Zero layer opacity keyboard gestures

The Layers tree and Move tool canvas SHALL recognize two zero presses within
500 ms from the same focus, document and selection. A change to any of those
contexts SHALL invalidate the pending press even if the original context returns.
With no modifiers the gesture SHALL set layer opacity to 0%; with Shift on both
presses it SHALL set fill opacity to 0%. The keypad modifier SHALL be allowed,
but Ctrl, Alt and Meta combinations SHALL remain available to their existing
commands. The number-row zero key SHALL be recognized across native keyboard
layouts even when Shift changes its resulting key or text; keypad zero SHALL
also be accepted. The gesture SHALL NOT run while a transform is pending. It
SHALL use the Layers header's eligibility and selection rules and record one
undo step when a value changes.

#### Scenario: Set opacity from the canvas [zero_opacity_canvas]
- **WHEN** the Move tool canvas has focus and the user taps 0 twice in quick succession
- **THEN** the selected eligible layers' Opacity reads 0%, their Fill is preserved, and one Undo restores the prior values

#### Scenario: Set fill from the Layers tree [zero_fill_tree]
- **WHEN** the Layers tree has focus and the user presses Shift+0 twice in quick succession
- **THEN** eligible selected layers have Fill set to 0% in one undo step, and their Opacity is preserved

#### Scenario: Apply the header's eligibility rules [zero_eligibility]
- **WHEN** the user invokes either gesture with an ineligible layer selected, or repeats a gesture on an already-zero value
- **THEN** no history entry is added for a no-op, and a group accepts Opacity but refuses Fill

#### Scenario: Incomplete and interrupted sequences [zero_ignore]
- **WHEN** a zero is pressed once, repeated by holding the key, separated from the next zero by more than 500 ms, interrupted by another non-modifier key, or followed by a zero with different Shift state or in a different focus context
- **THEN** no layer property is edited by that sequence

#### Scenario: Context changes invalidate the pending press [zero_context]
- **WHEN** focus, document or layer selection changes after the first zero, even if the original context is restored before another zero
- **THEN** the later zero does not complete the earlier gesture and no layer property or history is changed

#### Scenario: Modified commands retain their shortcuts [zero_modifiers]
- **WHEN** Ctrl, Alt or Meta accompanies a zero press, including Ctrl+0 and Ctrl+Alt+0
- **THEN** the press does not start or complete a layer gesture and existing commands retain their shortcut behavior

#### Scenario: A pending transform retains input [zero_transform]
- **WHEN** a transform is pending and the user presses either zero gesture on the Move canvas or Layers tree
- **THEN** no layer property or history is changed by the gesture and transform input remains available

#### Scenario: Editors keep their input [zero_editor]
- **WHEN** the user types two zeros in a focused Layers text input
- **THEN** the text input receives the zeros and no layer opacity or fill gesture fires
