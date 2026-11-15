# Design

## Context

See `proposal.md` — Why. The implementation constraints that shape this design:

- The File Info dialog is fed by five `PictureView` getters and one apply method (`crates/pictura-app/src/cxxqt_object/file_info.rs`). Adding a method costs a declaration in `crates/pictura-app/src/cxxqt_object.rs`, which is at its 1204-line allow-list entry, so no new `#[qinvokable]` may be added.
- `crates/pictura-codec/src/xmp.rs` is 1190 lines against a 1200-line cap, so it must be split before the property set grows.
- The current scanner records spans only for nine managed fields; arbitrary RDF is preserved but not parsed. IPTC Extension and the Photoshop tab need a generic reader.
- The writer's namespace insertion fails closed when the receiving `rdf:Description` does not declare the namespace. IPTC Extension documents often lack that declaration.
- The self-test suite owns the current File Info checks, including the metadata-template check 460 that reads `xmp_rows()`; new GUI coverage belongs in Qt Test (`AGENTS.md` rule 11).
- `ROADMAP.md:639-650` records video layers/timeline and DICOM as not planned. No DICOM import or video/audio layer kind exists.

## Goals / Non-Goals

**Goals:**

- Present the eight in-scope categories with real data and edit every field whose data model exists.
- Keep one write path and one undo state per dialog accept, and refuse a group whose own write cannot be completed.
- Preserve unknown XMP bytes and every unrelated image resource on every write.
- Keep the bridge and codec files under their size caps by generalising existing seams rather than adding new ones.

**Non-Goals:**

- DICOM import or measurement scale (#635), video/audio layer kinds, the timeline.
- Editing Artwork Or Object Shown and the PLUS nested structures; they are listed read-only.
- A general-purpose XMP toolkit: only the listed shapes are writable.
- Live synchronisation between categories while the dialog is open beyond mirroring shared field values; the document is unchanged until OK.

## Decisions

### D1 — Data-driven dialog over section-prefixed rows, no new bridge methods

The three existing read-only getters and the editable-field getter carry a section in every row:

```
read-only:  "Section\tLabel\tValue"
editable:   "Section\tid\tLabel\tValue"
apply:      "id\tValue"                    (repeated ids = list values)
raw:        "xmp-packet\t<whole packet>"
```

`frame.cpp` concatenates `exif_rows`, `iptc_rows`, and `xmp_rows` and passes the rows, the editable fields, and the raw packet to the dialog. The dialog owns the canonical category order (Description, Camera Data, IPTC, IPTC Extension, GPS Data, Origin, Photoshop, Raw Data) and pulls each page's rows by section. `iptc_edit_fields` is renamed `file_info_edit_fields`; the declaration stays on one line, so the bridge file does not grow.

The id families are:

- IIM-backed shared fields: `2:5`, `2:80`, `2:25`, etc.
- XMP-only fields: `xmpRights:Marked`, `xmpRights:WebStatement`, `xmpRights:UsageTerms`.
- IPTC Extension fields: `Iptc4xmpExt:PersonInImage`, `Iptc4xmpExt:LocationCreated/Iptc4xmpExt:City`, etc.
- Raw packet: the single sentinel `xmp-packet`.

Alternative considered: one new getter per tab. Rejected: seven declarations would break the bridge ceiling and duplicate the row grammar.

### D2 — Split `xmp.rs` along scanner / patcher / serialiser seams

`xmp/mod.rs` keeps `XmpField`, `XmpProperties`, `parse_xmp`, and the re-exports. `xmp/scan.rs` owns `tokenize` and the typed scan; `xmp/patch.rs` owns span replacement and insertion; `xmp/serialize.rs` stays; `xmp/tree.rs` is new. Pure moves first, then the feature changes, so a regression bisects cleanly and the file-size gate stays green. Helpers shared with `xmp/tree.rs` become `pub(super)`.

### D3 — Generic property tree plus a path patcher; keep the typed patcher

`xmp/tree.rs` reads the token stream with namespace scope tracking and returns leaves with a path of `(namespace URI, local name)` segments, a value, and a form (attribute, simple element, lang item, bag/sequence item, struct leaf). It reuses `tokenize`, `push_declarations`, `split_qname`, `str_at`, and `decode_text`.

`xmp/tree.rs` also patches by path. It collects `(start, end, replacement)` edits, rejects overlaps, and fails closed on shapes it cannot rewrite. An edit that already matches the current value is a no-op. For a missing property it inserts a complete element; the inserted element carries any namespace declaration not already in scope:

```xml
<Iptc4xmpExt:PersonInImage xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/">
  <rdf:Bag><rdf:li>Ada</rdf:li></rdf:Bag>
</Iptc4xmpExt:PersonInImage>
```

Declaring namespaces on the inserted element avoids rewriting the `rdf:Description` start tag and therefore avoids overlapping an attribute edit. A canonical prefix table supplies the prefix; when the prefix is already bound to a different URI, a generated prefix is used.

A missing lang alternative is created as `<ns:Prop><rdf:Alt><rdf:li xml:lang="x-default">value</rdf:li></rdf:Alt></ns:Prop>`. Struct creation writes child elements inside a nested `rdf:Description`; an existing attribute-form struct is extended by rewriting its self-closing inner description. Existing leaf edits replace only the text span (or attribute value span); bag edits rewrite the bag's item list; removal deletes the property span.

Alternative considered: extend the typed span model for every extension field. Rejected: each new schema field would need its own span logic, and Artwork/Photoshop would still need a generic reader.

### D4 — IPTC Extension is a schema table, XMP-only, with read-only leftovers

`metadata/iptc_extension.rs` uses the namespace
`http://iptc.org/std/Iptc4xmpExt/2008-02-29/` and these descriptors:

| UI field | Property | Shape |
|---|---|---|
| Person Shown | `Iptc4xmpExt:PersonInImage` | bag of text |
| Organisation Shown (Name) | `Iptc4xmpExt:OrganisationInImageName` | bag of text |
| Organisation Shown (Code) | `Iptc4xmpExt:OrganisationInImageCode` | bag of text |
| Event | `Iptc4xmpExt:Event` | lang alternative |
| Location Created | `Iptc4xmpExt:LocationCreated` | struct: `City`, `ProvinceState`, `CountryName`, `CountryCode`, `Sublocation`, `WorldRegion` |
| Location Shown | `Iptc4xmpExt:LocationShown` | struct: same leaves |
| Rights Usage Terms | `xmpRights:UsageTerms` | lang alternative |
| Image Rating | `Iptc4xmpExt:ImageRating` | integer 1..=5 |
| Model Release Status | `Iptc4xmpExt:ModelReleaseStatus` | text |
| Property Release Status | `Iptc4xmpExt:PropertyReleaseStatus` | text |
| Digital Source Type | `Iptc4xmpExt:DigitalSourceType` | text |
| Additional Model Information | `Iptc4xmpExt:AddlModelInfo` | text |
| Licensor | `Iptc4xmpExt:Licensor` | struct: `LicensorName`, `LicensorID`, `LicensorEmail`, `LicensorPhone`, `LicensorCity`, `LicensorState`, `LicensorCountry`, `LicensorPostalCode`, `LicensorStreetAddress`, `LicensorWebUrl` |
| Artwork Or Object Shown, PLUS structs, anything else in the namespace | — | read-only leaves |

The descriptor drives both the edit rows and the apply path. Edits go to XMP only because IPTC Extension has no IIM channel. Supported edit rows are emitted for every descriptor with an empty value when the property is absent, so the control can add it; read rows list only values present in the packet. When a packet carries more than one `LocationShown` or `Licensor` structure, the first is editable and the rest are read-only rows.

Alternative considered: implement the artifact and PLUS nested table editors. Rejected for this change: their UI is a nested table with add/remove rows, and the leaves are preserved and visible as a first step.

### D5 — GPS is a dedicated walk of `0x8825`

`parse_exif` follows `0x8825` after IFD0 and the Exif sub-IFD and decodes the GPS tags with array awareness: latitude/longitude refs plus three rationals, altitude ref plus rational, three-rational timestamp, ASCII date, and ASCII datum. `Exif` gains an optional typed `Gps`. The generic entry list stays unchanged, so no existing `get(tag)` result changes. The shared entry/value budget bounds the GPS walk. A malformed GPS IFD yields no GPS, not an error.

Alternative considered: expose GPS tags as `Undefined` entries and format in the app. Rejected: the app would then own TIFF array decoding.

### D6 — Expanded typed property set, fourteen-field sync, list-aware Keywords

`XmpField` gains City, State, Country, Instructions, AuthorsPosition, CaptionWriter, WebStatement, and UsageTerms. The typed parser, patcher, serialiser, and template `MANAGED` table follow the existing patterns: `rdf:Alt` for Title/Description/Rights/UsageTerms, `rdf:Seq` for Creator, `rdf:Bag` for Subject, a simple element for the `photoshop:` properties, and `xmpRights:Marked`/`xmpRights:WebStatement` as simple elements.

The fourteen shared IIM↔XMP fields are:

| IIM | XMP property |
|---|---|
| `2:5` Object Name | `dc:title` |
| `2:80` By-line | `dc:creator` |
| `2:85` By-line Title | `photoshop:AuthorsPosition` |
| `2:25` Keywords | `dc:subject` |
| `2:116` Copyright Notice | `dc:rights` |
| `2:120` Caption/Abstract | `dc:description` |
| `2:110` Credit | `photoshop:Credit` |
| `2:115` Source | `photoshop:Source` |
| `2:105` Headline | `photoshop:Headline` |
| `2:40` Instructions | `photoshop:Instructions` |
| `2:90` City | `photoshop:City` |
| `2:95` State/Province | `photoshop:State` |
| `2:101` Country | `photoshop:Country` |
| `2:122` Writer/Editor | `photoshop:CaptionWriter` |

`set_file_info_fields` keeps its scalar `(u8, u8, String)` form. A companion
`set_file_info_values(&mut Document, &[(u8, u8, Vec<String>)])` handles list-valued fields. `Iptc` gains `values`/`set_values` for repeated datasets: Keywords (`2:25`) writes one dataset per value and one `dc:subject` bag item per value; an empty list removes every occurrence. The XMP gate compares the full list, so an unchanged list is a no-op. `metadata/template.rs` routes Subject through the list form.

Alternative considered: keep Keywords XMP-only or comma-separated. Rejected: CS6 keeps the channels equal, and a single IIM record would lose all but one keyword.

### D7 — Raw packet replacement, no-change semantics, and all-or-nothing apply

`metadata::set_xmp_packet` validates with the scanner's well-formedness and size flags, frames resource 1060, preserves every other resource and the unparsed tail, and treats empty text as removal. It returns false before mutating on an invalid packet.

The apply path handles each group before mutating it: the raw setter validates well-formedness and size, and the extension setter builds the whole packet before committing, so a refused group leaves its resource unchanged. On success the order is raw packet, shared IIM/XMP fields, XMP-only fields, extension fields, so field edits patch the new packet. A refusal aborts the remaining groups and returns false; the dialog warns and stays open. Earlier groups that already applied remain (no cross-group rollback); the shared/XMP setters are per-resource atomic.

The dialog calls an `onApply` callback only when `edits()` is non-empty. An empty OK closes without applying and records no state. A non-empty apply that returns false shows a warning and keeps the dialog open. One `Edit::Meta` labelled `File Info` records the whole accepted change.

Alternative considered: write fields first and raw last. Rejected: the raw packet would clobber every field edit.

### D8 — Shared ids mirror edits across categories

The dialog keys editable widgets by the canonical id (`2:5`, `photo:AuthorsPosition`, `Iptc4xmpExt:PersonInImage`) and keeps a list of widgets per id. A change to one mirrors the text to the others. `edits()` emits one row per changed id (repeated for bag items) and the raw packet row only when the packet changed. This matches CS6, where Object Name and Document Title are the same property shown in two tabs.

### D9 — Qt Test owns the new GUI coverage; the self-test shrinks and check 460 migrates

New coverage lives in `crates/pictura-app/cpp/tests/tst_file_info.cpp`: category order, missing values still editable, shared-field mirroring, GPS and Photoshop rows, extension bag/struct editing, raw edit, invalid raw keeps the dialog open, cancel, and no-op. The three File Info checks in `selftest_layers_adjustments.cpp` (exit codes 456/457/459) are retired with their code; `scripts/selftest-budget.txt` drops from 506 to 503.

The metadata-template check 460 stays, but both its dialog construction and its `xmp_rows()` assertions migrate: the rebuilt dialog keeps the Export/Apply Template controls and the merge-mode combo, and check 460's value assertions switch to `file_info_edit_fields()` (or the packet) because Description fields are editable rows rather than read-only XMP rows.

### D10 — Fixtures and oracles grow with the data

`scripts/generate-fixtures.py` adds the GPS IFD, Origin/Keywords IIM records, and XMP Description, Origin, IPTC Extension, Photoshop, `xmp:`, and `xmpMM:` properties while keeping the existing unknown `acme:` property. `metadata.psd` is regenerated byte-stably. `metadata_oracle.rs` compares the new values with `exiftool` and proves the save round-trip; engine unit tests prove the same values without `exiftool`. The oracle uses explicit `-XMP-iptcExt:` tags confirmed by `exiftool -listx` and self-skips only when `exiftool` itself is absent.

### D11 — Ceilings recorded in this change

- Video Data, Audio Data, and DICOM tabs are absent. Adding them requires a video/audio layer model and DICOM import, which `ROADMAP.md` lists under "Not planned".
- Artwork Or Object Shown and PLUS nested structs are read-only rows.
- Repeated `LocationShown`/`Licensor` structures beyond the first are read-only.
- Prefix insertion uses a canonical table plus a generated fallback; an exotic packet whose `rdf:Description` cannot be recognised still supports raw replacement but rejects field edits.

## Risks / Trade-offs

- **The generic path patcher can corrupt a packet** → it edits only recorded spans, rejects overlaps, fails closed on unknown shapes, and is covered by fixture round-trips plus unknown-namespace byte comparisons.
- **Two writers (typed and generic) could disagree** → the typed writer remains the only path for the managed scalar/list fields; the generic writer is used only for extension and unknown leaves, and both share the same resource/framing helpers.
- **Expanding `MANAGED` changes Replace-mode templates** → a template written before this change omits the new fields, so applying it in Replace mode clears them. This follows the documented Replace semantics; the template UI and release note mention it.
- **The bridge row grammar changes existing output** → the consumers are `frame.cpp`, the three retired self-test checks, and the metadata-template check 460, which migrates to the new editable rows. The Qt Test locks the new grammar.
- **The fixture grows and could become unstable** → the generator is deterministic and the fixture is checked by hash/git diff.
- **The self-test migration touches a large file** → remove only the three checks and their fixture builder; update check 460's row assertions; lower the budget in the same commit.
- **Long extension forms overflow the dialog** → the form pages sit in a scroll area; the category list stays fixed.

## Migration Plan

1. Extend and regenerate the fixture; existing tests ignore the added properties until their readers land.
2. Land the codec splits and additions with tests; no app behaviour changes yet.
3. Switch the bridge rows and apply dispatch; update `frame.cpp` and the dialog; add `tst_file_info.cpp`.
4. Retire the three self-test checks, migrate check 460, and lower the budget.
5. Update `ROADMAP.md` and the parity register; run the gates, then archive the change with the PR.

Rollback: the change is additive at the file level; reverting the PR restores the four-category dialog and the old row grammar. No document format changes beyond metadata writes that already round-trip.

## Open Questions

None blocking. The exact CS6 category order remains a secondary-source inference in `docs/10-workflow-io/file-info-and-metadata.md`, but the spec fixes the order used here, and the order is a UI constant that can change without touching the codec.
