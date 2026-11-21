# Spec Delta

## MODIFIED Requirements

### Requirement: Raw settings are kept in a database or a sidecar by preference

Edit ▸ Preferences ▸ File Handling SHALL offer **Save Camera Raw Settings In**
with **Camera Raw Database** (the default) and **Sidecar ".XMP" Files**, kept
in the application preferences store. The database SHALL hold one XMP record per raw file, named by
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
