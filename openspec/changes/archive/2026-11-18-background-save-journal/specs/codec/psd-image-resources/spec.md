# PSD image resources delta

## ADDED Requirements

### Requirement: Recovery checkpoints carry autosave resources

New Pictura recovery checkpoints SHALL contain exactly one Auto Save File
Path resource (1086) and one Auto Save Format resource (1087). The path SHALL
name the checkpoint's final path and the format SHALL identify its actual PSD
or PSB container. Both values SHALL use PSD Unicode strings. Checkpoint
generation SHALL leave all other parseable resource records intact subject to the
existing writer's normalization rules; an unparsed suffix SHALL be preserved in
the recovery chain rather than placed before generated records in the checkpoint.

Ordinary native saves SHALL preserve imported 1086 and 1087 blocks verbatim
and SHALL NOT generate new ones. Recovery SHALL discover checkpoints through
the session store and SHALL NOT use these values as a path or format authority.
The recovery chain SHALL durably preserve the original complete raw image-resource
section, including imported 1086/1087 records, their order, duplicates, bytes,
and any unparsed suffix. This bounded, validated preservation data SHALL be
published and retired with its checkpoint. Recovery SHALL replace the entire
checkpoint resource section with the original bytes before replaying changes;
it SHALL NOT subsequently strip imported records by ID. If preservation data
cannot be read or validated, recovery SHALL retain the chain and report failure
without exposing an incomplete document or retiring its source. A later user
save SHALL NOT publish the generated local recovery path.

#### Scenario: Generated resources identify the checkpoint [ir_recovery_resources]

- **WHEN** Pictura writes PSD and PSB checkpoints to paths containing non-ASCII characters with existing 1086 and 1087 blocks present
- **THEN** each output has one Unicode path naming its final checkpoint and one Unicode format matching the container, and unrelated blocks keep their existing bytes

#### Scenario: Ordinary saves preserve imported autosave blocks [ir_foreign_recovery_resources]

- **WHEN** ordinary saves round-trip foreign PSD and PSB files with and without 1086 and 1087
- **THEN** existing blocks survive byte-for-byte and absent blocks remain absent

#### Scenario: The recovery manifest controls file discovery [ir_recovery_discovery]

- **WHEN** a checkpoint's stored 1086 and 1087 values disagree with its manifest and actual container, and the recovered document is then saved to a user path
- **THEN** recovery uses the manifest and actual file and the user output contains neither generated block

#### Scenario: Imported resource bytes survive recovery [ir_recovery_preserves_imports]

- **WHEN** an editable document with ordered duplicate imported 1086/1087 blocks is checkpointed, recovered after a crash, and saved normally
- **THEN** the imported blocks retain their order, duplicates, and bytes, and no generated local recovery path leaks into the save

#### Scenario: Recovery preserves an unparsed resource suffix [ir_recovery_unparsed]

- **WHEN** a recovery chain contains an unparsed raw resource suffix
- **THEN** recovery restores the complete original resource section byte-for-byte and the native document version policy leaves the recovered document inspectable read-only
