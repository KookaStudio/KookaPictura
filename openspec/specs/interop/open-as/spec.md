# interop/open-as Specification

## Purpose
`File ▸ Open As…`: open a file with a decoder the user picks from a format
list instead of the one its signature or extension selects, reporting a format
mismatch when that decoder rejects the file.

## Requirements

### Requirement: Open As picks the decoder

`File > Open As…` (`Ctrl+Alt+Shift+O`) SHALL show the open file dialog with
one filter per readable format: Photoshop, Large Document Format, Generic PDF,
Generic EPS, PNG, JPEG, TIFF, WebP, GIF, and BMP. The chosen filter SHALL
select the decoder regardless of the file's extension or signature. When that
decoder rejects the file, the window SHALL report `Could not open <file>:
format mismatch or damaged file.` and open nothing. `File > Open`'s filters
SHALL list Generic PDF (`*.PDF *.PDP *.AI`) and Generic EPS (`*.EPS *.EPSF
*.PS`), and All Formats SHALL include them.

#### Scenario: Open As is a live command [open_as_command]

- **WHEN** the File menu is shown
- **THEN** `Open As…` is enabled with shortcut `Ctrl+Alt+Shift+O`

#### Scenario: The chosen format overrides the extension [open_as_forces_format]

- **WHEN** a PNG file named `photo.dat` is opened with Open As and the PNG format
- **THEN** a document opens with the PNG's pixels

#### Scenario: A mismatched format is reported [open_as_mismatch]

- **WHEN** a PNG file is opened with Open As and the Generic PDF format
- **THEN** the status reads `Could not open <file>: format mismatch or damaged file.` and no document is added

#### Scenario: Open lists PDF and EPS [open_filters_pdf_eps]

- **WHEN** the Open dialog's filters are read
- **THEN** they include `Generic PDF (*.PDF *.PDP *.AI)` and `Generic EPS (*.EPS *.EPSF *.PS)`, and All Formats includes those patterns
