# Variable data set import and export

## Purpose

Imports spreadsheet rows into a PSD template and writes a layered PSD for each selected data set.

## ADDED Requirements

### Requirement: Delimited-text imports validate every row before commit

Import SHALL detect CSV or tab-delimited syntax from the header outside quoted fields and read LF, CRLF, or CR record endings. If the header has no unquoted delimiter, `.csv` SHALL select comma and `.tsv` or `.txt` SHALL select tab, case-insensitively; an unrecognized extension SHALL be rejected with an explanation. If the header contains both unquoted delimiters, import SHALL reject it as ambiguous. The selected delimiter SHALL govern every row, so an unquoted comma remains data in a one-column TSV. It SHALL support doubled quotes, delimiters and newlines within quoted cells, and preserve quoted whitespace while trimming unquoted outer spaces. Automatic encoding SHALL detect UTF-8 and Unicode BOMs and fall back to Windows-1252. The explicit selection SHALL support UTF-8, UTF-16 LE/BE, and Windows-1252 and report invalid input for the selected codec. The unique header names SHALL match all document variables in any order. Each row SHALL contain the same number of cells as the header and valid values for each variable kind. Relative image references SHALL resolve against the input file's parent directory and keep that meaning after saving the template. Import SHALL report the offending row or column for syntax and validation errors. It SHALL commit all rows as one undoable edit or leave the existing collection intact.

#### Scenario: CSV and TSV preserve quoted and encoded data [vars_import_syntax]

- **WHEN** files in the supported encodings and line endings contain quoted separators, a quoted multiline value, doubled quotes, and padded fields
- **THEN** the same typed values are parsed for both delimiters and whitespace inside quoted fields is preserved

#### Scenario: The first column is both a value and a name [vars_import_names]

- **WHEN** a reordered, valid header is imported with Use First Column For Data Set Names selected
- **THEN** the first cell names its set and supplies the named variable's value, and an extra naming-only column is rejected

#### Scenario: Rejected imports preserve the existing collection [vars_import_atomic]

- **WHEN** Replace Existing Data Sets is selected and a later row is malformed, mis-sized, invalid, or duplicates a set name
- **THEN** validation names the failure and no row is imported or removed

#### Scenario: A valid import resolves paths and is undoable [vars_import_commit]

- **WHEN** a valid file with a relative image path is appended or used to replace existing sets and the template is saved elsewhere
- **THEN** the stored image resolves against its import directory after reopen, automatic names avoid collisions, and one undo in the original session restores the original collection

#### Scenario: Single-column files use their extension [vars_import_single_column]

- **WHEN** a one-variable `.csv`, `.tsv`, or `.txt` file has no header delimiter, including a TSV row containing an unquoted comma, or an unknown extension or mixed-delimiter header is supplied
- **THEN** recognized extensions select their documented delimiter, the TSV comma remains part of the value, and ambiguous or unknown formats are rejected without changing sets

### Requirement: Batch export creates independent layered PSDs

Data Sets As Files SHALL accept a target folder and a selected subset of sets, processed in document order. Each PSD SHALL apply its own set to the same initial document state and retain the editable layers and template metadata. The default name SHALL combine the base name and set name. Custom naming SHALL support the base name, set name, set index and literal text. Output names SHALL be sanitized, unique, and valid as single file names in the target folder, with collisions resolved by numbered suffixes. Any set rejected by evaluation, including an unreadable replacement, invalid target, unsupported mode or conversion, or allocation failure, SHALL be reported with its name and reason and skipped; processing SHALL continue with the next selected set. An invalid destination or failed output write SHALL be reported and stop the batch. Cancellation SHALL stop before the next file. An incomplete file SHALL NOT be left at its final path. The source document, dirty state, history and path SHALL be unaffected.

#### Scenario: Selected outputs start from the same template [vars_export_outputs]

- **WHEN** two of three sets are selected and the second selected set uses Do Not Replace after the first set replaces an image
- **THEN** exactly two layered PSDs reopen with their own applied text, visibility, and pixel content, and the second retains the initial image

#### Scenario: Output names remain unique and inside the target folder [vars_export_names]

- **WHEN** a naming pattern yields long, illegal, already present, or colliding names
- **THEN** generated PSD names are safe, distinct file names and existing files remain intact

#### Scenario: Errors and cancellation preserve completed work [vars_export_errors]

- **WHEN** a set has an unreadable image, invalid target, unsupported mode or conversion, or another evaluator failure, a later output write fails, or cancellation is requested
- **THEN** the invalid set is reported and skipped, failure or cancellation stops further output, completed PSDs remain readable, and the live document is unaffected
