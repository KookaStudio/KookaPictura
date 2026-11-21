# Numeric Fields Spec Delta

## ADDED Requirements

### Requirement: Shared fields use the system numeric locale

Shared numeric fields SHALL use the same system numeric locale to validate,
parse, and display values, independent of the interface language. Decimal and
fractional edits SHALL accept the locale's decimal separator. Fields SHALL omit
and reject digit grouping, keep their configured precision and tight suffixes,
and preserve their existing clamping, stepping, preview, commit, and invalid
input handling. The result delivered to the model SHALL be numeric. Editing,
saving, and reopening a document SHALL preserve the canonical numeric value
through the existing document storage format.

#### Scenario: Comma decimal editing preserves a fraction [loc_numeric_edit]

- **WHEN** a decimal-capable or fractional shared field is edited under a
  comma-decimal locale by typing `1,5` and committing
- **THEN** it accepts the input, commits numeric 1.5 once, and displays the
  value with a comma and its configured precision and suffix

#### Scenario: Stepping, clamping, and reverting use the same locale [loc_numeric_keys]

- **WHEN** a user types a comma decimal, steps with an arrow key, and then
  tries an out-of-range value and invalid or grouped input
- **THEN** stepping starts at the typed value, the out-of-range commit
  clamps as before, and invalid input uses the existing revert behavior

#### Scenario: A localized document value survives a save and reopen [loc_numeric_roundtrip]

- **WHEN** the user enters a width of `1,5` inches for a document at 72
  pixels per inch, then saves and reopens a PSD
- **THEN** the reopened document is 108 pixels wide at 72 pixels per inch,
  and presenting that width in inches in the same locale uses a comma decimal
