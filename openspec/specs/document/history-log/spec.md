# History Log

## Purpose

Give users an opt-in record of document editing and sessions that they can keep
in image metadata, a text file, or both.

## Requirements

### Requirement: History Log preferences control new records

General preferences SHALL offer a History Log toggle, destination, verbosity,
and a file chooser for Text File and Both. The initial values SHALL be Off,
Metadata, and Sessions Only. Settings SHALL persist across launches. Enabling a
destination that writes text SHALL require a chosen path. Canceling that choice
SHALL preserve the prior effective settings. Each event SHALL use the effective
settings at capture time. Turning History Log off SHALL stop new capture and
preserve previously collected history.

#### Scenario: Defaults, file choice, and persistence [history_log_preferences]

- **WHEN** the user opens General on a fresh profile, cancels a required file choice, then accepts a valid configuration and restarts
- **THEN** the initial defaults appear, cancellation leaves the effective settings intact, and the accepted configuration is restored

#### Scenario: Disabling History Log stops capture [history_log_off]

- **WHEN** the user enables Both, commits an edit, disables History Log, commits another edit, and saves
- **THEN** only the edit captured while enabled appears in either destination and any pre-existing metadata history is preserved

### Requirement: Verbosity reflects successful user activity

Sessions Only SHALL record application start and quit and successful document
open and close. Document records SHALL identify the document by display name or
basename. Concise SHALL additionally record one label per committed edit that
produces a History panel entry. Detailed SHALL additionally include the stable
command identity and available supported parameters when a committed edit has
them. Records SHALL use canonical labels and UTC timestamps. Failed attempts,
canceled gestures, and previews SHALL produce no edit entry. Capturing history
SHALL create no undo state. Undo navigation, clearing undo history, and
evicting old History panel states SHALL leave previously captured records intact.

#### Scenario: Lifecycle and edit detail follow the selected verbosity [history_log_verbosity]

- **WHEN** the same session opens a document, commits a canvas stroke and a parameterized command, closes the document, and quits at each verbosity level
- **THEN** Sessions Only records lifecycle activity, Concise also records each edit label once, and Detailed additionally records the supported command identity and parameters

#### Scenario: Only committed edits contribute entries [history_log_committed_edits]

- **WHEN** the user previews and cancels an edit, triggers a refused edit, commits a gesture, and later undoes it or clears the History panel
- **THEN** the committed gesture is the only edit record, it remains present, and recording itself added no undo states

### Requirement: Text History Log appends to the chosen file

Text File and Both SHALL append readable records to the chosen file, preserving
existing contents. Text File SHALL leave the document's XMP history intact
without adding entries to it. Both SHALL continue to collect metadata history
when its text destination fails. A text write failure SHALL be surfaced to the
user while editing and image saves remain available. Text writes SHALL use a
bounded nonblocking queue. Rejected, failed, or undrained rows SHALL be reported
by sequence ID and destination through the user notice path while available and
stderr after UI teardown. Notices MAY batch sequence ranges by destination,
provided every affected row remains identifiable and the notice queue remains
bounded. On orderly quit, document close rows SHALL precede the quit row. The
writer SHALL drain and flush accepted rows for at most two seconds, then report
remaining rows and permit exit.

#### Scenario: Destination determines where new history goes [history_log_destinations]

- **WHEN** a user commits edits and saves to each of Metadata, Text File, and Both, with a pre-existing text file and pre-existing XMP
- **THEN** new records go to the selected destinations, preserving the original text and XMP

#### Scenario: A failed text destination leaves the document saveable [history_log_text_failure]

- **WHEN** a Both destination becomes unwritable during editing
- **THEN** the user receives a text-log failure notice and can save the image with the new XMP history

#### Scenario: Text queue saturation and orderly shutdown are visible [history_log_text_queue]

- **WHEN** a stalled text writer fills its queue and editing continues, and another session closes a document and immediately quits
- **THEN** each rejected or undrained row is identified to the user or stderr, metadata capture remains available, and a healthy writer flushes the close and quit rows in order before exit

### Requirement: Metadata History Log round-trips through supported saves

For PSD, PSB, JPEG, and TIFF, Metadata and Both SHALL append the document's
captured records to the text property `photoshop:History` at save time.
Patching SHALL preserve existing history and unrelated XMP. Nonempty history
SHALL be visible in the Photoshop tab of File Info after reopening, including
JPEG APP1 and TIFF tag 700 data. Document metadata SHALL contain only lifecycle
and edit records known for that document at the save snapshot. Close and quit
SHALL be written to the selected text log when enabled and SHALL trigger no
extra image write.

A background save SHALL include exactly the history captured in its snapshot.
A failed save SHALL keep eligible entries for retry. A successful save SHALL
account only for included entries, and a copy SHALL leave the source document's
pending history eligible for a later save. If unrelated File Info properties
change during a successful save, the included rows SHALL be rebased onto the
newest packet before they are acknowledged. If that rebase fails, they SHALL
remain eligible and the omission SHALL be reported. An accepted explicit raw
File Info edit SHALL count as a History change only when its namespace-resolved
History value changes, including absent versus present. A change followed by
restoration during the save SHALL still count.
Such a concurrent change SHALL make the current History value win over the
saved prefix, while records captured after the save snapshot remain eligible.
An unrelated rewrite with equal History SHALL preserve the saved rows.
If a new history append cannot fit the target's XMP limit, or an existing packet
cannot be patched safely, the save SHALL preserve the existing encodable packet,
skip the new append, and report the omission. The omitted records SHALL remain
eligible within the bounded session history for another compatible destination.

#### Scenario: Supported native and raster files reopen with history [history_log_xmp_roundtrip]

- **WHEN** a document with existing XMP is edited, saved in each supported format, reopened, edited again, and saved again
- **THEN** File Info shows the prior history followed by each newly captured edit exactly once, and unrelated XMP survives

#### Scenario: Delayed, failed, and copy saves keep the right entries [history_log_save_snapshot]

- **WHEN** an edit commits during a delayed save and the user tries a failed save, a copy, and a later successful save of the original
- **THEN** each file reflects its own snapshot and all committed records appear once in the original's final history

#### Scenario: File Info edits during a save retain the intended history [history_log_file_info_conflict]

- **WHEN** a save is held in flight while File Info rewrites unrelated data with equal History, changes or removes History, or changes and restores it, and a later save follows
- **THEN** equal History preserves saved rows exactly once alongside unrelated edits, effective changes or removal make the current History value win over older saved rows, and later captured rows remain eligible

#### Scenario: An unsafe or oversized append does not lose the image [history_log_xmp_limits]

- **WHEN** adding history would overflow JPEG APP1 or require rewriting an unsupported XMP packet
- **THEN** an otherwise valid image save succeeds with the previous XMP intact, the omission is reported, and the eligible entries can be saved through a compatible destination

### Requirement: Save for Web applies its metadata choice to history

Save for Web JPEG, PNG-8, and PNG-24 SHALL include eligible metadata history
only when All or All Except Camera Info is selected. The export SHALL leave
pending history eligible for a subsequent document save. Copyright-only
metadata choices and None SHALL omit History Log. If appending pending history
would overflow JPEG APP1 for an ordinary or sliced full-XMP export, the export
SHALL preserve the existing encodable packet, omit the append, warn the user,
and leave the omitted records eligible for the source save.

#### Scenario: Web export follows the selected metadata level [history_log_web_metadata]

- **WHEN** a document with saved and pending metadata history is exported through Save for Web at each metadata level
- **THEN** eligible JPEG and PNG files include history only for the two full-XMP choices, and a later document save still includes each pending record once

#### Scenario: Web JPEG history overflow preserves export and source records [history_log_web_overflow]

- **WHEN** pending history takes a previously encodable full-XMP JPEG packet over the APP1 limit in ordinary and sliced exports
- **THEN** each export succeeds with the previous packet intact, reports the omission, and a later compatible source save includes each pending record once
