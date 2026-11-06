# Verification — #1200

## Executed checks

- Production CMake build: passed, with 12 of 16 CPUs (75%). The initial
  sandboxed attempt completed Rust and was blocked by sccache access; the
  permitted incremental retry completed C++. A later test-only rebuild
  corrected the image reference described below.
- Renderer: 31 selected mask tests passed, including all new creation and
  polarity checks. The 1520 other tests were excluded by the targeted filter.
- Codec: all 4 mask-link integration tests passed. The independent psd-tools
  checks executed for both link states and both PSD/PSB outputs; none skipped.
- Qt: `tst_mask_link_state` passed all 8 reported cases, with zero skips.
  Existing `tst_layers_panel` (28), `tst_layers_mask_commands` (19), and
  `tst_linked_layers` (16) also passed with zero skips.
- Dependency gate: `cargo deny check` passed advisories, bans, licenses, and
  sources. No dependencies were added.
- Test-plan checker: passed before archive; the archived plan was then checked
  directly with the same checker and every named test was found.
- Spec sync: all three capability deltas were applied and compared against
  their main specs; strict main-spec validation passed.
- Full verification: `verify-fast.sh` passed with the isolated Cargo target.
  The unified report contains 7996 passed, 124 skipped, and 0 failed checks:
  5059 Rust tests passed, the Qt layer reported 2430 passed and 2 skipped
  across 245 registered entries, and the application self-tests passed.
  Workspace Clippy reported zero errors. All final checks passed: file-size,
  flatpak, worktree hook, build caching/lock, self-test budget, archive,
  icon provenance, guard, test plan, and strict OpenSpec validation.

## Test-reference correction

The first Qt run exposed an incorrect assumption in the new reference image:
transparent corners of the oversized glyph rectangle overlap neighbouring
thumbnail pixels, so they are not uniformly the row background. The corrected
check compares every visible reference-ink pixel at the specified opacity and
compares all pixels outside the union of both glyphs' ink before and after a
link toggle. It passes at DPR 1 and 2 without increasing the rounding tolerance.
No golden image or pre-existing fixture was changed.

## Environment limitation

The first full-gate attempt stopped in the existing shared Cargo target's
cxx-qt build script. That cache contains generated-header symlinks to other
worktrees, including a removed worktree. The isolated retry uses
`CARGO_TARGET_DIR=build/verify-target` instead of modifying the shared cache.

## Manual reviewer checklist

- [ ] Open `/mnt/work/PhotoshopRequest/2026.10/_auxien/20261009_01.psd` in
  Photoshop and Pictura, save a separate copy from Pictura, and reopen the copy
  in Photoshop. Confirm the user-verified linked Smart Object mask remains
  linked and in the same position. The original file must stay untouched.

This Photoshop application check was not performed by the Linux test harness.
Both supplied documents were inspected with psd-tools during exploration and
contain bit-0-clear raster masks; neither supplies an independently verified
Photoshop-unlinked mask. The committed tiny fixtures are independently authored
with psd-tools, not extracted from the user's artwork.

## Compatibility

Older Pictura saves can encode the opposite of their intended mask link state.
The correction follows the stored PSD bit without guessing file provenance or
rewriting imported flags. Such a file may need an explicit link/unlink edit.
