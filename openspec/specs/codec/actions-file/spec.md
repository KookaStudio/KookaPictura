# codec/actions-file Specification

## Purpose
Read and write Adobe Photoshop version-16 action-set and palette files and the
exact Action Manager descriptor encoding, preserving unknown data and failing
safely on untrusted input.

## Requirements

### Requirement: Action-set container read

The system SHALL read a version-16 `.atn` file as exactly one action set. The
reader SHALL accept the file with no leading magic or file-type signature and
SHALL read all container integers big-endian; descriptor internals follow the
value grammar, including the little-endian `Pth ` path fields. It SHALL read the
set name as a counted UTF-16BE string whose count is authoritative and whose
trailing NUL is preserved when present, the set expanded flag, and the action
count; each action's function-key number (0 = none, n = Fn; values outside 1–12
preserved opaquely), Shift and Ctrl/Command flags, color index (0–7 documented;
other values preserved opaquely), name (counted UTF-16BE; trailing NUL preserved
when present), expanded flag, and step count; and each step's expanded, enabled,
with-dialog, and dialog-options bytes (the last in 0–4), event-ID form (`TEXT` or
`long`), event ID (a four-character code after `long` or a length-prefixed byte
string after `TEXT`), display name (a length-prefixed byte string; preserve its
bytes verbatim and decode lazily), has-descriptor marker (`0` or `-1`), and, when
the marker is `-1`, the
Action Manager descriptor with no version or length prefix. Bytes after the
single declared set SHALL be preserved opaquely and re-emitted on write. A file
whose version is not 16 SHALL fail with a named diagnostic rather than be
decoded with version-16 rules.

#### Scenario: Valid version-16 set loads [af_atn_read]

- **WHEN** a version-16 `.atn` with one set, actions, and steps is read
- **THEN** every documented field is decoded and the set is returned

#### Scenario: No magic signature is required [af_atn_no_magic]

- **WHEN** a valid version-16 file begins directly with the version integer
- **THEN** it loads successfully and no `8BAC` signature is required

#### Scenario: Wrong version is refused [af_atn_version]

- **WHEN** the file version is not 16
- **THEN** reading fails with a diagnostic naming the version and no set is returned

#### Scenario: Trailing bytes are preserved [af_atn_trailing]

- **WHEN** a version-16 file has bytes after the declared action set
- **THEN** the bytes are retained and re-emitted unchanged on write

#### Scenario: Dialog options survive a read [af_atn_dialog_options]

- **WHEN** steps carrying dialog-options values 0 through 4 are read and written
- **THEN** 0–3 are preserved and 4 is accepted but written as 0

### Requirement: Action-set container write

The system SHALL write one action set per `.atn` file using the version-16 layout.
Writing a set read from a supported file and reading it again SHALL reproduce the
same set, action, step, flag, function-key, color, event-ID, display-name, and
descriptor values. Authored set and action names SHALL be counted UTF-16BE
strings including the trailing NUL code unit; authored event display names and
`TEXT` event identifiers SHALL be length-prefixed byte strings without a NUL
terminator; imported byte strings SHALL be preserved verbatim. The writer SHALL
preserve the exact identifier form used by the source (length-prefixed string
versus interned four-character code), descriptor
item order, duplicate keys, and unknown event identifiers. An action set written
by the system and read back SHALL be byte-identical when every contained value is
supported.

#### Scenario: Write-read round-trip [af_atn_roundtrip]

- **WHEN** a supported set is written to `.atn` and read back
- **THEN** the decoded structure equals the source structure

#### Scenario: Identifier form is preserved [af_atn_id_form]

- **WHEN** a descriptor uses both a length-prefixed string identifier and an interned code with the same characters
- **THEN** writing and reading preserves which form each identifier used

#### Scenario: Item order and duplicates are preserved [af_atn_order]

- **WHEN** a descriptor contains repeated keys or a non-sorted item order
- **THEN** the written bytes preserve that order and the duplicate entries

#### Scenario: Unknown event identifiers survive a load-save [af_atn_unknown_save]

- **WHEN** a set contains an event whose identifier is not understood but whose descriptor is fully decodable
- **THEN** the identifier, display name, flags, and descriptor are preserved and re-emitted on save

### Requirement: Palette container

The system SHALL read and write `Actions Palette.psp` files as a version-16
palette containing a set count followed by that many action sets. It SHALL NOT
require a magic signature. Reading SHALL return every set in order; writing SHALL
write every set in order. A palette SHALL NOT be treated as a backup format or as
an import/export interchange format.

#### Scenario: Multiple sets round-trip [af_psp_roundtrip]

- **WHEN** a palette containing two sets is written and read
- **THEN** both sets and their order are preserved

#### Scenario: Palette set count is honored [af_psp_count]

- **WHEN** a palette declares more sets than its bytes contain
- **THEN** reading reports a truncated palette error and returns the sets it decoded

#### Scenario: Partial palette load is reported [af_psp_partial]

- **WHEN** a palette file contains a valid prefix followed by a malformed set
- **THEN** the valid sets are returned together with an error naming the failure offset

#### Scenario: Wrong palette version is refused [af_psp_version]

- **WHEN** a palette file's version is not 16
- **THEN** reading fails with a diagnostic naming the version and no sets are returned

### Requirement: Descriptor value and reference codec

The system SHALL decode and encode the Action Manager descriptor values used by
actions: object/global object, list, double, unit double, unit-float list, text,
enumeration, 32-bit integer, 64-bit integer, boolean, class/global class, alias,
raw data, file path, object array, and reference. It SHALL decode and encode the
reference forms property, class, enumerated, offset, identifier, index, and name.
`ObAr` SHALL be read with its object count, shared object class, item count,
and one key/unit/double-list entry per item; each `UnFl` entry SHALL carry its
unit ID, a double count equal to the object count, and that many doubles. `Pth `
SHALL use its little-endian path fields.
In `.atn` version-16 descriptors, identifier and index references SHALL carry the
desired class before the numeric value. The codec SHALL distinguish a
length-prefixed string identifier from an interned four-character code.

#### Scenario: Every supported value type round-trips [af_desc_values]

- **WHEN** a descriptor contains every supported value type, including `ObAr`, `UnFl`, and `Pth `
- **THEN** decode and encode reproduce the same values and bytes

#### Scenario: Every reference form round-trips [af_desc_refs]

- **WHEN** a reference contains every supported form
- **THEN** decode and encode reproduce the same forms and bytes

#### Scenario: ATN integer references have a class [af_desc_idnt_class]

- **WHEN** an identifier or index reference is decoded from a version-16 `.atn`
- **THEN** the desired class is read before the numeric value

#### Scenario: Unknown descriptor type fails closed [af_desc_unknown]

- **WHEN** a descriptor contains an unrecognized value type
- **THEN** decoding fails with a named diagnostic and does not guess a byte length

### Requirement: Malformed input safety

The system SHALL treat `.atn` and `.psp` files as untrusted input. Lengths,
counts, nesting depth, and allocations SHALL be bounded. A malformed file SHALL
return a typed error with the failing offset and SHALL NOT panic, over-read, or
silently drop a partially decoded set. A load failure SHALL return a typed error
and SHALL NOT mutate caller-owned data.

#### Scenario: Truncated file is rejected safely [af_malformed_truncated]

- **WHEN** a file ends inside a string, descriptor, or action record
- **THEN** reading returns a typed truncation error and the process does not crash

#### Scenario: Oversized count is rejected [af_malformed_count]

- **WHEN** a count or length exceeds the documented bound
- **THEN** reading stops with a typed error before allocating the claimed size
