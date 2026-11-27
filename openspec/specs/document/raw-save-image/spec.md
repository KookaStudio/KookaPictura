# document/raw-save-image Specification

## Purpose
The raw open's Save Image: Save Options, a DNG written on a worker with the dialog's settings while the Pictura Raw dialog stays open, and the refusals that keep the source and existing files safe (#402).

## Requirements

### Requirement: Save Image is offered in a raw open's Pictura Raw dialog

The Pictura Raw dialog of a camera raw open SHALL show a **Save Image…**
button. The dialog reached from `Filter ▸ Pictura Raw…` SHALL NOT show it.

#### Scenario: Only the raw open shows Save Image [raw_save_button]

- **WHEN** a DNG is opened interactively, and `Filter ▸ Pictura Raw…` is shown on an open document
- **THEN** the raw open's dialog has a `rawSaveImageButton`, and the filter dialog has none

### Requirement: Save Options choose the destination and the DNG options

Save Image SHALL open **Save Options** with a destination folder, a file name,
and the DNG options **Compatibility** (Camera Raw 2.4, 4.1, 4.6, 5.4 or 6.6
and later; default 6.6), **JPEG Preview** (None, Medium Size, Full Size;
default Medium Size), **Embed Original Raw File** (default off) and **Convert
to Linear Image** (default off). The folder SHALL default to the raw file's
folder and the name to `<stem>.dng`, or `<stem>-<n>.dng` with the smallest
free `n` when that file exists. Medium Size SHALL be a preview whose long side
is at most 1024 pixels; Full Size the developed image's size.

#### Scenario: Default destination avoids existing files [raw_save_default_path]

- **WHEN** the default path is asked for `shot.dng` in a folder holding `shot.dng` and `shot-1.dng`, and for `frame.cr2` in an empty folder
- **THEN** the first is `shot-2.dng` in that folder and the second `frame.dng`

#### Scenario: Save Options defaults [raw_save_options_defaults]

- **WHEN** Save Image is clicked in a raw open's dialog
- **THEN** Save Options shows the raw file's folder, the default name, Camera Raw 6.6 and later, Medium Size, and both checkboxes off

### Requirement: Save writes a DNG with the dialog's settings and keeps the dialog open

Accepting Save Options SHALL write the DNG on a worker under a cancellable
progress window, then show the result in the Pictura Raw dialog, which stays
open; no document SHALL open and the Camera Raw settings stores SHALL NOT be
written. The worker SHALL re-read the raw file and refuse it when its content
hash differs from the open's. The DNG SHALL hold the mosaic plane read from
the file (or, with Convert to Linear Image, its demosaiced camera RGB), the
file's colour, crop, orientation, make and model, the dialog's settings as an
XMP packet over the seed record with the white-balance label of
`document/raw-settings-storage`, previews developed with those settings, and,
with Embed Original Raw File, the raw file's bytes. A raw file whose plane
is a SuperCCD grid SHALL be saved as its demosaiced, upright camera RGB
whether or not Convert to Linear Image is checked, since a mosaic DNG cannot
carry the turned lattice. The file SHALL be written
under a temporary name in the target folder and renamed into place.

#### Scenario: Save from the dialog [raw_save_writes_dng]

- **WHEN** a DNG open's dialog clicks Save Image, Save Options is accepted with a new name and Convert to Linear Image, and the dialog is then cancelled
- **THEN** the new file exists and starts with `II*\0`, the dialog's status names it, no document opened, and the Camera Raw database is empty

#### Scenario: Saved contents [raw_save_dng_contents]

- **WHEN** a pending raw from a synthetic DNG is saved as a mosaic with exposure +0.50, a Medium preview and the original embedded
- **THEN** `read_dng` of the output equals `read_raw` of the source, `raw_xmp` of the output reads exposure 0.5, the preview IFD exists, and the embedded original equals the source bytes

#### Scenario: Linear contents [raw_save_dng_linear]

- **WHEN** the same pending raw is saved with Convert to Linear Image and no preview
- **THEN** `read_linear_dng` of the output equals the source's demosaiced camera RGB and there is no preview IFD

#### Scenario: A SuperCCD raw saves linear [raw_save_superccd_linear]

- **WHEN** a pending raw from a synthetic SuperCCD RAF is saved without Convert to Linear Image
- **THEN** `read_linear_dng` of the output equals the source's demosaiced, upright camera RGB

### Requirement: A save never overwrites the source or a file without asking

A target that is the raw file itself SHALL be refused. A target that exists
SHALL be replaced only after the user agrees; declining SHALL write nothing.
A raw file changed since the open SHALL be refused. A cancelled save SHALL
leave neither the target nor a temporary file.

#### Scenario: Source and changed files are refused [raw_save_refuses]

- **WHEN** a pending raw is saved onto its own path, and saved after its file was rewritten with other bytes
- **THEN** both saves fail, the first naming the source and the second saying the file changed, and the raw file keeps its bytes

#### Scenario: Declining replace writes nothing [raw_save_confirms_replace]

- **WHEN** Save Options names an existing file and the replace question is declined
- **THEN** the existing file keeps its bytes and the dialog's status is empty

#### Scenario: Cancel leaves no file [raw_save_cancel]

- **WHEN** a save is cancelled before it writes
- **THEN** it reports cancelled, and the target folder holds neither the target nor a temporary file
