# Design

## Context

See `proposal.md` for motivation. The relevant mechanics:

- `crates/pictura-app/cpp/tool_options_json.cpp` defines one `fields(...)`
  visitor per option struct, shared by `Writer` and `Reader`. `eachGroup`
  names the 26 typed groups; `brush`, the three selection groups, and `crop`
  are handled separately. Every serialized group is a flat object of scalars
  (bool, number, string), so a field's JSON path is its key.
- `toolOptionsToJson` writes whichever groups a `ToolController` exposes;
  `applyToolOptionsJson` restores from a default-constructed group per key,
  clamping through `clampOptions` or the paint/selection/crop restore setters,
  and never notifies. `patchToolOptionsJson` merges into the current value and
  is out of scope here.
- Startup calls `applyToolOptionsJson` from `buildTools`
  (`frame_build.cpp`) with `SessionState::toolOptions`; `saveSession` writes
  `toolOptionsToJson` into the store (`frame_session.cpp`). Tests isolate the
  store through `ScopedStateHome` (temp `XDG_STATE_HOME`) and build windows
  with `test::makeMainWindow()`.
- The existing suite is 366 lines, registered once, and not sharded. Test
  files have a 1400-line cap.
- The four top-level `crop*` keys (`cropClassicMode`, `cropDeletePixels`,
  `cropRatio`, `cropGridOverlay`) live on `SessionState`, not in the
  `toolOptions` object. `tst_crop_options::cropOptionsPersist` owns their value
  read-back, but its `cropClassicMode` assertion currently reads the default;
  this change strengthens it and covers their startup restore in the new
  restart case.

## Goals / Non-Goals

**Goals:**

- One literal, reviewable oracle that pins every persisted key, every value,
  and the fact that each value reaches its own field.
- Cover the real store path once (save, new window, restore) without paying
  for a window pair per key.
- Bad-entry behaviour per option family, including clamping and the default
  fallback.
- Keep the suite fast, order-independent, and diagnosable from one failure
  line.

**Non-Goals:**

- No production change: the anonymous `fields` visitors stay private.
- No checked-in session fixture or schema asset; the data rows are the
  contract.
- No duplication of the four `crop*` value read-back owned by `tst_crop_options`;
  the restart case only checks their startup restore.
- No UI-widget assertions; the existing suite covers the options-bar wiring.
- No session migration or backward-compatibility guarantee for old stores.

## Decisions

### Literal per-key rows, not a fixpoint

Each data row names a `toolOptions` key and carries every persisted field of
that group at a literal non-default value. The test applies the row to a fresh
controller, serializes, and compares the key's object exactly; then applies
the serialization to a second controller and compares again.

Alternatives considered:

- **Save → restore → save fixpoint plus a key manifest.** Less test data, but a
  cross-field swap (for example a positional mistake in `paintFields`) is
  consistently wrong on both saves and stays invisible. The key manifest also
  only catches renames, not wrong values.
- **Hash of the sorted key paths.** Fails a rename but reports an opaque
  digest; localizing the failure costs more than the data it saves.
- **Expose `fields` to the test and drive it generically.** The test would
  share the exact list whose omission is the blind spot, and it needs a
  production seam.

### One row per group, all fields

31 rows rather than ~256 field rows: the failure already names the key, and a
wrong field value is visible in the JSON diff. Values are chosen inside each
clamp range and with canonical `QColor::HexArgb` spelling so a restore is
byte-for-byte stable.

### The table validates itself

Before any round-trip, a meta-test asserts:

- each row's declared field set equals the field set `toolOptionsToJson` emits
  for that key from a fresh controller (catches a dropped field or a rename on
  the writer side, and a forgotten table update when a new `fields` line lands);
- the declared row-key set equals the top-level keys
  `toolOptionsToJson(ToolController{})` emits, so a new `eachGroup` entry with
  no row fails;
- every declared value differs from the same field's default (catches a row
  that silently stops exercising its field);
- every colour leaf (`fillColor`, `strokeColor`, `shieldColor`) is already
  canonical 8-digit ARGB hex. The same list identifies colour fields in the
  wrong-type table.

The oracle is independent of the writer because the values are literal, so
the coverage check cannot be satisfied by a broken reader. A struct field that
was never added to its `fields` visitor remains invisible; C++ has no
reflection, so that stays a review responsibility.

### Two codec passes and one merged restart

Controller-only rows are milliseconds and give per-key diagnostics. The
restart case merges every row into one object, applies it to a window's
`ToolController`, also sets the four top-level `crop*` values (Classic on,
delete-pixels off, non-zero ratio and grid overlay), calls `saveSession()`,
opens a second window, and compares the whole restored `toolOptions` object to
the merged row object (so a new top-level key with no row fails) plus the four
`CropOptions` getters. One window pair proves the `frame_build`/`saveSession`
wiring without 31 boots. A scope guard resets the store to defaults even when a
`QVERIFY` returns early.

### Bad entries: generated wrong types and explicit tables

Before generating bad rows, the test asserts that applying the option defaults
and serializing reproduces the fresh payload exactly (the baseline idempotence
precondition).

A generator walks the fresh controller's serialized payload and, for each
field, replaces it in turn with a non-null value of a different JSON type.
Expected result is the payload with that field back at its default, except
colour leaves (`fillColor`, `strokeColor`, `shieldColor`): `take(QJsonValue, QColor&)` discards
the seeded default and yields an invalid colour for a bad value, so their
expectation is the empty string. `null` rows are exercised separately because
`Reader` skips them and keeps the default. A `shape.fillColor` wrong-type row
is omitted: its default already serializes as `""`, so the row cannot fail.

An explicit malformed-colour table (`"nope"`, `"#12345"`) seeds each colour
leaf with a valid non-default colour and then asserts the empty string. A
group-level table applies a missing key, a non-object value, and `null` per
group to a controller that already holds non-default values, and asserts each
group resets to its defaults (`groupEntriesRestoreDefaults`).

Clamping ranges live in code, not in the payload, so a small explicit table
covers one out-of-range row per clamped family (`type`, `shape`, `pen`,
`eyedropper`, `selection`, `healing`, `contentAwareMove`, `redEye`, `move`,
`brush`, `crop`). Unclamped families get wrong-type rows only. Generated
wrong-type and clamp rows use `newRow(key + "/" + field)`, because field names
repeat across groups.

### Crop keys stay with their owner

`tst_crop_options::cropOptionsPersist` remains the value read-back for the four
top-level `crop*` keys; this change strengthens it to set `cropClassicMode`
true and assert true before resetting (the current false assertion equals the
default). The merged restart case covers their startup restore through the
window's `CropOptions` getters, so the test plan names both suites.

## Risks / Trade-offs

- **Data drift when a field, group key, or name is added or renamed** → the
  coverage meta-assertion fails until the table is updated; the diff is the
  acknowledgement.
- **A value accidentally equal to the default** → the non-default
  meta-assertion fails with the key and field name.
- **Clamp or colour canonicalisation rewrites a value** → in-range literals and
  canonical hex make restores exact; any mismatch prints the compact JSON of
  the whole group.
- **Struct field missing from `fields`** → invisible to every serialization
  test; documented non-goal, caught only by review.
- **File-size cap** → the suite is expected around 900 lines; if it approaches
  the 1400-line test cap, move the static rows and helpers to a test-local
  `tool_options_sweep_data.h` and include it from the suite rather than adding
  an allowlist entry.
- **Order dependence through the shared temp store** → the merged restart case
  restores the store through a scope guard, so an early failure does not leak
  non-default values; controller-only cases use a stack `ToolController`.
