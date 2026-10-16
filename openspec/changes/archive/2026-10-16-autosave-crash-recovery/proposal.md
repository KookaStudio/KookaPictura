# Proposal: autosave-crash-recovery

## Why

Issue #184: when Kooka Pictura crashes or is closed unexpectedly, everything
since the last save is lost. CS6 periodically writes recovery information and,
after a crash, reopens the work as "Recovered" documents
(`XC-012`, `docs/11-cross-cutting/crash-recovery-and-autosave.md`). The issue
also reports that saving to PSD and reopening loses the document's paths; the
codec never wrote the Work Path or saved paths, so a recovery snapshot would
have lost them too.

## What Changes

- **Path resources (codec).** A PSD/PSB save writes the Work Path as image
  resource 1025 and each saved path as 2000-2997 (named by the resource name),
  using the same 26-byte path records `vmsk` already encodes; a read derives
  `work_path` / `saved_paths` from them. An unedited file keeps its original
  path-resource bytes; the clipping-path resource (2999) stays verbatim.
- **Recovery store.** Each interactive instance owns a session directory under
  `$XDG_STATE_HOME/kooka-pictura/recovery/`, held by a `QLockFile`. A clean exit
  deletes it; a session whose owner is dead is an orphan.
- **Autosave.** A timer (CS6 Auto Save interval, default 10 minutes) writes a
  PSD snapshot of every document with unsaved changes made since its last
  snapshot, on a worker thread, with a small JSON manifest (tab name, original
  path). Saving or closing a document deletes its snapshot. Snapshots never
  touch the user's file.
- **Recovery.** At launch, orphaned documents are offered in a Recover /
  Discard / Later prompt. Recover reopens each as an untitled document with
  unsaved changes named `<name>-Recovered` (CS6 naming), snapshots it into the
  new session, then deletes the orphan.
- **Preferences.** Edit > Preferences > File Handling becomes a real page with
  CS6's "Automatically Save Recovery Information Every" checkbox and interval
  (5 / 10 / 15 / 30 minutes / 1 hour), persisted in the session store.
- The self-test, `--headless`, and `--control` runs never enable recovery.

## Capabilities

### New Capabilities

- `document/crash-recovery`: autosave snapshots, orphan detection, and
  recovery of documents after an unclean exit.
- `codec/psd-path-resources`: Work Path and saved-path image resources.

## Impact

- `pictura-codec`: new `path_resources.rs`; `vector_mask.rs` shares its path
  record encoder/decoder; reader and writer hook the resources.
- `pictura-app` (Rust): `impl_core/recovery.rs` bridge (background snapshot
  write, open as recovered); one state field.
- `pictura-app` (C++): `recovery_store.*`, `frame_recovery.cpp`, a File
  Handling preferences page, session keys `autoSaveRecovery` /
  `autoSaveMinutes`.
- No new dependencies.

## Provenance

Behaviour from `XC-012` and `docs/02-ui-ux/preferences.md` (File Handling);
path-resource layout from the Adobe PSD file-format specification's path
resource format, verified against `psd-tools`. Ceilings (`ponytail:`): no
journal, crash handler, or Background Save; work since the last snapshot is
lost (bounded by the interval); History is not recovered (CS6 does not either).
