# Tasks: background-copy-unlock

## 1. Engine

- [x] 1.1 `release_background`; `duplicate_layer` / `duplicate_paths` / `layer_from_background` use it; unit tests.

## 2. App

- [x] 2.1 Layers panel: the Background's lock badge converts it; `clickLockBadgeForTest`.

## 3. Verification

- [x] 3.1 `tst_edit_shortcuts::backgroundCopyIsUnlockedAndTheLockBadgeUnlocks`.
- [x] 3.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
