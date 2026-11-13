## ADDED Requirements

### Requirement: The build lock wraps direct heavy commands

The repository SHALL provide `scripts/with-build-lock.sh -- <command...>` for
direct heavy commands (`cmake --build`, `cargo build`, `cargo nextest`,
`ctest`) that bypass the verification gates. It SHALL resolve the same
`PICTURA_VERIFY_LOCK` and `PICTURA_VERIFY_LOCK_TIMEOUT` settings as the gates,
acquire the lock when enabled, export `PICTURA_VERIFY_LOCK_HELD=1` only when it
owns the lock, run the command, propagate the command's exit code, and release
the lock after the command exits and on interrupt or termination. When the lock
is disabled it SHALL run the command without creating a lock. A verification
gate run under the wrapper SHALL recognize the held marker, SHALL NOT acquire
the lock again, and SHALL NOT release the wrapper's lock. This is an explicit
exception to the acquisition rule in `Opt-in serialization of local
verification runs`: while the wrapper holds the lock, the gate neither acquires
nor releases it. `AGENTS.md` and `DEVELOPING.md` SHALL name the wrapper for
direct heavy commands.

#### Scenario: Two wrapped commands queue [bl_wrap_queue]

- **WHEN** the lock is enabled and a second wrapper starts while the first
  wrapper's command still runs
- **THEN** the second wrapper reports the holder and its worktree, waits, and
  starts its command only after the first releases the lock

#### Scenario: A failing command keeps its exit code [bl_wrap_failure]

- **WHEN** the wrapped command exits with status 7
- **THEN** the wrapper exits with status 7 and leaves no lock directory behind

#### Scenario: Wrapping a gate does not deadlock [bl_wrap_nested]

- **WHEN** the wrapper holds the lock and runs a verification gate that sources
  the lock helper
- **THEN** the gate sees the held marker, does not acquire the lock again,
  completes, and the wrapper releases the lock

#### Scenario: The disabled lock runs the command without locking [bl_wrap_disabled]

- **WHEN** `PICTURA_VERIFY_LOCK` is absent or `0`
- **THEN** the wrapper runs the command, creates no lock directory, and
  propagates the command's exit code

#### Scenario: A terminated wrapper releases the lock [bl_wrap_signal]

- **WHEN** the wrapper is running a command and receives `INT` or `TERM`
- **THEN** it releases the lock it owns and does not leave the wrapped child
  running

#### Scenario: The wrapper is documented for direct commands [bl_wrap_docs]

- **WHEN** a reader opens the commands or worktree guidance in `AGENTS.md` and
  `DEVELOPING.md`
- **THEN** both show the wrapper with a direct `cmake`, `cargo`, or `ctest`
  command and state that the lock stays opt-in
