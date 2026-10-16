# color/color-management Specification (delta)

## ADDED Requirements

### Requirement: ICC serialization is byte-deterministic

`Profile::to_icc` SHALL produce byte-identical output each time it serializes a
built-in working-space profile, regardless of when it is called. To make this
hold, `to_icc` SHALL zero the profile's creation-date field — the 12-byte
`dateTimeNumber` at ICC header bytes 24..35 (ICC.1:2022 §7.2.8) — rather than
leaving the wall-clock time Little CMS stamps at profile creation, for which it
offers no setter. The zeroing MUST NOT break `to_icc`'s panic-free contract on a
short or empty buffer. A serialized built-in profile MUST still reload through
`Profile::from_icc` with the same color-space signature and MUST remain a valid
ICC profile that an external ICC reader accepts and can name by its description.

#### Scenario: Repeated serialization is byte-identical [icc_serial_repeat]

- **WHEN** `to_icc` is called twice on a built-in profile
- **THEN** the two byte vectors are equal

#### Scenario: The serialized creation date is zeroed [icc_serial_zero]

- **WHEN** a built-in profile is serialized with `to_icc`
- **THEN** bytes 24..35 of the result are all zero

#### Scenario: A serialized built-in profile reloads [icc_serial_reload]

- **WHEN** the bytes from `to_icc` are passed to `Profile::from_icc`
- **THEN** the reloaded profile succeeds and has the same color-space signature as
  the original

#### Scenario: An external ICC reader accepts the zeroed-date profile [icc_serial_external]

- **WHEN** a document carrying a built-in profile is saved and an external ICC
  reader opens the embedded profile
- **THEN** the reader accepts it and reports the profile's description
