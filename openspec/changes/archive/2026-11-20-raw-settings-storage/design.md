# Design: raw settings storage (#401)

## `crs:` mapping (`pictura-codec`, `pictura_raw/xmp.rs`)

One table, `CRS_KEYS`, pairs each `crs:` property with the `Fltr` key the
settings codec already uses and a value form:

| Form | Read | Write |
|---|---|---|
| number (signed) | `f64` → `DescValue::Double` | `+N` / `-N`, decimals per key (Exposure 2, radius / perspective 1, others 0) |
| bool | `True` / `False` → `Bool` | `True` / `False` |
| text | XML-unescaped → `Text` | XML-escaped, trailing NUL dropped |
| white balance | label ↔ `WBal` enum code (`As Shot` ↔ `AsSh`, … `Custom` ↔ `Cst `) | label |
| process version | `6.7` / `5.7` / `5.0` ↔ `PrVe` 2012 / 2010 / 2003 | string |
| curve | `rdf:Seq` of `"x, y"` items → `List(Long)` | element with `rdf:Seq` |

Reading builds `(key, DescValue)` items and feeds the existing
`decode_items`; writing takes `encode_items` and renders each item under its
`crs:` name. The `Fltr` key map stays the single source of value types.

`pictura_raw_to_xmp(existing, settings)` patches the packet one property at a
time, re-scanning after each edit (packets are a few KiB):

- a scalar that exists (attribute or element form) has its text replaced; an
  absent, set scalar is inserted as an attribute on the `rdf:Description` that
  carries the `crs` namespace; a modelled scalar the settings leave unset is
  removed (Camera Raw reads a missing key as its default);
- a curve element is replaced whole, removed, or inserted before that
  description's end tag (a self-closing description is opened first);
- when no description carries `crs`, one is added before `</rdf:RDF>`; a
  packet without `</rdf:RDF>` cannot be patched and the call returns `None`
  (the caller refuses rather than overwriting it);
- `crs:HasSettings="True"` is set on every write; a fresh packet also gets
  `crs:Version="7.0"` and the `<?xpacket?>` wrapper.

ponytail: like `crs_xmp.rs`, only the literal `crs:` prefix is matched, and the
description's end tag is the first `</rdf:Description>` after its start tag (a
nested description inside the `crs` one would misplace inserted curves).

`raw_settings_key(bytes)` is the lowercase hex SHA-256 of the whole file
(`sha2`, already a codec dependency).

## Stores (`pictura-app`, `open_jobs/raw_open/store.rs`)

```text
StoreKind { Database = 0, Sidecar = 1 }        // the preference, bridged as i32
RawStores { preferred, database: PathBuf }     // database = $XDG_DATA_HOME/kooka-pictura/Camera Raw
read_record(raw, key, stores) -> Option<Vec<u8>>   // preferred, other, (caller: embedded)
save_settings(pending, settings, stores) -> Result<Saved, String>
Saved { Sidecar, Database, DatabaseFallback(reason) }
```

- The sidecar is `<stem>.xmp`; an existing `<stem>.XMP` is read and updated
  in place instead.
- The write base is the destination's own record when it has one, else the
  packet the seed came from (so a Lightroom sidecar's unknown keys reach a
  database record), else a fresh packet.
- Files are written to a temporary name in the same folder and renamed, so a
  failed write never leaves a truncated record.
- The write derives the white-balance label from the values: As Shot when
  temperature and tint equal the rounded as-shot values, else Custom in place
  of an unset or As Shot preset (the dialog leaves the preset alone when a
  slider moves), so a reader does not recompute an as-shot balance over an
  edited one.

Read order puts the preferred store first. WF-012's pipeline lists the
database before the sidecar; with the sidecar preferred that would let a stale
database record (left by an earlier read-only fallback or the other
preference) hide newer sidecar edits, so the preference decides and WF-012's
open question on precedence is answered this way.

## Bridge and shell

- `open_raw_start(view, path, store)` gains the preference; the decode worker
  hashes the file, reads the record, and keeps `key` and the record bytes on
  `PendingRaw`.
- `raw_save_settings(view, settings, store) -> QString` writes on the GUI
  thread (a few KiB) and answers `sidecar`, `database`,
  `fallback:<reason>`, or `error:<reason>`. It lives in the existing
  `open_jobs.rs` bridge (the bridge-file budget).
- `PicturaRawDialog` in raw mode adds **Done** (`rawDoneButton`); it records
  the choice and rejects, and the coordinator reads `saveOnlyRequested()`.
- `ImportSaveCoordinator` holds `rawSettingsStore_`, saved in
  `SessionState::rawSettingsStore`; the Preferences File Handling page shows
  the combo (`preferencesRawSettingsStore`), wired in `frame_recovery.cpp`
  like Maximize Compatibility.
