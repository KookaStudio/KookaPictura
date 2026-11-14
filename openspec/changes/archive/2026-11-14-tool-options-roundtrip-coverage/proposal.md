# Proposal

## Why

Issue #712: the session store serializes 31 `toolOptions` groups (26 typed
option groups plus `brush`, `selection`, `ellipticalSelection`,
`quickSelection`, and `crop`) carrying about 256 fields, but
`tst_tool_options_persistence.cpp` asserts only a small sample of them. A
renamed key, a dropped `fields(...)` line, or a reader that ignores a field
would leave CI green while users silently lose settings.

The writer and reader share one per-group `fields` visitor, so any test that
only compares save against save can pass with a symmetric rename. The coverage
contract has to pin the key names and values independently, which is what this
change adds.

## What Changes

- Add data-driven Qt Test cases in
  `crates/pictura-app/cpp/tests/tst_tool_options_persistence.cpp` with one row
  per `toolOptions` key. Each row carries every persisted field of that group
  at a literal non-default value.
- Add meta-assertions over the table itself: the declared fields must equal the
  group's serialized field set, the declared row keys must equal the top-level
  keys `toolOptionsToJson(ToolController{})` emits, every value must differ
  from the same field's default, and the colour leaves (`fillColor`,
  `strokeColor`, `shieldColor`) must be canonical 8-digit ARGB hex. A table
  that omits a new field or group key, or
  asserts a default, then fails before any round-trip runs.
- Prove each key through the codec twice: apply the row to a fresh
  `ToolController`, serialize and compare the key's object, apply the
  serialization to a second controller, and compare again.
- Add one restart case: apply every row's values and the four top-level `crop*`
  values to a window, call `saveSession()`, open a new window, and compare the
  whole restored `toolOptions` object to the merged row object plus the four
  restored `CropOptions` getters. A scope guard restores the store to defaults
  so an early failure still leaves sibling test functions isolated.
- Add per-leaf wrong-type rows generated from a fresh controller's serialized
  payload (after checking the default payload round-trips), explicit
  malformed-colour rows, and a group-level table (`groupEntriesRestoreDefaults`)
  for a missing key, a non-object value, and `null` applied to a controller
  that already holds non-default values, asserting defaults. Also add a small
  explicit out-of-range table for the families whose `clampOptions` (or restore
  setters) clamp.
- Cross-reference the four top-level `crop*` session keys to
  `tst_crop_options::cropOptionsPersist` for the value read-back, strengthened
  to assert a non-default `cropClassicMode`; their startup restore is covered
  by the restart case. They are `SessionState` fields, not part of
  `toolOptions`.
- No production behaviour, API, or dependency change.

## Capabilities

No spec delta: this is a test-only change. The persistence requirement already
states that every tool's options survive a restart, are clamped on restore,
and fall back to defaults on bad entries; this change makes the test suite
prove that existing contract rather than altering it (`skip_specs`).

## Impact

- `crates/pictura-app/cpp/tests/tst_tool_options_persistence.cpp` grows by
  roughly 500 lines of data and helpers; it stays under the 1400-line test cap.
- No production source, CMake, engine, codec, or dependency change.
- No `docs/` change.

## Provenance

Test-only coverage for existing repository behaviour; no Adobe algorithm or
asset is involved.
