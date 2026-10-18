## ADDED Requirements

### Requirement: Hex shorthand and prefixes

The color picker's hexadecimal field SHALL accept a leading `#` or `0x`
(case-insensitive) and a three-digit shorthand, expanding each shorthand digit
to a pair (`#123` becomes `#112233`). A complete six-digit value SHALL apply
while the user types. A three-digit shorthand SHALL apply when the field is
committed by Enter or focus loss. A value that is neither three nor six hex
digits SHALL leave the current colour unchanged; committing an invalid value
SHALL restore the field to the current colour. After a commit the field SHALL
show the canonical bare uppercase six-digit value.

#### Scenario: Prefixes are accepted [cp_hex_prefix]

- **WHEN** `#aabbcc` or `0xAABBCC` is entered in the hex field
- **THEN** the picker's colour is `#aabbcc`

#### Scenario: Shorthand expands on commit [cp_hex_shorthand]

- **WHEN** `#123` is entered and the field is committed
- **THEN** the picker's colour is `#112233` and the field shows `112233`

#### Scenario: Invalid input leaves the colour [cp_hex_invalid]

- **WHEN** a value that is not three or six hex digits is committed
- **THEN** the picker's colour is unchanged and the field shows the current colour's hex

### Requirement: Add to Swatches

The color picker SHALL provide an Add to Swatches action that appends the
picker's current colour to the application's Swatches panel. The action SHALL
prompt for a name defaulting to the panel's next generated swatch name, and
SHALL append the colour only when the prompt is confirmed. The action SHALL NOT
close the picker. When the picker has no Swatches panel, the action SHALL be
disabled.

#### Scenario: Add the current colour [cp_add_swatch]

- **WHEN** Add to Swatches is clicked with a Swatches panel present and the name prompt is confirmed
- **THEN** the current colour is appended to the panel under the entered name and the picker stays open

#### Scenario: Cancel the name prompt [cp_add_swatch_cancel]

- **WHEN** the name prompt is cancelled
- **THEN** no swatch is appended

#### Scenario: No panel [cp_add_swatch_disabled]

- **WHEN** the picker has no Swatches panel
- **THEN** Add to Swatches is disabled
