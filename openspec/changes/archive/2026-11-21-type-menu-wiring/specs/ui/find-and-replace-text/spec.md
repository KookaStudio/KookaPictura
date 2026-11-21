# Spec Delta

## Purpose

Find and replace text in a document's type layers, with a scope and matching options, recording the result as a single undoable state.

## ADDED Requirements

### Requirement: The Find And Replace dialog

`Edit > Find And Replace Text…` SHALL open a window-modal dialog with `Find What`, `Replace With`, `Search All Layers`, `Forward`, `Case Sensitive`, `Whole Word Only`, `Replace All`, and `Close`. The command SHALL be enabled when the document has at least one type layer and its type layers can be re-set (8-bit RGB), and disabled otherwise. With `Search All Layers` off, Replace All SHALL require the current layer to be type.

#### Scenario: The dialog opens on a type document [fr_dialog]

- **WHEN** `Edit > Find And Replace Text…` is triggered on a document with a type layer
- **THEN** the dialog shows the two text fields, the four matching options, and the Replace All and Close buttons; with no type layer the command is disabled

#### Scenario: Non-editable and non-type scopes are disabled [fr_not_editable]

- **WHEN** the document's type layers cannot be re-set, or the current layer is not type with Search All Layers off
- **THEN** Replace All is disabled and records no state

#### Scenario: A live Type session gates the command [fr_session_gate]

- **WHEN** a Type tool session is open
- **THEN** `Edit > Find And Replace Text…` is disabled and records no state until the session commits or cancels

#### Scenario: The command is declared modal [fr_modal_metadata]

- **WHEN** a client asks the control server to dispatch `edit.find.and.replace.text`
- **THEN** the command is refused as modal and its handler does not run

### Requirement: Replace All replaces in the chosen scope

`Replace All` SHALL replace every match in the current type layer, or in every type layer that is not fully locked when `Search All Layers` is checked, and SHALL record exactly one `Find And Replace Text` state. Only type layers SHALL be searched and changed. `Replace All` SHALL be disabled when Find What is empty. A replacement that empties a layer's text SHALL use the blank-allowed re-set path and leave an empty type layer. Cancelling the dialog or finding no match SHALL record no state. A fully locked type layer SHALL be left unchanged.

#### Scenario: The current layer is the default scope [fr_current_layer]

- **WHEN** the current layer is type and Replace All runs with `Search All Layers` off
- **THEN** matches change in that layer only, one state is recorded, and other type layers are unchanged

#### Scenario: Search All Layers covers the document [fr_all_layers]

- **WHEN** Replace All runs with `Search All Layers` on
- **THEN** every type layer that is not fully locked, including layers inside groups, has its matches replaced under one state

#### Scenario: No match records nothing [fr_no_match]

- **WHEN** Replace All finds no match or the dialog is cancelled
- **THEN** the document and history are unchanged

#### Scenario: A locked type layer is skipped [fr_locked]

- **WHEN** `Search All Layers` includes a fully locked type layer
- **THEN** that layer's text is unchanged and the other matching layers are replaced

#### Scenario: An empty Find What disables Replace All [fr_empty_find]

- **WHEN** Find What is empty
- **THEN** Replace All is disabled and records no state

#### Scenario: A blank result keeps the type layer [fr_blank_result]

- **WHEN** a replacement removes all of a layer's text
- **THEN** the layer becomes an empty type layer and one state is recorded

### Requirement: Matching options are honoured

`Case Sensitive` SHALL make matching case-sensitive; without it, ASCII letters SHALL compare without case. `Whole Word Only` SHALL accept a match only when the characters immediately before and after it are not alphanumeric or an underscore. Replacements SHALL NOT be rescanned. `Forward` SHALL select the scan direction: on, it SHALL scan the original text left-to-right and pick the first non-overlapping match; off, it SHALL scan right-to-left and pick the last. Replacing `aa` with `X` in `aaa` SHALL produce `Xa` with Forward on and `aX` with Forward off.

#### Scenario: Case sensitivity changes the result [fr_case]

- **WHEN** Find What is `cat` and the text contains `cat` and `Cat`
- **THEN** Replace All with Case Sensitive changes only `cat`, and without it changes both

#### Scenario: Whole Word Only rejects part of a word [fr_whole_word]

- **WHEN** Find What is `cat` and the text contains `cat` and `catalog`
- **THEN** Replace All with Whole Word Only changes `cat` and leaves `catalog` unchanged

#### Scenario: Forward decides overlapping replacements [fr_forward]

- **WHEN** Find What is `aa`, Replace With is `X`, and the text is `aaa`
- **THEN** Forward on produces `Xa`, Forward off produces `aX`, and no inserted replacement is rescanned

### Requirement: Replaced text stays live type

After a replacement, the changed layers SHALL re-layout and re-render, SHALL remain editable type layers, and SHALL round-trip their new text through PSD type data. One undo SHALL restore the previous text in every changed layer.

#### Scenario: The result is still type [fr_round_trip]

- **WHEN** a replacement changes a type layer and the document is saved and reopened
- **THEN** the layer is still a type layer with the replaced text and its font attributes
