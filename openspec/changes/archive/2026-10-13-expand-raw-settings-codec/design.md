# Design

## Context

See `proposal.md` — Why. Current state that shapes the approach:

- `crates/pictura-core/src/crs.rs` defines `CrsSettings` (11 optional `f64`) and
  `pub type PicturaRawSettings = CrsSettings`. `SmartObject.crs` and
  `crates/pictura-codec/src/crs_xmp.rs` use `CrsSettings` for the `crs:` XMP view.
- `crates/pictura-codec/src/pictura_raw.rs` (759 LOC) encodes those 11 controls, all as
  `Long` except `Ex12` as `Double`, and merges a re-attached filter over the preserved
  `Fltr` so unmodeled keys survive.
- `crates/pictura-adjust/src/pictura_raw.rs` renders the 11 Basic controls; a default
  settings value is a byte-identical no-op.
- A reference Photoshop camera-raw filter descriptor exposes 111 `Fltr` keys. Value
  types are verified: only `Ex12`, `ShpR`, `PerR`, `PerX`, `PerY` are doubles; `Crv `,
  `CrvR`, `CrvG`, `CrvB` are lists of integers; `CMod`, `CamP`, `CP_D` and the opaque
  XMP/placeholder keys are text; `CtoG` is a boolean; `WBal` and `Sett` are enums; every
  other modeled key is an integer.
- Color and grayscale HSL are mutually exclusive in the descriptor: `CtoG = true` writes
  `GM_*` (grayscale mix) and drops `HA_*`/`SA_*`/`LA_*`, `Strt`, `Vibr`, and the defringe
  keys; `CtoG = false` does the reverse.
- The committed `assets/test_with_smart_object02.psd` is synthetic and carries only the
  11 Basic keys plus `Dhze`; the authoritative Adobe files are gitignored under
  `reference-fixtures/` and are not redistributed.

## Goals / Non-Goals

**Goals:**
- A typed, per-tab Pictura Raw settings model that can represent every modeled control
  without collapsing "unset" into "default".
- A type-aware `Fltr` codec that round-trips every modeled key exactly and emits nothing
  for unset controls.
- A synthetic fixture that exercises every modeled key through a real PSD write/read.
- Keep the `crs:` XMP path and the public bridge signatures untouched.

**Non-Goals:**
- Rendering the new tabs (#394) — new controls decode/encode but do not affect
  `render_pictura_raw` output beyond the existing Basic pipeline.
- The tabbed dialog, live preview, and histogram (#395).
- Byte-parity with Adobe for arbitrary files; only the modeled key map is grounded.

## Decisions

### D1 — `PicturaRawSettings` owns per-tab sub-structs; `CrsSettings` stays

`PicturaRawSettings` becomes a struct of sub-structs (Basic, mode/white balance, tone
curve, detail, HSL + grayscale mix, split toning, lens, effects, calibration). `CrsSettings`
is unchanged and reused as the `basic` field.

- **Why**: the `crs:` XMP vocabulary (long names) and the `Fltr` vocabulary (short keys)
  are different surfaces; folding them couples the preserve-only CS6 path to the CC codec.
  Reusing `CrsSettings` as `basic` avoids duplicating the 11 fields and touches nothing in
  `crs_xmp.rs` or `smart_object.rs`.
- **Alternatives**: (a) delete `CrsSettings` and fold into one struct — rejected, forces
  the XMP path to change and its long-name map is different; (b) a fresh `Basic` struct —
  rejected as needless duplication.
- The requirement name "Pictura Raw settings alias" is now stale; renaming it is a
  mechanical follow-up, not part of this change (see Open Questions).

### D2 — Every control is `Option`; arrays and modes are typed

- Scalars: `Option<i32>` for integer keys, `Option<f64>` for the five double keys.
- Point curves: `Vec<i32>` holding the flattened input/output pairs, matching the wire
  list exactly (variable length; identity is `[0,0,255,255]`).
- HSL / grayscale: `[Option<i32>; 8]` per channel keyed by the `R O Y G A B P M` ranges.
- Enums (`WBal`, `Sett`): `Option<[u8; 4]>` holding the 4-byte enum code, tolerating any
  code and round-tripping it. Type id is written as the key's own id.
- Text (`CMod`, `CamP`, `CP_D`): `Option<String>` stored **verbatim**, including any
  trailing NUL the descriptor carries, so re-encoding is byte-identical. Display code
  trims the terminator; the codec does not.
- **Why**: an `Option` per control keeps default-vs-unset distinguishable, which is what
  makes "default settings emit nothing" and the byte-identical no-op hold.

### D3 — Type-aware, per-tab codec in a split module

Decode and encode are written per tab against a small `kind` distinction (int, double,
list, text, bool, enum). `crates/pictura-codec/src/pictura_raw.rs` becomes a module
directory: `pictura_raw/mod.rs` (attach/merge), `pictura_raw/codec.rs` (the key map), and
`pictura_raw/tests.rs` (tests, so they fall under the test file cap).

- **Why**: the file is already 759 LOC against a 1200 cap; ~95 keys of explicit code plus
  tests would exceed it. Explicit per-tab functions stay greppable; a macro/data table
  was considered and rejected as cleverer than the code it saves.
- **Only `Some`/non-empty controls emit a key**, so the encoder's output for default
  settings is an empty `Fltr` object and re-attaching default settings leaves the
  preserved descriptor byte-identical.

### D4 — Preserve the opaque / later-CC keys

`Dhze`, `Upri`, `GuUr`, `Rtch`, `REye`, `LCs `, `AuCA`, `TMMs` are not modeled. The
existing merge path already preserves any unmodeled `Fltr` key; no codec change is needed
for them, only a regression test.

### D5 — Regenerate the synthetic fixture from scratch

`examples/make_smart_fixtures.rs` is extended to populate every modeled key with values
we author, and `assets/test_with_smart_object02.psd` is regenerated by running the
example. No bytes or values are copied from `reference-fixtures/`.

- **Why**: acceptance is "all modeled keys round-trip on the fixture"; the current
  synthetic fixture cannot exercise keys it does not contain. The reference files must
  stay out of the repository (provenance).
- The generator stays deterministic, so the committed binary is reproducible.

### D6 — Keep the bridge and renderer at Basic for now

`apply_pictura_raw_filter(path, <11 values>)` and `layer_pictura_raw_settings` keep their
signatures, constructing/reading `settings.basic`. `render_pictura_raw` reads
`settings.basic` and ignores the new tabs, preserving its default no-op test. #395 widens
the dialog and #394 the renderer.

## Risks / Trade-offs

- **[Value types are grounded on a later-CC file, not the earliest CC]** → the modeled
  subset is the PV2012 key set; later-CC keys are preserved. If an earliest-CC fixture
  appears, its types can be diffed against this map.
- **[Fixture values are authored by us, so round-trip proves self-consistency, not Adobe
  interop]** → accepted and stated in the spec; the key names/types are the verified part.
- **[Breaking type change touches ~30 call sites]** → mechanical; `cargo clippy` and the
  existing tests catch any missed site. The bridge signatures stay stable to limit churn.
- **[Curve lists have variable length]** → stored flattened and round-tripped as-is;
  rendering (#394) will define how to interpret short/identity curves.
- **[Oversized `Fltr` after adding ~95 modeled keys]** → only set controls emit, so the
  synthetic fixture and a default filter stay small.

## Migration Plan

1. Add the settings model in `pictura-core` and re-export it; keep `CrsSettings`.
2. Split the codec module and implement the type-aware key map; move tests.
3. Regenerate the synthetic fixture; add round-trip and preserve tests.
4. Update call sites (`pictura-adjust`, `pictura-render`, `pictura-app`, codec tests) to
   compile against the nested type; confirm the default no-op test still passes.
5. Correct `docs/dev/camera-raw-cc-notes.md` §Fltr key map (commit carries
   `TASK-ALLOWS-DOCS`).

Rollback: the change is confined to the Pictura Raw settings/codec; reverting the commit
restores the alias and 11-key codec. The regenerated fixture is regenerated by the example.

## Open Questions

- The semantics of `AuCA` and `TMMs` are unknown (both observed `0`); they stay
  preserve-only until a consumer needs them.
- Whether to rename the "Pictura Raw settings alias" requirement to a name that matches
  its new behavior — a cosmetic follow-up.
- Whether `Sett` and `WBal` warrant named enum variants beyond the raw 4-byte code.
