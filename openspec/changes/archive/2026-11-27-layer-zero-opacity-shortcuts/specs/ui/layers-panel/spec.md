## ADDED Requirements

### Requirement: Numeric layer opacity keyboard gestures

The Layers tree and Move tool canvas SHALL interpret one digit as its tens
percentage (1–9 give 10–90%, and 0 gives 100%) after a 500 ms window.
Two digits within that window SHALL set the exact percentage from 00 to 99.
The first digit SHALL leave the layer and history unchanged while waiting.
Unmodified digits set Opacity, and Shift on both presses sets Fill. The layer
gesture SHALL accept number-row and keypad digits, including native shifted
number-row events whose resulting key differs from the digit. Ctrl, Alt and
Meta combinations and registered shortcuts SHALL keep their normal routing.

Both presses SHALL share the focus owner, document, selection and Shift state.
Focus, document, selection, tool and pointer context changes, other non-modifier
keys, and registered shortcuts SHALL invalidate an unresolved press. Shift key
presses and releases alone SHALL NOT interrupt it; a digit with a different
Shift state starts its own gesture and discards a still-pending prefix. A digit
in a new eligible context starts its own gesture. Auto-repeat SHALL
neither add a digit nor extend the window. The layer gesture SHALL leave text
entry, painting canvas input, and pending transforms to their normal handlers.
It SHALL use the Layers header's eligibility rules and record one undo step
when a resolved gesture changes one or more layers.

#### Scenario: Set opacity from the canvas [zero_opacity_canvas]
- **WHEN** the Move tool canvas has focus and the user taps 0 twice in quick succession
- **THEN** the selected eligible layers' Opacity reads 0%, their Fill is preserved, and one Undo restores the prior values

#### Scenario: Set fill from the Layers tree [zero_fill_tree]
- **WHEN** the Layers tree has focus and the user presses Shift+0 twice in quick succession
- **THEN** eligible selected layers have Fill set to 0% in one undo step, and their Opacity is preserved

#### Scenario: Apply the header's eligibility rules [zero_eligibility]
- **WHEN** the user invokes either gesture with an ineligible layer selected, or repeats a gesture on an already-zero value
- **THEN** no history entry is added for a no-op, and a group accepts Opacity but refuses Fill

#### Scenario: A single digit resolves after the window [numeric_single]
- **WHEN** the user presses a lone digit with either supported modifier state and keeps the same context
- **THEN** at timeout the matching property changes to its single-digit percentage in one undo step, with no earlier change to the layer or history

#### Scenario: Mixed and repeated digit pairs [numeric_pair]
- **WHEN** the user taps pairs such as 05, 54, and 99 quickly in an eligible context
- **THEN** the matching property reaches the exact percentage in one undo step, without applying the first digit's standalone value

#### Scenario: Auto-repeat, timing and input interruptions [zero_ignore]
- **WHEN** a digit is held, digits are split by the timeout, a second digit has a different Shift state, or a non-digit interrupts input
- **THEN** auto-repeat adds no digit, digits separated by the timeout resolve independently, and a still-pending interrupted digit is discarded; after a Shift-state change only the second digit starts a new gesture

#### Scenario: Context changes invalidate the pending press [zero_context]
- **WHEN** focus, document, selection, tool or pointer context changes while a digit is pending, even if the original context is restored
- **THEN** the original digit neither resolves after the timeout nor combines with another digit

#### Scenario: A registered numeric shortcut takes precedence [numeric_registered]
- **WHEN** a registered shortcut consumes an otherwise eligible digit between two layer digits, including when its action enters a nested event loop across the pending deadline
- **THEN** the registered shortcut runs, the first pending digit is discarded without resolving, and the later digit starts a fresh layer gesture

#### Scenario: Painting canvas input retains precedence [numeric_paint]
- **WHEN** the user enters digits with a painting tool active and the canvas focused
- **THEN** the layer shortcut makes no property or history edit

#### Scenario: Modified commands retain their shortcuts [zero_modifiers]
- **WHEN** Ctrl, Alt or Meta accompanies a zero press, including Ctrl+0 and Ctrl+Alt+0
- **THEN** the press does not start or complete a layer gesture and existing commands retain their shortcut behavior

#### Scenario: A pending transform retains input [zero_transform]
- **WHEN** a transform is pending and the user presses either zero gesture on the Move canvas or Layers tree
- **THEN** no layer property or history is changed by the gesture and transform input remains available

#### Scenario: Editors keep their input [zero_editor]
- **WHEN** the user types two zeros in a focused Layers text input
- **THEN** the text input receives the zeros and no layer opacity or fill gesture fires
