# Crash recovery delta

## MODIFIED Requirements

### Requirement: Recovery session store

An interactive launch SHALL create its own recovery session directory under
`$XDG_STATE_HOME/kooka-pictura/recovery-v2/`, outside the legacy
`recovery/` scanner, held by a lock that is stale only
when its owning process is no longer running. A clean exit SHALL delete the
directory. The self-test, `--headless`, and `--control` runs SHALL NOT create
one. Discovery SHALL read both roots, retain legacy quarantine behavior, and
place both legacy and versioned quarantine under the existing `recovery/quarantine/`
directory, which the legacy scanner excludes; session names SHALL avoid collisions. Unsupported versions SHALL
remain intact and SHALL NOT be treated as empty sessions.

#### Scenario: A clean exit leaves nothing behind

- **WHEN** the `tst_recovery` test enables recovery, snapshots a modified document, and destroys the window
- **THEN** the session directory existed with its lock while the window lived, is gone afterwards, and no orphan remains

#### Scenario: A live session is not an orphan

- **WHEN** the `tst_recovery` test scans for orphans while another session directory's lock is held
- **THEN** neither the scanning session nor the locked one is reported, and the other session is reported once its lock is released



### Requirement: Autosave snapshots

With Automatically Save Recovery Information on, the app SHALL write a
PSD or PSB checkpoint when a document first becomes dirty and at the chosen
interval when committed changes need a new checkpoint. It SHALL reuse a
current durable checkpoint when no new changes need to be captured. A
checkpoint SHALL cover committed state without changing the user's file, the
live path, or the dirty state, and without blocking the UI on the encode.
When PSD limits would prevent a complete checkpoint, the app SHALL use PSB.

An unconfirmed publication SHALL retain or create recovery of the latest live
committed state, even if previously clean, until its protection obligation is
resolved. A successful durable save of the current state SHALL remove redundant
recovery data. A stale save SHALL retain the recovery data for newer edits.
Closing a document or turning recovery off SHALL stop the relevant recovery
writes and remove their data after in-flight work is resolved. A late worker
MUST NOT recreate that data.

#### Scenario: Only unsaved changes are snapshotted

- **WHEN** recovery runs on an unmodified document, after drawing a Work Path on it, and again with no further change
- **THEN** it writes nothing for the unmodified document, writes one checkpoint that includes the Work Path, and reuses it with no further changes

#### Scenario: Save and close drop the snapshot

- **WHEN** two modified documents have recovery data, the user durably saves the current state of one and closes the other with Discard during a recovery write
- **THEN** both documents' checkpoint, manifest, and journal are removed and late writes do not recreate them

#### Scenario: A large recovery checkpoint uses PSB [cr_psb_checkpoint]

- **WHEN** an edited document exceeds a PSD limit
- **THEN** its recovery checkpoint is a readable PSB, and recovery restores the edited document

## ADDED Requirements

### Requirement: Recovery journals committed document changes

For each recovery-enabled document, the app SHALL record the resulting
document state in commit order, covering the state that PSD or PSB checkpoints
can represent. Undo, redo, history jumps, and named-snapshot restores SHALL
record their resulting state when it changes that recoverable document.
Committed persisted session metadata, including Vanishing Point planes changed
without a history entry or content revision, SHALL be captured and replayed.
Refused edits and in-progress gestures and previews SHALL create no journal
records. Recovery SHALL yield a single initial history state with unsaved
work. It SHALL NOT rerun user commands, scripts, or actions.

The app SHALL publish recovery records from a worker. It SHALL schedule a sync
periodically while changes are pending, including during continuous editing,
and SHALL also sync at a bounded batch size. Edits can finish before the
records reach durable storage. The app SHALL report when a write fails or
protection falls behind, retaining the last durable state. Turning recovery off
SHALL stop both checkpointing and journaling for the live session.

#### Scenario: The journal captures committed results [cr_committed_results]

- **WHEN** the user commits edits including a paint stroke, a structural change, and a metadata change, navigates history, and starts an uncommitted gesture
- **THEN** recovery yields the last durably recorded document state without the gesture, and starts with a single recovered history state

#### Scenario: Metadata without a history entry survives recovery [cr_metadata_only]

- **WHEN** a user commits a Vanishing Point plane-only change with no history or content revision change and its recovery state becomes durable
- **THEN** crash recovery restores the changed planes and the recovered document is modified

#### Scenario: Continuous editing advances recovery [cr_batched_sync]

- **WHEN** committed edits keep arriving while the recovery worker follows its sync schedule
- **THEN** a batch becomes durable while editing continues, and a refused edit and a preview add no records

#### Scenario: Turning off recovery drains pending writes [cr_disable_journal]

- **WHEN** the user turns off recovery with a checkpoint and journal write pending
- **THEN** the app removes both after the old work is quiescent, and further edits produce no recovery files

#### Scenario: A recovery write failure retains the previous state [cr_write_degraded]

- **WHEN** an append or sync fails after a valid checkpoint and batch have landed
- **THEN** the app reports degraded protection, leaves the user document and file intact, and retains the previously durable recovery state

### Requirement: Recovery publishes checkpoints before retiring journal data

Each journal record SHALL identify its sequence and base and carry a bounded,
checksummed payload. Recovery SHALL replay only consecutive valid records
after the selected checkpoint, stopping at the first invalid, incomplete, or
out-of-order record. It SHALL offer the valid prefix when the tail is damaged
and report the partial recovery. Older checkpoint-only sessions SHALL still
be offered. A checkpoint replaced by a newer one SHALL remain recoverable
until the new checkpoint and its discovery metadata are durable. Records for
changes newer than the checkpoint SHALL survive rotation. The app SHALL retain
the old session's recoverable data until the new session has a durable
checkpoint of the replayed state. A newly created recovery root and session SHALL have each newly created ancestor
directory entry synced through an existing durable ancestor before any checkpoint
or transfer is acknowledged durable. Failure SHALL report degraded protection
and SHALL NOT retire the source chain. The existing retry and quarantine rules
SHALL apply to the checkpoint and its associated recovery files.

#### Scenario: Legacy scanner cannot remove a versioned orphan [cr_downgrade_isolation]

- **WHEN** a versioned orphan exists and an older build runs its unmodified legacy recovery scanner
- **THEN** the orphan and every chain file remain intact and a subsequent current build offers it, while legacy sessions remain discoverable

#### Scenario: New recovery directories become durable [cr_first_store_durable]

- **WHEN** the first checkpoint or a recovery transfer creates missing root or session directories and an injected sync failure occurs at each new ancestor entry
- **THEN** no affected checkpoint is acknowledged durable, any transfer source remains available, and a successful retry syncs all entries before acknowledging durability

#### Scenario: A corrupt or torn tail leaves a valid prefix [cr_replay_prefix]

- **WHEN** recovery encounters a corrupt, truncated, or out-of-order journal record after valid records
- **THEN** it restores the state from the valid prefix, reports the partial recovery, and does not apply later records

#### Scenario: Interrupted compaction retains a usable recovery chain [cr_checkpoint_commit]

- **WHEN** an interruption occurs before or after publishing a new checkpoint while newer edits are queued
- **THEN** recovery uses a complete checkpoint and the matching journal, and no previously durable newer edit is lost

#### Scenario: Legacy sessions still recover [cr_legacy_checkpoint]

- **WHEN** startup discovers a legacy PSD checkpoint and manifest with no journal
- **THEN** it offers the checkpoint and recovers it as an untitled, modified document

#### Scenario: Recovery transfer retains the only copy [cr_transfer_durable]

- **WHEN** a journal-backed document replays successfully but writing its new-session checkpoint fails
- **THEN** the old checkpoint and journal stay available for a subsequent recovery attempt

#### Scenario: A stale background save preserves later recovery [cr_save_race]

- **WHEN** revision A is saving, B is committed and durably journaled, A finishes, and the process is killed
- **THEN** the user file contains A and recovery offers B

#### Scenario: A recovered chain survives machine interruption [cr_power_loss]

- **WHEN** the machine loses power during checkpoint rotation or after a batch has been durably acknowledged
- **THEN** a complete checkpoint chain remains recoverable through the acknowledged batch
