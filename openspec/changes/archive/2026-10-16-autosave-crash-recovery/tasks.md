# Tasks: autosave-crash-recovery

## 1. Codec

- [x] 1.1 Share the path-record encoder/decoder in `vector_mask.rs`.
- [x] 1.2 `path_resources.rs`: read 1025 / 2000-2997 into `work_path` / `saved_paths`; write them back, verbatim when unchanged.
- [x] 1.3 Unit tests plus the `path_resource_oracle` psd-tools oracle.

## 2. App

- [x] 2.1 `impl_core/recovery.rs` bridge: background `recovery_write`, `recovery_wait`, `recovery_open`.
- [x] 2.2 `recovery_store.*`: locked session directory, manifests, orphan scan, discard.
- [x] 2.3 `frame_recovery.cpp`: autosave timer, drop on save/close, clean shutdown, recovery prompt, `-Recovered` naming.
- [x] 2.4 File Handling preferences page and session persistence.
- [x] 2.5 Enable only for interactive launches (`main.cpp`).

## 3. Verification

- [x] 3.1 Qt Test `tst_recovery`; updated `tst_command_tree` and the `prefs_pages` self-test expectation for the new real page.
- [x] 3.2 `bash scripts/verify-fast.sh`.
