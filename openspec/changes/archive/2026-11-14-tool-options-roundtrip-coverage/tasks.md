# Tasks

## 1. Sweep data and self-checks

- [x] 1.1 Add the test-local sweep helpers to `tst_tool_options_persistence.cpp`: a `ToolController` defaults snapshot, a compact-JSON equality assertion that prints both objects on failure, and the canonical-colour check for `fillColor`, `strokeColor`, and `shieldColor`.
- [x] 1.2 Add the 31 literal non-default rows (26 typed groups, `brush`, three selection groups, `crop`) and the `roundTripTableIsComplete` data/meta test; the meta test also asserts the row-key set equals the top-level keys emitted by `toolOptionsToJson(ToolController{})`.
- [x] 1.3 Wire the rows into per-key Qt Test data (`QTest::addColumn<QString>`, `QTest::addColumn<QJsonObject>`).

## 2. Codec round-trip

- [x] 2.1 Implement `everyToolOptionsKeyRoundTrips`: apply each row to a fresh `ToolController`, assert the key's serialized object equals the row, apply that serialization to a second controller, and assert it again.

## 3. Store restart and bad entries

- [x] 3.1 Implement `everyToolOptionsKeySurvivesARestart`: merge every row, apply it to one window, also set the four top-level `crop*` values on that window (Classic on, delete-pixels off, non-zero ratio and grid overlay), call `saveSession()`, open a second window, compare the whole restored `toolOptions` object to the merged row object and the four `CropOptions` getters, and restore the store through a scope guard (or `cleanup()`) so an early failure resets it.
- [x] 3.2 Implement the generated `wrongTypeEntriesRestoreDefaults` rows from a fresh serialized payload, the `null` rows, and the malformed-colour rows (`"nope"`, `"#12345"`, seeded from a valid non-default colour); first assert that applying the option defaults and serializing reproduces the fresh payload, skip the `shape.fillColor` wrong-type row, and use `newRow(key + "/" + field)` tags.
- [x] 3.3 Implement `outOfRangeEntriesClamp` with one explicit out-of-range row per clamped family (`type`, `shape`, `pen`, `eyedropper`, `selection`, `healing`, `contentAwareMove`, `redEye`, `move`, `brush`, `crop`), using `newRow(key + "/" + field)` tags.
- [x] 3.4 Implement `groupEntriesRestoreDefaults`: after filling a controller with non-default values, apply a missing key, a non-object value, and `null` per group and assert each group resets to its defaults.
- [x] 3.5 Strengthen `tst_crop_options::cropOptionsPersist` to set `cropClassicMode` true, assert true, then reset; document in `test-plan.md` that the cross-reference owns the value read-back and the restart case owns startup restore.

## 4. Verification

- [x] 4.1 Build once (`cmake --build build --target tst_tool_options_persistence tst_crop_options`), then run both suites offscreen (`QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(tool_options_persistence|crop_options)' --output-on-failure`) and confirm the file stays under the 1400-line test cap (`scripts/check-file-size.sh`).
- [x] 4.2 Run `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, and `bash scripts/verify-fast.sh`; verify all pass. (The earlier load-induced 120 s timeouts in `tst_quick_mask`/`tst_selection_preview` were environmental; the rerun on a quieter machine passed every gate.)
- [x] 4.3 Archive the change with `openspec archive tool-options-roundtrip-coverage` in the same pull request so no in-flight change remains.
