# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- **Wire codes cross the seam.** `SaveFormatKind` integers (JPEG 1, PNG 2,
  BMP 3, TIFF 4, GIF 5, EPS 6, DCS 7, Raw 8, TGA 9, Cineon 10) cross the
  C++/Rust boundary and are held per document in `save_options`; they must not
  move. Persisted settings use string keys (`saveFormatOptions`) and action
  steps record the `as` suffix, so the catalog adds an ordinal id that is not
  the wire code.
- **The suffix is not the decision.** EPS and DCS share `.eps`; DCS is selected
  by filter hint or remembered kind. DCS 1.0 writes the `.C/.M/.Y/.K` plate
  set; DCS 2.0 (the default) writes one file. A remembered DCS save sets
  `source_format = "dcs"`, a name that is not a suffix.
- **Some Rust callers have no shell.** C++ self-tests call `view->save(path)`
  directly, and the smart-object temp save calls `document_api::save`. The
  control server does not bypass C++: its save operations go through
  `saveActiveAsWithRememberedOptions`/`saveActive`, which resolve the format
  first. Alias resolution therefore cannot live only in C++.
- **Rust cannot call C++ app code.** A table in the shell read by the engine is
  not an option; the direction must be Rust table, C++ query.
- **The route lives in Rust.** `save_with_intent` already owns the branch that
  picks the PSD worker, PDF export, GIF encoder, engine writer, or Qt writer.
  A table anywhere else would be a second route decision.
- **Two GIF writers exist.** `pictura-codec` has a GIF encoder used by
  `save_document` and golden tests; the document route uses the app's
  `write_gif` (indexed-source reuse and palette options). Preserving behavior
  means the catalog records the app route for the document save.

## Goals / Non-Goals

**Goals:**

- One place to read a Save As format: identity, wire code, aliases, canonical
  extension, writer route, and the policy flags callers actually branch on.
- Deleting the parallel switches: position→kind, format→position,
  position→suffix, suffix→kind, native/worker, profile prompt, raster warning,
  and the codec's own alias and writer-format matches.
- A check per displayed format that ties its filter choice to the normalized
  path, the options kind, and the writer route the real dispatcher takes, plus a
  codec matrix that pins each row's `route()`.
- Zero observable behavior change.

**Non-Goals:**

- No new format, no filter reorder or relabel, no encoder or dialog change.
- No unification of the two GIF writers; the document route keeps the app
  encoder.
- No spec delta; the requirements this preserves are already specified.
- No `docs/` edit, no new dependency, no golden-baseline change.

## Decisions

### D1. The catalog lives in `pictura-codec::save`

The engine crate already owns `SaveKind`, the wire codes, the aliases, and the
encoder dispatch, and it has no Qt dependency. The catalog's route values sit
next to the payload writers the codec owns. The app-level routes (`Native`,
`Pdf`, `AppGif`, `QtRaster`) are values the app dispatcher matches; a codec unit
test cannot reach them. The codec test (`save_format_route_matrix`) therefore
pins each row's route value, and the Qt case
`tst_save_format_catalog::eachDisplayedFormatWritesItsRoute` proves every
displayed format reaches its writer through the real dispatcher. The shell
keeps only presentation (order, label, pattern) and queries the catalog.

*Alternatives:* a shell-owned table with the engine taking an explicit kind
would leave `PictureView::save(path)` (C++ self-tests and the smart-object temp
save) needing its own alias resolution, so the duplicate table survives;
rejected. A shared generated table adds build machinery for a 14-row constant;
rejected.

### D2. Add `SaveFormat` above `SaveKind`; do not widen `SaveKind`

`SaveKind` stays the wire-code set the dispatch recognizes, not a claim about
what every encoder accepts: `SaveKind::Jpeg` is recognized so `set_save_options`
can carry JPEG options, but `save_document` refuses it (`JPEG stays on Qt`),
and WebP is engine-encoded outside `SaveKind` (`pictura_codec::encode_webp` /
`encode_webp_lossy`). A new `SaveFormat` covers all 14 displayed Save As
formats, including PSD, PSB, and PDF, which the engine does not encode but the
route and display must distinguish; WebP is engine-encoded but routed through
the app's raster seam. Widening `SaveKind` would add arms to every encoder match
and blur "the engine can write this" with "Save As offers this".

| id | name | code | aliases | canonical | route | flags |
|---|---|---|---|---|---|---|
| 0 | psd | 0 | psd | .psd | Native | native, offers-profile |
| 1 | psb | 0 | psb | .psb | Native | native, offers-profile |
| 2 | png | 2 | png | .png | Engine(Png) | raster |
| 3 | jpeg | 1 | jpg jpeg jpe | .jpg | QtRaster("JPG") | raster, offers-profile |
| 4 | tiff | 4 | tif tiff | .tif | Engine(Tiff) | raster, keeps-layers, offers-profile |
| 5 | webp | 0 | webp | .webp | QtRaster("WEBP") | raster |
| 6 | bmp | 3 | bmp | .bmp | Engine(Bmp) | raster |
| 7 | pdf | 0 | pdf pdp | .pdf | Pdf | pdf |
| 8 | gif | 5 | gif | .gif | AppGif | raster |
| 9 | eps | 6 | eps | .eps | Engine(Eps) | raster |
| 10 | dcs | 7 | (none) | .eps | Engine(Dcs) | raster |
| 11 | raw | 8 | raw | .raw | Engine(Raw) | raster |
| 12 | tga | 9 | tga vda icb vst | .tga | Engine(Tga) | raster |
| 13 | cineon | 10 | cin | .cin | Engine(Cineon) | raster |

The `id` column is the catalog ordinal (0..13); `-1` means unknown. Ids are
never the wire codes — DCS is id 10/code 7, and PSD, PSB, WebP, and PDF share
code 0 — and a test asserts `id != code` where they differ. All bridge queries
and `save_document_format` speak ordinals: `save_format_id_for_suffix`,
`save_format_id_for_source_name`, `save_format_flags`, `save_format_code`,
`save_format_canonical`, and `save_format_aliases`.

`code` 0 means "no per-format options dialog"; it maps to
`SaveFormatKind::None`. The flag set is exactly what callers branch on:
`native` (PSD/PSB worker and `isNativeDocumentPath`), `pdf`, `raster`
(the `isRasterImageSuffix` set), `keeps-layers` (TIFF, the no-dialog and
no-warning exemption), `offers-profile` (PSD, PSB, JPEG, TIFF). DCS's empty
alias set is deliberate: `.dcs` must stay an unknown suffix, and DCS must stay
reachable only by filter hint or remembered kind.

### D3. The document dispatcher matches `route()`

`save_with_intent` resolves a `SaveFormat` from an explicit catalog id when the
shell supplies one (`-1` otherwise) and matches `format.route()`. The native arm
passes that same resolved id to the worker, which resolves it through the shared
`save_format_for_save` helper and chooses PSD versus PSB from it. `SaveKind` is
used only to select the options blob and the encoder. The pdf/pdp, psd/psb, and
gif suffix checks and the final JPEG/WebP guard become route arms. The
EPS-vs-DCS override from the stored kind is not a route arm: plain Save resolves
it in the shell (D5) and passes the id, while the suffix-only entry point
resolves `.eps` to EPS. Refusal for a missing or unknown suffix stays at the
entry point.

*Alternative:* keep the if-chain and use the table only for lookup. That leaves
two sources for the route and fails the issue's deletion test.

### D4. The shell display table references catalog ids

One ordered `constexpr`/static array in `save_format_options.cpp` holds
`{ ordinal id, SaveFormatKind, name, pattern, canonical suffix }` for the
14 formats; "All Formats" stays a display string after them. Positions are the
array indices, so `saveFormatKindForFilter`, `defaultSuffixForFilter`, and
`defaultSaveFilterIndex` are scans, not switches; `defaultSaveFilterIndex` keeps
its `documentRequiresLayers(view) → 0` rule as a view predicate above the scan.
The kind and canonical columns are mirrors of the catalog `code` and canonical,
pinned by `save_format_code(id)` and `save_format_canonical(id)` in the Qt row
test; for rows whose kind is not `None`, `saveFormatKey(kind) == row name`, and
for rows whose kind is `None`, `saveFormatKey(kind)` is empty, both pinned by
the same test.

The name-filter patterns remain display literals. Six read-only queries cross
the bridge, implemented in the existing free-function `export` bridge (not a new
`PictureView` invokable, so the capped declaration list does not grow):

1. `save_format_id_for_suffix(suffix) -> i32` — alias-only, for typed suffixes;
   `-1` when unknown. `.dcs` stays unknown.
2. `save_format_id_for_source_name(name) -> i32` — aliases plus identity names,
   so a remembered `source_format = "dcs"` selects id 10.
3. `save_format_flags(id) -> u32` — the bitmask above.
4. `save_format_code(id) -> i32` — the wire code, so the shell's kind mirror is
   pinned to the catalog.
5. `save_format_canonical(id) -> QString` — the canonical extension, so
   `defaultSuffixForFilter` is pinned to the catalog.
6. `save_format_aliases(id) -> QString` — space-separated lowercase aliases
   (empty for DCS), so the display patterns and All Formats are proved against
   the catalog.

*Alternative:* return the whole catalog as JSON and parse it once in C++. More
machinery than six scalar queries for a 14-row table; rejected.

### D5. The save edges take the resolved format explicitly

`document_api::save` / `saveCopy` and the native `saveStart` / `saveCopyStart` /
`native_compatibility_save_start` bridge gain an optional resolved catalog id
(`-1` = resolve from the path). The coordinator chooses the native route from
that id when supplied and passes it through the background and synchronous save
paths. The worker resolves it with the same `save_format_for_save` helper, so
route selection and the PSD/PSB container choice share one catalog decision. The
format selects the route; the extension guard stays.

Plain Save resolves its format in the shell: `saveActive` starts from
`saveFormatKindForSuffix(suffix)` and lets `lastSaveKinds_` override only the
shared `.eps` suffix when the remembered kind is Eps or Dcs; `saveActiveAs`
then passes that catalog id to `document_api::save`. Save a Copy passes the id
computed by `saveCopyAsWithOptions` through `saveCopyAs` to
`document_api::saveCopy`. This is what preserves `save-options-plain-kind`; the
Rust dispatcher no longer reads `save_options.kind` for routing, though it still
uses it to select the options blob.

The suffix-only `PictureView::save(path)` remains for the direct callers that
have no shell: C++ self-tests and the smart-object temp save. It resolves
through the same catalog, where `.eps` is EPS; DCS is reachable only when the
shell supplies the id.

*Alternative:* trust `save_options.kind` in the Rust dispatcher. That is stale
after an earlier EPS save and a direct `view->save("x.png")`; rejected. The
shell's `lastSaveKinds_` is the intentional per-document source, not the Rust
view state.

### D6. Preserved behaviors and how each is derived

- Missing/unknown suffix refusal: route resolution returns `-1`; the save edge
  refuses before writing.
- Filter order, labels, and the All Formats text: unchanged literals; a test
  asserts the All Formats pattern set equals the union of the catalog aliases
  queried through `save_format_aliases`, and each row's pattern suffixes equal
  its aliases (DCS: pattern `.EPS`, alias set empty).
- Chosen-format precedence: `pathForSaveFilter` keeps its rule; it now reads
  canonical suffixes from the table.
- Default save filter: `documentRequiresLayers(view) → 0` stays a view predicate
  above the catalog scan in `defaultSaveFilterIndex`.
- Plain Save and Save a Copy: the shell resolves the remembered kind
  (`lastSaveKinds_`, `save-options-plain-kind`) and passes the id; a DCS copy
  keeps DCS.
- `.eps` replay and control-server saves: a recorded `.eps` step whose options
  carry the DCS-only `version` field resolves to DCS; otherwise
  `saveActiveAtPath` falls back to `lastSaveKinds_`, and the control server to
  `source_format`, so an `.eps` destination keeps its separation set.
- TIFF layer retention, feature warning, profile prompt, native/worker, PDF:
  flags and `keeps-layers`, with the `rememberedDcs` combination kept in the
  shell because it is per-document state, not a format fact. `lastSaveKinds_` is
  authoritative in the coordinator; the control server has only
  `source_format`, so it uses `save_format_id_for_source_name` rather than a
  literal `"dcs"`.
- Stored wire codes: the catalog reuses the existing numbers; a test pins
  `code` to `SaveKind::code` for every engine kind and to `None` for 0. The
  ordinal ids are pinned separately and never equal the wire code by accident.

## Risks / Trade-offs

- **[Bridge call on path normalization]** `lowercasedImageSuffix`,
  `saveFormatKindForSuffix`, and the policies now cross the seam. These run on
  dialog and save events, not per frame. → Keep the queries scalar and pure; if
  profiling ever shows a hotspot, cache the catalog in C++ at first use.
- **[Display row and catalog drift]** The shell's kind, canonical, and pattern
  columns are mirrors. → `everyDisplayedFormatResolvesItsPathAndKind` asserts
  `saveFormatKindForFilter(i) == save_format_code(id)`,
  `defaultSuffixForFilter(i) == save_format_canonical(id)`, and for rows whose
  kind is not `None`, `saveFormatKey(kind) == row name` (for rows whose kind is
  `None`, `saveFormatKey(kind)` is empty); `allFormatsCoversEveryAlias` compares
  the display patterns against `save_format_aliases`. A Rust test pins ids,
  codes, and aliases.
- **[Route values and the dispatcher]** The codec matrix pins each row's
  `route()`, but it cannot prove the app's match arm. → The Qt case
  `eachDisplayedFormatWritesItsRoute` saves one tiny document per displayed row
  through the common save leaf (with profile/GIF/format-option stubs) and asserts the
  written signature, container version, or plate/header detail, so the real
  dispatcher is exercised without the codec depending on the app.
- **[DCS aliasing regression]** Adding `dcs` as an alias would accept a new
  suffix. → The catalog keeps the alias set empty and a test asserts
  `save_format_id_for_suffix("dcs")` is `-1` while the source-name lookup finds
  DCS.
- **[File-size ceilings]** `cxxqt_object/tests.rs` is at 1397/1400 lines. →
  Put the bridge-query tests in a new `#[cfg(test)] mod tests` in
  `save_format/imp.rs` and `explicit_format_selects_the_route` in the existing
  `impl_core/save.rs` test module. `dialogs.cpp` and `save/mod.rs` remain below
  their caps, so no allowlist entry is added.

## Migration Plan

1. Add the catalog and its Rust tests (ids/codes/aliases, route matrix); nothing
   consumes it yet (behavior unchanged).
2. Add the six bridge queries and `save_format_queries_match_the_catalog` in
   `save_format/imp.rs`; add `explicit_format_selects_the_route` in
   `impl_core/save.rs`.
3. Rewrite the shell helpers as table lookups (layered rule, `saveFormatKey`),
   keeping the public signatures the existing tests use; run the named suites.
4. Switch `save_with_intent` to `route()`, resolve plain Save in `saveActive`,
   and pass the id from `saveActiveAs` and `saveCopyAs`.
5. Delete the parallel decisions; rerun every named suite and both self-test
   runs.

Each step compiles and leaves the existing behavior in place, so a regression
bisects to one step. Rollback is a revert of the whole change; no stored data
format changes.

## Open Questions

None.
