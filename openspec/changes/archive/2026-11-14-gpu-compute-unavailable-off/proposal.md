# Proposal

## Why

Preferences > Performance shows **Use GPU Compute** checked but disabled when no
GPU candidate is usable, while the status bar already reports `CPU (no GPU)`.
The contradictory state also re-persists `gpuCompute = true` on every session
save, so a machine without a usable adapter keeps a preference that cannot be
honored.

## What Changes

- When the launch has no GPU candidate (no usable adapter and no safe-mode reset
  path), the effective `gpuCompute` preference becomes false: the checkbox is
  unchecked and disabled, and a session save records false.
- Safe mode is unchanged: a reset path keeps the toggle enabled and preserves
  the stored value, so turning it off and on still clears the cache.
- Update the self-test's default-on expectation to the candidate-gated contract.
- Extend the GPU backend Qt Test to assert the unchecked state and the persisted
  false value on a machine without a candidate.
- No new session field, no schema bump, and no command change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/gpu-compute-backend`: an unavailable GPU becomes an effective-off
  preference, covering the checkbox state and the saved value, while safe mode
  keeps the existing enabled reset path. The disable requirement now names the
  Preferences > Performance **Use GPU Compute** checkbox as the toggle instead
  of the removed command.

## Impact

- `crates/pictura-app/cpp/frame.cpp` (startup probe block).
- `crates/pictura-app/cpp/preferences_dialog.cpp` (silent programmatic seed).
- `crates/pictura-app/cpp/selftest.cpp` (check 70 expectation).
- `crates/pictura-app/cpp/tests/tst_gpu_backend.cpp` (toggle state coverage).
- `crates/pictura-app/cpp/tests/tst_command_tree.cpp` (preference-page signal
  count on a no-candidate machine).
- The session schema is unchanged and the existing `gpuCompute` field is reused.
- No dependency changes.
