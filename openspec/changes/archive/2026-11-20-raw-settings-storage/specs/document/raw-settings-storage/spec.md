## ADDED Requirements

### Requirement: Raw settings are kept in a database or a sidecar by preference

Edit ▸ Preferences ▸ File Handling SHALL offer **Save Camera Raw Settings In**
with **Camera Raw Database** (the default) and **Sidecar ".XMP" Files**, kept
in the session. The database SHALL hold one XMP record per raw file, named by
the file's content hash, under `$XDG_DATA_HOME/kooka-pictura/Camera Raw/` (or
`~/.local/share/…` when the variable is unset). The sidecar SHALL be
`<stem>.xmp` beside the raw file; an existing `<stem>.XMP` SHALL be used
instead. A record SHALL be written to a temporary file in the same folder and
renamed over the old one. A DNG SHALL never be modified.

#### Scenario: The preference persists [raw_store_preference]

- **WHEN** Sidecar ".XMP" Files is chosen in Preferences and the session is saved and loaded
- **THEN** the coordinator's store is Sidecar, and a fresh session's store is Camera Raw Database

#### Scenario: The database record follows the content [raw_store_database_by_content]

- **WHEN** settings with exposure 0.75 are saved to the database for `shot.dng`, and the same bytes are decoded from `moved.dng` in another folder
- **THEN** the record file is `<sha256>.xmp` in the database folder, no sidecar is written, the DNG's bytes are unchanged, and the seed of `moved.dng` has exposure 0.75

### Requirement: OK and Done write the settings; Cancel does not

In the Pictura Raw dialog of a raw open, OK SHALL write the dialog's settings
to the preferred store and then open the document. A **Done** button SHALL
write them and open nothing. Cancel, and an open that shows no dialog, SHALL
write nothing. When the record is not already in the store, the base SHALL be
the record the seed came from, so its unknown keys are kept. The write SHALL
record the white balance as As Shot when Temperature and Tint equal the rounded
as-shot values, and otherwise as Custom in place of an unset or As Shot preset
(the dialog does not change the preset when a slider moves), keeping any other
preset.

#### Scenario: OK writes a sidecar and opens [raw_store_ok_writes]

- **WHEN** the store is Sidecar and a DNG is opened interactively, the dialog's exposure set to +1.00, and OK pressed
- **THEN** `<stem>.xmp` holds `crs:Exposure2012="+1.00"` and `crs:WhiteBalance="As Shot"`, and one document opens

#### Scenario: Done writes and opens nothing [raw_store_done_writes]

- **WHEN** the store is Camera Raw Database and the dialog is closed with Done
- **THEN** no document opens and the database holds the file's record

#### Scenario: Cancel and non-interactive opens write nothing [raw_store_cancel_writes_nothing]

- **WHEN** one open is cancelled in the dialog and another runs without a dialog, with the store Sidecar
- **THEN** no sidecar exists and the database folder is empty

#### Scenario: A sidecar's unknown keys reach the database record [raw_store_keeps_seed_record]

- **WHEN** the store is Camera Raw Database, a sidecar holds `crs:Exposure2012="+1.00"` and an unknown `crs:LensProfileName`, and the settings are saved with exposure 0.5
- **THEN** the database record holds exposure 0.5 and the unknown property, and the sidecar is unchanged

#### Scenario: An edited white balance is recorded as Custom [raw_store_white_balance_label]

- **WHEN** settings with the As Shot preset, the as-shot tint, and a temperature 500 K above the as-shot value are saved
- **THEN** the record holds `crs:WhiteBalance="Custom"`

### Requirement: A sidecar that cannot be written falls back to the database

When the sidecar is preferred but cannot be written, the record SHALL go to the
database and the status bar SHALL say the settings were saved in the Camera Raw
database. When neither can be written, the status bar SHALL warn that the
settings were not saved, and OK SHALL still open the document.

#### Scenario: A read-only folder falls back [raw_store_read_only_fallback]

- **WHEN** the store is Sidecar and the raw file's folder is read-only
- **THEN** the save reports a fallback, no sidecar exists, and the database holds the record

#### Scenario: Neither store writable warns and still opens [raw_store_unwritable_warns]

- **WHEN** the store is Sidecar, the folder is read-only, the database path is a regular file, and OK is pressed
- **THEN** the status bar message contains "not saved" and the document opens
