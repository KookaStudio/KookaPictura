# Tasks

Status: implemented, verified, and archived. The CMake/Ninja build, formatting,
clippy, all 6,855 Rust tests, the new catalog suite, both headless self-test
invocations, and the scoped guards passed. The aggregate gate has two proven
baseline failures, recorded below.

## 1. Codec format catalog

- [x] 1.1 Add `crates/pictura-codec/src/save/format.rs` with `SaveFormat` (14 rows: ordinal `id` 0..13, `name`, wire `code`, aliases, canonical extension, route, flags), `SaveRoute`, the flag constants, and the id/name/suffix/source-name/code/route/flag/canonical/alias lookups; declare the module and re-export it from `save/mod.rs`. Ids are ordinals (`-1` = unknown) and never the wire codes. Verify `bash scripts/with-build-lock.sh -- cargo build -p pictura-codec` succeeds and the table is the only alias list in the crate.
- [x] 1.2 Add the catalog unit tests named in `test-plan.md`: `save_format_catalog_matches_wire_codes` (also asserts `id != code` where they differ: DCS 10/7; PSD, PSB, WebP, PDF code 0), `save_format_suffix_aliases_are_case_insensitive`, `dcs_has_no_suffix_alias`, `save_format_source_names_resolve`. Verify `bash scripts/with-build-lock.sh -- cargo test -p pictura-codec save_format` passes and the four `fn` names exist.
- [x] 1.3 Add `save_format_route_matrix` asserting each row's `route()` value (route values only; the app dispatcher is proved by the Qt case in 3.2). Verify `bash scripts/with-build-lock.sh -- cargo test -p pictura-codec save_format` passes.

## 2. Bridge queries and the explicit save format

- [x] 2.1 Add the six read-only queries `save_format_id_for_suffix`, `save_format_id_for_source_name`, `save_format_flags`, `save_format_code`, `save_format_canonical`, and `save_format_aliases` as free functions in `save_format/imp.rs`, declared in the `export` bridge (all take/return ordinal ids; `-1` unknown). Put `pictura-app::save_format_queries_match_the_catalog` in a new `#[cfg(test)] mod tests` in `save_format/imp.rs`, covering a known alias, `.dcs` as `-1`, `"dcs"` as source name 10, one flag set, one code, one canonical, and one alias set. Verify `bash scripts/with-build-lock.sh -- cargo test -p pictura-app save_format`.
- [x] 2.2 Add a `save_document_format` free bridge function taking an explicit ordinal format id (`-1` resolves from the path) and the copy flag, route `save_with_intent` through an optional resolved format, and give `document_api::save`/`saveCopy` an id parameter that defaults to "resolve from the path". Put `pictura-app::explicit_format_selects_the_route` in the existing `impl_core/save.rs` test module; it proves an explicit id wins over the suffix, a suffix-only call resolves, and an explicit id cannot carry an unknown suffix. Verify `bash scripts/with-build-lock.sh -- cargo test -p pictura-app explicit_format`.

## 3. Shell display table and policy lookups

- [x] 3.1 Replace the position and suffix switches in `save_format_options.{h,cpp}` with the ordered display table (ordinal id, kind, name, pattern, canonical suffix) and the six bridge queries; derive `saveFormatKey` from the row name (empty for `SaveFormatKind::None`) and pin it in 3.2's row test; add the pure predicates `saveFormatIsNativeSuffix`, `saveFormatKeepsLayers`, `saveFormatOffersProfile`, and `saveFormatIsRasterSuffix`. Keep the existing public function names and signatures. Verify `tst_save_format_options::formatKindForSuffix`, `filterHintChoosesDcs`, and `filterHintChoosesEps` still pass. No new shell C++ file is added; the new `tst_save_format_catalog.cpp` suite is registered in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 3.2 Add the table-driven cases `tst_save_format_catalog::everyDisplayedFormatResolvesItsPathAndKind` (assert `saveFormatKindForFilter(i) == save_format_code(id)`, `defaultSuffixForFilter(i) == save_format_canonical(id)`, and for rows whose kind is not `None`, `saveFormatKey(kind) == row name`; for rows whose kind is `None`, `saveFormatKey(kind)` is empty), `tst_save_format_catalog::allFormatsCoversEveryAlias` (compare the All Formats and each row's pattern suffixes against `save_format_aliases`; DCS pattern `.EPS` with empty aliases), `tst_save_format_catalog::profilePromptFollowsTheCatalog`, `tst_save_format_catalog::eachDisplayedFormatWritesItsRoute` (save one tiny document per displayed row through `saveActiveAsWithOptions` with profile/GIF stubs; set `setNativeSaveCompatibility(Always)`, the default, explicitly so no merged-image prompt appears; assert the PSD/PSB version word, DCS plates, EPS no-plates, RAW header, and TGA image type), and `tst_save_format_catalog::saveAsDialogRefusesUnknownSuffix` (All Formats with a typed `.xyz`; no file written). Put them in the new `tst_save_format_catalog.cpp` suite, registered next to `tst_save_format_options`, and make them pass. Verify `cmake --build build --target tst_save_format_catalog` then `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_save_format_catalog$' --output-on-failure`.
- [x] 3.3 Rewrite the view-dependent policies in `dialogs.cpp` — `saveFileFilters`, `defaultSaveFilterIndex`, `isKnownImageSuffix`, `isRasterImageSuffix`, `savePathNeedsFormatDialog`, `saveAsNeedsFeatureWarning`, `pathForSaveFilter` — as table lookups and delete their literal suffix lists and position switches; keep `defaultSaveFilterIndex`'s `documentRequiresLayers(view) → 0` rule as a view predicate above the catalog scan. Verify `tst_open_save::chosenFormatWinsOverTypedSuffix`, `tst_open_save::layeredRasterDefaultsToPsd`, `tst_save_pdf::saveAsListsPhotoshopPdf`, `tst_save_gif_copy::gifFilterOffered`, and the `tst_offthread_jobs` filter loop pass.
- [x] 3.4 Derive `confirmSaveProfile`, `isNativeDocumentPath`/`saveRunsOnWorker` in `import_save_open.cpp`, `isPdfPath` in `import_save_pdf.cpp` (catalog `pdf` flag), and the remembered-DCS combination in `import_save_save.cpp` and `control_server_documents.cpp` (`save_format_id_for_source_name`; `lastSaveKinds_` stays authoritative in the coordinator, while `source_format` is the control server pre-check's only source) from the catalog flags. Verify `tst_save_options::saveKeepsChoice`, `tst_save_gif_copy::saveCopyProfileDialog`, and `tst_save_format_options::plainSaveLayeredTiff` pass.

## 4. Document save dispatch

- [x] 4.1 Change `save_with_intent` to resolve a `SaveFormat` (explicit id first, suffix second) and match `route()` instead of the prefix if-chain. Resolve plain Save in `saveActive` from the suffix kind, with `lastSaveKinds_` overriding only the shared `.eps` suffix, and pass the id from `saveActiveAs` to `document_api::save`; pass `saveCopyAsWithOptions`'s id through `saveCopyAs` to `document_api::saveCopy`. Write `tst_save_format_catalog::dcsCopyKeepsDcs` proving a DCS Save a Copy stays DCS (assert the `.C` plate file or the recorded source format), and `tst_save_format_catalog::recordedDcsStepKeepsDcs` proving a replayed `.eps` step carrying the DCS-only `version` option writes the plate set. Verify `tst_save_format_catalog::eachDisplayedFormatWritesItsRoute`, `tst_save_format_options::plainSaveKind`, `tst_save_format_catalog::dcsCopyKeepsDcs`, `tst_save_format_catalog::recordedDcsStepKeepsDcs`, `pictura-app::explicit_format_selects_the_route`, and the preserved `tst_save_*` suites pass.
- [x] 4.2 Rewrite `pictura-app::output_format_classification_matches_suffix` to assert catalog lookups instead of the deleted suffix helper. Verify `bash scripts/with-build-lock.sh -- cargo test -p pictura-app output_format`.

## 5. Delete the parallel decisions and integrate

- [x] 5.1 Delete `SaveKind::from_suffix`, `raster_writer_for_suffix`, the EPS override in the dispatcher, and the remaining literal Save As suffix/position lists. Do not touch the open/stack lists (`openFileFilters`, `stackSourceSuffixes`), whose membership differs. Verify `rg -n 'from_suffix|raster_writer_for_suffix' crates/` finds only catalog-internal uses and `bash scripts/with-build-lock.sh -- cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] 5.2 Run the named Qt suites together and both self-test invocations; compare the self-test summary and exit codes with the pre-change run. Verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(open_save|save_format_options|save_options|save_gif_copy|save_pdf)' --output-on-failure` and both `./build/pictura --headless --self-test` invocations with unchanged results.
- [x] 5.3 Archive the change before the aggregate gate: run `openspec archive save-as-format-resolution` and verify `bash scripts/check-openspec-archived.sh` passes.
- [x] 5.4 Run the aggregate gate and confirm scope: `bash scripts/verify-fast.sh`, `python3 scripts/check-test-plan.py`, no new `scripts/file-size-allowlist.txt` entry, and no `ROADMAP.md`/`PARITY.md`/`docs/` change because no feature coverage moved.

## Verification record

- Rebased onto `origin/master` at `d593663a9`; the tracked patch and untracked
  archive survived the rebase unchanged apart from line offsets.
- `cargo fmt --all` and the requested CMake/Ninja build passed. The first build
  attempt stopped on stale Unix Makefiles caches; clearing those generated caches
  fixed the generator mismatch.
- `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh`
  passed formatting, clippy, 6,855 Rust tests, both self-test runs, and 309 of 310
  Qt entries. Its only test failure was
  `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`, reproduced on a clean
  `origin/master` checkout.
- The checks after the test step passed when run directly, except
  `check-milestone-names.py --self-test`, which also fails on clean
  `origin/master`. `openspec validate --all --strict` passed.
- `tst_save_format_catalog` passed all eight test cases. Both headless self-test
  invocations matched clean `origin/master`: 467 checks without a document and
  502 with `two_layers.psd`, all passing.
- `ROADMAP.md` and `PARITY.md` were reviewed and left unchanged. No feature
  coverage moved.
