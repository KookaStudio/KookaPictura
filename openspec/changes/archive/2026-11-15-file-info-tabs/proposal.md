# Proposal

## Why

`WF-010` specifies the CS6 File Info categories, and `docs/dev/doc-code-parity-gaps.md` WF-7 records that the dialog still shows a fixed four-category set with a read-only Raw XMP tab (`file_info_dialog.cpp:73`). The codec also stops at nine fixed XMP properties, IFD0 plus the Exif sub-IFD, and six IPTC-Core fields, so the missing tabs have no data behind them and no way to edit it.

## What Changes

- File Info gains the categories **Description** (editable), **IPTC Extension** (editable simple, bag, lang-alt, and struct fields), **GPS Data** (read-only), **Origin** (editable), and **Photoshop** (read-only `photoshop:` / `xmp:` / `xmpMM:` properties). Camera Data, IPTC, and Raw Data remain.
- Raw Data becomes editable: the packet is validated for well-formedness, replaces image resource 1060, preserves every other resource and unparsed tail, and records one undo state with the other edits.
- The shared IPTC-Core synchronisation grows from six to fourteen IIM↔XMP fields, including Keywords as a repeated IIM dataset and a `dc:subject` bag, and Origin fields. Editing a value in any tab updates the same underlying property.
- `pictura-codec` gains a generic XMP property tree and path patcher that can read and patch simple elements, lang alternatives, bags, and one-level structs, adding any missing namespace declaration on the inserted element so unknown packet shapes still round-trip.
- `pictura-codec` decodes the EXIF GPS IFD (`0x8825`) into typed latitude, longitude, altitude, timestamp, date, and datum values.
- Metadata templates carry the expanded managed and shared field sets. Replace mode now clears the newly managed fields when an older template omits them; design.md records the compatibility note.
- Existing file-info self-test checks move to a new Qt Test suite; the metadata-template check moves its value assertions to the new editable rows, and the self-test budget shrinks.

Explicit ceilings, recorded in design.md:
- **Video Data, Audio Data, and DICOM tabs are not added.** `ROADMAP.md` "Not planned" declares video layers/timeline and DICOM out of scope by design; the app has no DICOM import and no video/audio layer kind.
- **Artwork Or Object Shown and the PLUS nested structs are read-only** in this change: their leaves are listed, but the nested table editors are deferred.
- **DICOM measurement scale remains #635.**

## Capabilities

### New Capabilities

- `codec/exif-gps`: decode the EXIF GPS sub-IFD into typed geolocation values, bounded and panic-free.
- `codec/xmp-structured-properties`: read an XMP packet into a generic property tree and patch simple, lang-alt, bag, and one-level struct properties by path while preserving every unknown byte.
- `codec/iptc-extension`: read and write the IPTC Extension (Iptc4xmpExt) schema fields the dialog exposes, with read-only listings for the nested artwork and PLUS structures.

### Modified Capabilities

- `codec/psd-file-info`: the dialog requirement changes from four fixed categories to the eight listed categories, the editable field sets, Raw packet editing, and the one-state/no-op/cancel rules.
- `codec/psd-xmp-metadata`: the fixed property set gains City, State, Country, Instructions, AuthorsPosition, CaptionWriter, WebStatement, and UsageTerms; the IIM synchronisation set grows to fourteen fields including list-valued Keywords; a raw-packet replacement operation is added.
- `codec/metadata-templates`: the managed property set and the shared IIM↔XMP synchronisation set expand with the new fields.

## Impact

- `crates/pictura-codec`: `xmp.rs` splits into `xmp/` modules; new `xmp/tree.rs`, `metadata/iptc_extension.rs`; `exif.rs`, `iptc.rs`, `metadata.rs`, `metadata/template.rs`, `lib.rs`.
- `crates/pictura-app`: existing `PictureView` row getters gain a section prefix and the apply method dispatches the new edits; no new `#[qinvokable]` declarations (the bridge file is at its allow-list ceiling). `file_info_dialog.{h,cpp}` is rebuilt around data-driven sections; `frame.cpp` wires the apply callback.
- Tests: new `tst_file_info.cpp`; Rust unit and `metadata_oracle.rs` coverage; extended `scripts/generate-fixtures.py` and regenerated `metadata.psd`.
- Docs/process: `ROADMAP.md` item and `docs/dev/doc-code-parity-gaps.md` WF-7; the migration retires self-test exit codes 456/457/459 and lowers `scripts/selftest-budget.txt`.
- No new dependencies. No `docs/` behavioural spec change beyond the parity register and roadmap.
