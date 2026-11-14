# Design

## Context

See proposal.md for motivation.

- The frame loads `gpuCompute_` from the session before it probes the adapter,
  then computes `gpuCandidatesAvailable_ = gpuAvailable_ || gpuChainHasResetPath()`
  (`crates/pictura-app/cpp/frame.cpp:163-168`).
- The Preferences dialog is created lazily and seeded with the raw
  `gpuCompute_` before the enable flag (`frame_preferences.cpp:59-61`). It is
  never re-seeded after the first open.
- `saveSession()` writes `gpuCompute_` unchanged (`frame_session.cpp:231`), and
  `saveSession` can run from many UI paths before Preferences is opened.
- Every new view starts with `gpuCompute` true in Rust and is synced from
  `gpuCompute_` in `addDocument`.
- Safe mode is a distinct state: `backend_label` checks no-adapter first,
  user-disabled second, safe mode third
  (`crates/pictura-render/src/gpu/select.rs:238`). The self-test asserts the
  default is on (`crates/pictura-app/cpp/selftest.cpp:1340`).

## Goals / Non-Goals

**Goals:**

- Make the in-memory preference agree with the probe at the earliest point, so
  every consumer (views, Preferences, About, session save) sees the same value.
- Leave safe mode and the off-and-on cache reset untouched.
- Add regression coverage for the unchecked state and the saved false value.

**Non-Goals:**

- No session schema change, no new preference field, no command-tree change.
- No attempt to remember a true preference for a later launch that has a GPU.
- No change to the status-bar label logic or to the Rust compositor fallback.

## Decisions

1. **Normalize once at startup**, immediately after `gpuCandidatesAvailable_` is
   computed: when it is false, set `gpuCompute_ = false`.
   - Alternative: normalize inside `showPreferences`. Rejected because a window
     move, panel toggle, or any other early `saveSession()` would still persist
     true before Preferences is ever opened.
   - Alternative: compute an effective value in both `showPreferences` and
     `saveSession` while keeping `gpuCompute_` true. Rejected because the
     in-memory model would still contradict the requirement and the gate would
     live in two places.
2. **Gate on candidates, not on `gpuAvailable_`.** Safe mode has no adapter but
   does have a reset path. Forcing the preference off there would flip the label
   from `CPU (safe mode)` to `CPU` and turn the reset flow into an enable action;
   the existing spec requires the toggle to stay enabled in safe mode.
3. **Keep the checkbox/enablement logic; make the seed setter silent.** The
   dialog already reflects a false value as unchecked and the candidate flag as
   disabled, so no view-level policing of the model is added.
   `PreferencesDialog::setGpuCompute` gains a `QSignalBlocker` (consistent with
   `setTypeShowAsianOptions` and `setMemoryUsagePercent`) so the programmatic
   seed cannot emit `gpuComputeChanged`. Without it, a no-candidate frame seeds
   false and `tst_command_tree::preferencesPages` counts the programmatic
   `setGpuCompute(true)` as a user toggle.
4. **Do not save during construction.** `saveSession()` serializes geometry and
   panel layout, so calling it mid-construction would persist a partial frame.
   The value is re-normalized on every launch, and the next ordinary save
   (preference change, window move, quit) writes false.
5. **Update self-test check 70 to the candidate-gated contract.** `defaultOn`
   becomes `false` on a launch with no candidate. The check keeps asserting the
   toggle round-trip; only the default expectation changes, justified by the new
   spec scenario.
6. **Extend the existing Qt Test** `toggleIsDisabledOnlyWhenNoBackendExists`
   inside its current disabled branch: unchecked, the active view's
   `gpu_compute()` false, and `saveSessionNow()` recording false. This follows
   the suite's existing hardware-conditional style (`available || resetPath`).
7. **Record the safe-mode retention as a manual check.** No Linux path sets the
   reset flag, and the test suite cannot call `prepareGpuBackend()` without
   spawning probe children, so the enabled-and-checked safe-mode case is
   verified on Windows with a cached `crashed` backend.

## Risks / Trade-offs

- **Stored preference is overwritten false.** A launch without a usable GPU
  discards a stored true, so a later launch with a working GPU stays CPU until
  the user re-enables it. Acceptable: the session store is machine-local and the
  issue explicitly asks not to persist an unavailable backend.
- **No-GPU CI fails the self-test if check 70 is not updated in the same
  change.** Mitigated by listing it as an implementation task.
- **Candidates are computed once.** `addDocument` only re-reads the cached
  `gpu_available()` result, which cannot change after startup, so the startup
  gate is final. Enablement already uses the startup value; this change does not
  widen that behavior.
- **Early saves before normalization.** Normalization runs before the menus and
  panels are built, so no widget-driven save can precede it. A process killed
  before any save leaves true on disk; the next launch normalizes again.

## Migration Plan

No data migration. The session schema is unchanged; a stored false loads as
false under the existing loader. Rollback is a code revert; a false already
written can be re-enabled from Preferences on a machine with a candidate.
