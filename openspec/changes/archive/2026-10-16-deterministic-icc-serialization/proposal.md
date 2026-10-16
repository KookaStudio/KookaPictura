# Proposal

## Why

Little CMS stamps the current wall-clock time into the ICC header at profile
creation (`cmsCreateProfilePlaceholder` calls `_cmsGetTime`), so two calls to
`Profile::to_icc()` on the same built-in profile produce different bytes whenever
they straddle a second boundary. Every test that byte-compares a freshly
serialized profile against a stored one is therefore time-flaky (issue #410).
Upstream rejects a creation-date setter by design (Little-CMS #71, Debian
#814883); the sanctioned fix is to patch the header bytes at the point they are
produced. Today each consumer works around this by hand: the app tests mask the
date through an `undated_icc` helper, and the `dump_adobe_rgb` example zeroes the
field itself.

## What Changes

- `pictura_color::Profile::to_icc()` zeroes the ICC header creation-date field
  (the 12-byte `dateTimeNumber` at bytes 24..35, ICC.1:2022 §7.2.8) before
  returning, so serializing a built-in profile is byte-deterministic across calls
  and processes. The existing panic-free empty fallback is preserved by guarding
  the write on a long-enough buffer.
- Add a regression test asserting a serialized built-in profile has a zeroed
  creation-date field and still reloads through `from_icc`, plus a check that
  repeated serialization is byte-identical.
- Delete the now-redundant consumer-side masking: the `undated_icc` helper and
  its uses in the `pictura-app` profile-history tests, and the manual date
  zeroing in the `dump_adobe_rgb` example.
- The three date-flaky `to_icc()` comparison sites in `pictura-codec`
  (`icc.rs`, `profile_assignment_oracle.rs`; four assertions) become
  deterministic without edits.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `color/color-management`: adds a requirement that ICC serialization is
  byte-deterministic — `to_icc` zeroes the serialized creation-date field instead
  of leaving the wall-clock time, so repeated serialization of a built-in profile
  yields identical bytes and the bytes reload and are accepted by external ICC
  readers.

## Impact

- **Code**: `crates/pictura-color/src/lib.rs` (`Profile::to_icc` and its unit
  tests); `crates/pictura-app/src/cxxqt_object/tests.rs` and `tests_impl.rs`
  (drop the masking helper); `crates/pictura-color/examples/dump_adobe_rgb.rs`
  (drop the manual zeroing).
- **Behavior**: `to_icc` zeroes the creation-date field unconditionally. In
  production only synthesized built-in profiles reach it — `assign_document_profile`
  is called exclusively with a built-in (`impl_core.rs` `builtin_profile`), and
  the PSD writer re-emits a preserved/loaded profile's stored resource-1039 bytes
  verbatim rather than round-tripping them through `to_icc`. A caller that
  serializes a parsed profile through `to_icc` would lose that profile's original
  creation date; no such caller exists today.
- **Fixtures**: `crates/pictura-codec/tests/fixtures/psd_icc_rgb.icc` is already
  committed with a zeroed date because the example masks it today. Moving the
  masking into `to_icc` and removing it from the example must regenerate the
  fixture unchanged.
- **Dependencies**: none.
- **API**: no signature change.
