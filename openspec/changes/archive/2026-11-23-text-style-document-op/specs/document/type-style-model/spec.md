# Spec Delta

## ADDED Requirements

### Requirement: Named style edits are one validated document operation

The system SHALL apply a named character or paragraph style edit — an optional
rename from its source name to a target name plus the replacement character and
paragraph attribute sets — as one document operation that validates the whole
request before changing anything. The operation MUST refuse a request whose
source style does not exist, whose target name already names another style of
the same kind in the sheet, that renames the protected `Basic Paragraph` style,
or that renames another style to `Basic Paragraph`. A refused request MUST leave
the style sheet,
every type layer's stored applied-style reference, the rendered type content, the
composite pixels, and history unchanged, and MUST report failure to its caller.

On acceptance the operation SHALL rename the style when the names differ, write
the replacement attributes, rewrite the applied-style reference of every type
layer that applied the source name to the target name — including layers nested
inside groups — and, where the document's type layers are editable (8-bit RGB),
re-resolve every type layer applying the target name against the updated sheet
while preserving each layer's manual overrides. Editing `Basic Paragraph` under
its own name SHALL re-resolve every type layer that applies another character or
paragraph style, because those layers inherit `Basic Paragraph` as their base; a
type layer that applies no style keeps its authored attributes as local
formatting and is left as it is. Where they are not editable, the sheet and
reference changes still apply and the rendered content is left as it was. An accepted edit, including a rename combined with attribute changes, SHALL
be one undoable state that restores the style sheet, the applied references, and
the resulting type content together. A refused edit SHALL record no history
state.

#### Scenario: Renaming an applied paragraph style keeps it applied

- **WHEN** a paragraph style applied to type layers, including a type layer nested in a group, is renamed together with an attribute change and the rename is accepted
- **THEN** the style sheet holds the new name, every applying layer references it, and a later attribute edit to the renamed style still updates those layers' content

#### Scenario: A rename and its attribute change are one undo step

- **WHEN** an accepted edit renames an applied paragraph style and changes an attribute, and then undo runs once
- **THEN** the style name in the sheet, the applying layers' references, and their rendered content are restored together, and redo reapplies all three together

#### Scenario: A duplicate target name is refused

- **WHEN** a rename request names a target name already used by another style of the same kind in the sheet
- **THEN** the operation reports failure and the style sheet, the applied references, the rendered content, the composite pixels, and history are unchanged

#### Scenario: A missing source style is refused

- **WHEN** an edit or rename request names a source style that does not exist
- **THEN** the operation reports failure and the style sheet, the applied references, the rendered content, the composite pixels, and history are unchanged

#### Scenario: Basic Paragraph is never renamed

- **WHEN** a request renames `Basic Paragraph` to another name or renames another paragraph style to `Basic Paragraph`
- **THEN** the operation reports failure and leaves the sheet, applied references, content, and history unchanged, while an attribute edit to `Basic Paragraph` under its own name is still accepted

#### Scenario: Editing Basic Paragraph reaches the layers that apply another style

- **WHEN** `Basic Paragraph` is edited under its own name and a type layer applies another character or paragraph style
- **THEN** the operation re-resolves that layer against the updated sheet, so an attribute it inherits from `Basic Paragraph` changes while the applied style's own attributes still win

#### Scenario: Character styles share the same edit contract

- **WHEN** an applied character style is renamed with an attribute change, or a character-style request is refused for a missing source or duplicate target
- **THEN** the accepted edit keeps every applying layer's reference and records one undoable state, and the refused request changes nothing and reports failure

#### Scenario: Non-editable type content is left as it is

- **WHEN** a rename is accepted in a document whose type layers cannot be re-set (not 8-bit RGB)
- **THEN** the style sheet and the applying layers' references carry the new name, and the rendered type content is left as it was
