# Spec Delta

## Purpose

Keeps an action step's executable payload and its preserved imported form
consistent across every edit, so a saved library reproduces exactly what the
in-memory library holds after a rename or parameter change.

## ADDED Requirements

### Requirement: Semantic edits own imported-form consistency

The system SHALL apply every change to an action step's executable payload
through one edit surface that also decides the fate of the preserved imported
form. A semantic edit that changes the executable payload and is not a Play or
Conditional reference rewrite SHALL invalidate the preserved form so
serialization re-authors the step from the payload. A Play or Conditional
reference rewrite SHALL update the payload and preserved form together as
required below. An edit whose resulting value equals the current value SHALL NOT
invalidate the preserved form, and a flag-only edit (enabled or with-dialog)
SHALL NOT invalidate it. Structural edits (moving, reordering, duplicating, or
deleting) SHALL NOT invalidate it. Callers SHALL NOT be required to clear the
preserved form themselves after a semantic edit.

#### Scenario: A changed payload re-authors the step [ase_edit_invalidates]

- **WHEN** a step with a preserved imported form has its parameters or Insert Path changed to a different value
- **THEN** saving writes a descriptor built from the new payload and does not carry the superseded imported descriptor

#### Scenario: A no-op edit preserves the imported form [ase_noop_keeps_imported]

- **WHEN** a step with a preserved imported form receives a parameter edit equal to its current value, or an Insert Path edit with its current path
- **THEN** saving reproduces the imported form, including unknown descriptor content, unchanged

#### Scenario: Flag-only edits preserve the imported form [ase_flag_keeps_imported]

- **WHEN** the enabled or with-dialog flag of a step with a preserved imported form is toggled
- **THEN** saving still emits the imported descriptor byte-for-byte

#### Scenario: Structural edits keep the imported form [ase_structural_keeps_imported]

- **WHEN** a step with a preserved imported form is moved or duplicated
- **THEN** saving reproduces the imported form unchanged

### Requirement: Reference rewrites survive save and reload

Renaming an action SHALL rewrite its Play and Conditional references in both the
executable payload and the preserved imported form, so that serialization and
reload reproduce the new reference. When the preserved descriptor can be
updated, unrelated imported descriptor items, unrelated JSON bytes, and the
step's display-name bytes SHALL be preserved. JSON patching SHALL replace only
recognized top-level string values. Conditional patching SHALL touch only
sidecars matching the decoded condition and expected branches. When the
preserved descriptor cannot be updated, the step SHALL be re-authored from the
executable payload rather than keep a stale reference.

#### Scenario: An imported Play reference survives save and reload [ase_rename_play]

- **WHEN** an imported Play step targets an action and that action is renamed
- **THEN** saving and loading the library yields a Play step whose target is the new name

#### Scenario: An imported Conditional reference survives save and reload [ase_rename_conditional]

- **WHEN** an imported Conditional step names an action in its Then or Else branch, carries unrelated `jsn` sidecars, and that action is renamed
- **THEN** saving and loading the library yields a Conditional step whose branch names the new action, while the unrelated sidecars remain byte-identical

#### Scenario: A jsn-only Play reference is rewritten [ase_rename_jsn_only_play]

- **WHEN** an imported Play step stores its target only in the `jsn` action key and that action is renamed
- **THEN** saving and loading the library yields a Play step whose target is the new name

#### Scenario: A rewrite keeps unrelated imported content [ase_rename_preserves_unknown]

- **WHEN** an imported Play step carries an unknown descriptor item, duplicate JSON keys, a high-precision number, noncanonical key order or whitespace, and its target action is renamed
- **THEN** the saved and reloaded step still carries that unknown item, preserves every unrelated JSON byte, updates every matching target copy, and retains its original display-name bytes

### Requirement: A rejected rename is atomic

A rename rejected because another action already holds the new name SHALL leave
the in-memory library unchanged and SHALL produce serialized bytes identical to
the bytes produced before the rejected rename.

#### Scenario: Duplicate rename leaves everything unchanged [ase_rename_duplicate_atomic]

- **WHEN** an action is renamed to a name already used by another action in the set
- **THEN** the rename is refused and the library and its saved bytes are unchanged

### Requirement: Unedited imported steps remain lossless

A step that is not semantically edited SHALL re-emit its imported event,
descriptor, and display-name bytes verbatim, including unknown descriptor items
and unknown event identifiers. An edit to one step SHALL NOT disturb the
preserved form of any other step.

#### Scenario: An unedited imported step round-trips byte-for-byte [ase_unedited_roundtrip]

- **WHEN** an imported step with unknown descriptor items is saved without a semantic edit
- **THEN** the written step is byte-identical to the imported step

#### Scenario: An unrelated unknown event is untouched [ase_unrelated_unknown_event]

- **WHEN** an action is renamed while another action contains a step with an unknown event identifier and a preserved form
- **THEN** that unknown step is saved byte-identically to its imported form
