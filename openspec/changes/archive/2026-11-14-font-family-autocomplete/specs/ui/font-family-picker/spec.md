# Spec Delta

## ADDED Requirements

### Requirement: Typing a family name starts a new search

When a font family field is focused, the first printable character typed SHALL
start a new family search: it SHALL replace the family already shown rather than
append to it. Navigation and deletion keys SHALL leave the shown text editable
without starting a new search. A programmatic family write from the Type options
or a text session that carries the family already shown in the field SHALL NOT
replace the field's text or clear its completion; an external change to a
different family may update the field.

#### Scenario: A typed prefix replaces the shown family [ffp_typed_prefix]

- **WHEN** the Character panel's family field shows `Liberation Sans` and the user focuses it and types `noto`
- **THEN** the field's text is a family beginning with `noto` under a case-insensitive comparison, not `Liberation Sansnoto`

#### Scenario: Navigation keeps the shown family editable [ffp_nav_keeps_text]

- **WHEN** the Character panel's family field shows `Liberation Sans` and the user focuses it, presses End, and types ` x`
- **THEN** the field keeps `Liberation Sans` and appends ` x` (the text is edited, not replaced)

#### Scenario: A live option sync keeps the typed prefix [ffp_live_sync]

- **WHEN** the Type options bar or the Character panel writes the same family back while the user is typing a prefix
- **THEN** the field keeps the typed prefix and its completion set, and does not jump to an unrelated family

### Requirement: Family completion matches case-insensitively

The font family fields SHALL complete from the typed prefix against every
installed family whose name begins with that prefix, comparing
case-insensitively, so typing `noto` offers `Noto Sans` regardless of the
family's casing. Selecting a family from the family list SHALL keep applying
the exact family name.

#### Scenario: Lowercase prefixes match mixed-case families [ffp_prefix_case]

- **WHEN** `noto` is typed as the search prefix in a family field
- **THEN** the offered completions include `Noto Sans`

#### Scenario: Every family input completes the same way [ffp_all_inputs]

- **WHEN** `noto` is typed into the Character panel, the Type options bar, the Paragraph Style dialog, or the Glyphs panel family field
- **THEN** each field offers `Noto Sans` as a case-insensitive completion

#### Scenario: Exact-case selection is unchanged [ffp_exact_pick]

- **WHEN** the user selects `Noto Sans` from a family list
- **THEN** `Noto Sans` is applied exactly, as before
