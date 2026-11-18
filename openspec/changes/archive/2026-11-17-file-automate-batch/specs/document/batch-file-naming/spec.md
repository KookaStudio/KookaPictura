# batch-file-naming Specification

## Purpose
The CS6 Batch File Naming scheme: an ordered list of name fields evaluated per
input into an output file name, with serials, dates, extensions, cross-platform
compatibility, and a uniqueness rule.

## ADDED Requirements

### Requirement: File Naming tokens

A File Naming scheme SHALL be an ordered list of up to six fields. Each field
SHALL be empty, custom text, or exactly one of the CS6 tokens: `Document Name`,
`document name`, `DOCUMENT NAME`, `1 Digit Serial Number`,
`2 Digit Serial Number`, `3 Digit Serial Number`, `4 Digit Serial Number`,
`Serial Letter (a, b, c...)`, `Serial Letter (A, B, C...)`, `mmddyy (date)`,
`mmdd (date)`, `yyyymmdd (date)`, `yymmdd (date)`, `yyddmm (date)`,
`ddmmyy (date)`, `ddmm (date)`, `extension`, and `EXTENSION`. The output name
SHALL be the concatenation of the evaluated fields in order:

- `Document Name` SHALL evaluate to the input's base name without its final
  extension, as written; `document name` to its lowercase form; `DOCUMENT NAME`
  to its uppercase form.
- An `N Digit Serial Number` SHALL evaluate to the input's serial number
  zero-padded to `N` digits; a serial with more than `N` digits SHALL be
  written in full.
- A serial letter SHALL evaluate to the input's zero-based position as a
  bijective base-26 letter sequence (`a`…`z`, `aa`, `ab`, …), in the field's
  case. CS6 Help documents only that letters begin at `a`/`A`; the sequence
  after `z` is an inferred app decision.
- A date token SHALL evaluate to the batch's start date in local time in the
  token's digit order.
- `extension` SHALL evaluate to a period followed by the output format's
  extension in lowercase, and `EXTENSION` to the same in uppercase.
- Custom text SHALL evaluate to itself.

When no field is an extension token, the system SHALL append a period and the
lowercase output extension so that no output is written without an extension
(an inferred app decision).

#### Scenario: Tokens evaluate in order [bfn_tokens]
- **WHEN** the scheme is `document name`, custom text `_v`, `3 Digit Serial Number`, `EXTENSION` for input `Beach.PSD` with serial 7 and output format JPEG
- **THEN** the output name is `beach_v007.JPG`

#### Scenario: Serial letters continue past z [bfn_serial_letter]
- **WHEN** a `Serial Letter (A, B, C...)` field is evaluated for zero-based positions 0, 25, and 26
- **THEN** the results are `A`, `Z`, and `AA`

#### Scenario: Date tokens use the batch start date [bfn_dates]
- **WHEN** each date token is evaluated for a batch started on 2026-03-04
- **THEN** `mmddyy` gives `030426`, `mmdd` `0304`, `yyyymmdd` `20260304`, `yymmdd` `260304`, `yyddmm` `260403`, `ddmmyy` `040326`, and `ddmm` `0403`

#### Scenario: A missing extension field is appended [bfn_default_extension]
- **WHEN** the scheme is `Document Name` alone for input `a.tif` saved as TIFF
- **THEN** the output name is `a.tif`

### Requirement: Starting serial number

The scheme SHALL carry a starting serial number of at least 1, defaulting to 1.
The first processed input SHALL receive the starting serial and each following
input the next integer, in source order. Serial letters SHALL start at `a`/`A`
for the first input regardless of the starting serial number. An input that
fails SHALL still consume its serial.

#### Scenario: Serials count from the start value [bfn_serial_start]
- **WHEN** three inputs are named with `2 Digit Serial Number` and starting serial 9
- **THEN** their names begin `09`, `10`, and `11`

### Requirement: Name compatibility

The scheme SHALL carry Windows, Mac OS, and Unix compatibility flags. Unix
compatibility SHALL always apply on Linux and SHALL replace `/` and NUL with
`_`. Windows compatibility SHALL replace `\ / : * ? " < > |` and control
characters with `_` and SHALL remove trailing spaces and periods from the name
before the extension. Mac OS compatibility SHALL replace `:` with `_`. The
replacement character is an inferred app decision; CS6 Help documents only that
the option makes names safe for the platform.

#### Scenario: Windows compatibility sanitizes a name [bfn_compat_windows]
- **WHEN** the evaluated base name is `a:b?c. ` with Windows compatibility on
- **THEN** the base name becomes `a_b_c`

#### Scenario: Unix compatibility always applies [bfn_compat_unix]
- **WHEN** custom text `x/y` is evaluated with every optional compatibility flag off
- **THEN** the base name becomes `x_y`

### Requirement: Unique-field rule

A scheme SHALL be valid only when at least one field is a document-name token,
a serial-number token, or a serial-letter token. A Batch whose Folder
destination uses a scheme with no such field SHALL be refused with a diagnostic
naming the rule, and no input SHALL be processed.

#### Scenario: A scheme without a unique field is rejected [bfn_unique]
- **WHEN** the scheme is `yyyymmdd (date)` and `extension` for a Folder destination
- **THEN** validation fails with the unique-field diagnostic and no file is opened
