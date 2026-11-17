## ADDED Requirements

### Requirement: Open steps

`File > Open` SHALL be a recordable, Adobe-verified command (`file.open`, event
`Opn `) whose step carries the opened file's absolute path as the `path`
parameter (descriptor key `null`). Opening a document through `File > Open` or
`Open Recent` while recording SHALL append one Open step after the document has
opened successfully; a cancelled or failed open SHALL NOT append a step. Playing
an Open step SHALL open the recorded path without the open-options dialogs,
wait for the open to finish, and make the opened document active; a missing or
unreadable file SHALL fail the step. An Open step SHALL round-trip through
`.atn` write and read with its path unchanged.

#### Scenario: Opening a file while recording appends an Open step [ae_open_record]
- **WHEN** recording is active and `a.png` is opened through `File > Open`
- **THEN** one `file.open` step whose `path` is the absolute path of `a.png` is appended

#### Scenario: Playing an Open step opens the file [ae_open_replay]
- **WHEN** an action containing an Open step for `a.png` is played
- **THEN** `a.png` opens as the active document before the next step runs

#### Scenario: A missing file fails the Open step [ae_open_missing]
- **WHEN** an Open step's recorded path no longer exists
- **THEN** playback stops and reports the Open step as failed

#### Scenario: An Open step round-trips through .atn [ae_open_atn]
- **WHEN** a set holding an Open step is written to `.atn` and read back
- **THEN** the step is `file.open` with the same `path`

### Requirement: Save As steps carry their format options

A `Save As` step recorded from a save in a format with an options dialog (JPEG,
PNG, BMP, TIFF, GIF, Photoshop EPS, DCS, Photoshop Raw, Targa, Cineon) SHALL
carry the options confirmed in that dialog as an `options` parameter, an object
with that format's fields. Playing a Save As step that carries `options` SHALL
save with those options and SHALL NOT change the session's remembered options
for the format. A Save As step without `options` (recorded before this change,
or in a format without an options dialog) SHALL save with the remembered
options. `options` SHALL round-trip through `.atn` write and read. PDF Save As
steps keep the remembered PDF settings in this release.

#### Scenario: Save As records the confirmed options [ae_saveas_record]
- **WHEN** recording is active and the document is saved as JPEG with quality 4
- **THEN** the appended Save As step's `options` holds quality 4

#### Scenario: Playback uses the step's options [ae_saveas_replay]
- **WHEN** the remembered JPEG quality is 12 and a Save As step carrying quality 4 is played
- **THEN** the written JPEG is encoded at quality 4 and the remembered quality stays 12

#### Scenario: A step without options uses the remembered options [ae_saveas_legacy]
- **WHEN** a Save As step with no `options` parameter is played
- **THEN** the file is written with the remembered options for its format

#### Scenario: Save As options round-trip through .atn [ae_saveas_atn]
- **WHEN** a set holding a Save As step with `options` is written to `.atn` and read back
- **THEN** the step carries equal `options`

### Requirement: Suppressed-dialog playback

The system SHALL support a suppressed-dialog playback mode, used by Batch, in
which every command step executes with its recorded values and opens no dialog,
whatever its dialog options. In this mode a Stop step whose allow-continue flag
is set SHALL continue without showing its message, and a Stop step without
allow-continue SHALL fail the playback with its message. Treating Stop steps
this way is an inferred app decision, because CS6 Help documents only that
batch steps use recorded values.

#### Scenario: Modal steps run with recorded values [ae_play_suppressed]
- **WHEN** a step whose dialog state is on is played in suppressed-dialog mode
- **THEN** the command executes with its recorded values and no dialog opens

#### Scenario: Stop steps do not wait in suppressed mode [ae_play_suppressed_stop]
- **WHEN** suppressed-dialog playback reaches a Stop step with allow-continue set, and then a Stop step without it
- **THEN** the first continues without a message and the second fails the playback with its message
