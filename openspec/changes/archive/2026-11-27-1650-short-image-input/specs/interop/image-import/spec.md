# Spec Delta

## ADDED Requirements

### Requirement: The Qt decode edge refuses short input and pins known signatures

Before building a `QImageReader`, the Qt decode edge SHALL refuse input shorter
than 16 bytes. No image plugin SHALL be asked to recognise such input. When the
input begins with a PNG, JPEG, GIF, BMP, or TIFF signature, the edge SHALL pin
the reader's format to that format and SHALL turn content auto-detection off,
so that no other plugin is probed. Input of 16 bytes or more that begins with
any other signature SHALL still be probed through the installed plugins. A
refused open SHALL report a failure to the user and SHALL NOT end the process.

#### Scenario: An empty file is refused before any plugin is probed [image-import-empty-input]

- **WHEN** the Qt decode edge is given 0 bytes
- **THEN** it returns failure with a zero width and height, without building a `QImageReader`

#### Scenario: Input under 16 bytes is refused [image-import-short-input]

- **WHEN** the Qt decode edge is given 3 bytes, or the first 15 bytes of a valid PNG
- **THEN** it returns failure with a zero width and height

#### Scenario: A known signature pins the reader's format [image-import-pinned-format]

- **WHEN** the input begins with a PNG, JPEG, GIF, BMP, or TIFF signature
- **THEN** the reader's format is `png`, `jpeg`, `gif`, `bmp`, or `tiff` with auto-detection off, a valid PNG still decodes, and a PNG signature followed by garbage is refused

#### Scenario: An unknown signature is still probed [image-import-unknown-probed]

- **WHEN** the input is a valid 16-byte-or-longer PBM, whose signature the engine probe does not know
- **THEN** no format is pinned and Qt decodes it

#### Scenario: Opening an empty or 3-byte image reports a failure [image-import-open-short-file]

- **WHEN** File ▸ Open is given a 0-byte `.jpg` and then a 3-byte `.jpg`
- **THEN** no document opens, the status bar reports `Could not open`, and the app keeps running
