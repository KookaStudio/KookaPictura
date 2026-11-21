## ADDED Requirements

### Requirement: The local build lock uses a stable per-user path

The opt-in verification lock SHALL use the canonical per-user runtime
directory when available and a stable portable fallback otherwise. The path
SHALL be independent of the worktree, `XDG_RUNTIME_DIR`, and `TMPDIR`.
An explicit lock-root override MAY isolate test fixtures. The existing opt-in,
owner record, and queue behavior SHALL continue to apply.

#### Scenario: Private temporary directories do not split the lock [acq_lock]

- **WHEN** two opted-in verification runs for a user have different
  `XDG_RUNTIME_DIR` and `TMPDIR` values
- **THEN** both resolve the same lock path and the second run queues behind
  the first
