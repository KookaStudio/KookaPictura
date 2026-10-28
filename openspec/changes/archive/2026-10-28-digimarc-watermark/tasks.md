# Tasks

## 1. Engine: the watermark payload and carrier

- [ ] 1.1 Add `crates/pictura-filters/src/digimarc.rs` with `WatermarkKind`, `Durability`, `WatermarkPayload`, the 66-bit codeword (magic, durability, type, creator ID, ID, CRC-8), the splitmix64 ±1 chip, `embed`, `read`, and `read_with_margin`; declare `pub mod digimarc;` and re-export the types from `lib.rs`; verify `cargo nextest run -p pictura-filters digimarc`.
- [ ] 1.2 Write the in-module unit tests `watermark_round_trips_at_each_durability`, `corrupted_carrier_is_rejected`, `plain_image_has_no_watermark`, `carrier_is_deterministic_and_alpha_safe`, and `higher_durability_has_a_larger_margin`; verify `cargo nextest run -p pictura-filters digimarc`.

## 2. Engine: the Embed Watermark filter row

- [ ] 2.1 Add `Filter::EmbedWatermark { kind, creator_id, id, durability }` in `filter/types.rs`, its row in the `filter_variants!` list, and its dispatch arm in `filter/apply.rs`; verify `cargo nextest run -p pictura-filters catalogue`.
- [ ] 2.2 Add `catalogue/rows/digimarc.rs` with the `EMBED_WATERMARK` row (path `Filter/Digimarc/Embed Watermark`, four params, `whole_layer: true`, `alpha: Never`, `apron: Fixed(16)`, `gpu: CpuPreferred`) and its builder in `catalogue/build/digimarc.rs`; register both modules; add the variant to the `catalogue_rows_match_kind_order` expectation; verify `cargo nextest run -p pictura-filters catalogue`.
- [ ] 2.3 Add the `EmbedWatermark` instance to `tests/common/instances.rs`; verify `cargo nextest run -p pictura-filters properties`.

## 3. App: the bridge and command catalogue

- [ ] 3.1 Add the `WatermarkRow` struct and the `read_watermark(view) -> WatermarkRow` bridge method to `filter_tools.rs`, decoding the active pixel layer's buffer with `pictura_filters::digimarc::read`; verify `cargo nextest run -p pictura-app read_watermark`.
- [ ] 3.2 Flip `filter.digimarc.embed.watermark` and `filter.digimarc.read.watermark` to `implemented: true` in `command_catalogue/entries.rs`; update `crates/pictura-script/tests/golden/catalogue.tsv` and the counts in `crates/pictura-script/tests/catalogue.rs`; verify `cargo nextest run -p pictura-script catalogue`.

## 4. C++ shell: dialog, read command, title indicator

- [ ] 4.1 Add the read-result dialog (`crates/pictura-app/cpp/digimarc_read_dialog.{h,cpp}`) listing the decoded type, creator ID, and ID or "no watermark found"; add both files to `CMakeLists.txt`.
- [ ] 4.2 Register the Read Watermark handler in `frame_menus_filter.cpp` (call `read_watermark`, set the document's detected flag, refresh the title, show the dialog); the Embed row is wired by the existing `wireFilterMenu`.
- [ ] 4.3 Add `bool watermarkDetected` to `DocEntry` and append the copyright symbol in `document_window.cpp::fullLabel`; add a `setWatermarkDetected`/refresh path in `frame.cpp`; verify `ctest --test-dir build -R '^tst_digimarc_watermark$'`.

## 5. Tests

- [ ] 5.1 Add `crates/pictura-app/cpp/tests/tst_digimarc_watermark.cpp` with `embedMenuIsEnabledAndOpens`, `dialogExposesCs6Controls`, `commitRecordsOneStateAndCancelRestores`, `readMenuIsEnabled`, `readReportsEmbeddedPayload`, `readReportsNoWatermark`, `detectedWatermarkMarksTheTitle`, and `undetectedWatermarkLeavesTheTitlePlain`; register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify `ctest --test-dir build -R '^tst_digimarc_watermark$' --output-on-failure`.

## 6. Docs

- [ ] 6.1 Add `docs/06-filters/digimarc-watermark.md` (decision record, scheme, controls, approximation label); update `docs/06-filters/extract-and-pattern-maker.md`'s Digimarc open question, `docs/dev/STATE.md`, `docs/dev/doc-code-parity-gaps.md`, and tick `ROADMAP.md`.

## 7. Verify and archive

- [ ] 7.1 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify both pass.
- [ ] 7.2 Run `bash scripts/verify-fast.sh`; verify it passes.
- [ ] 7.3 Archive with `openspec archive digimarc-watermark`; verify `scripts/check-openspec-archived.sh` passes.
- [ ] 7.4 Commit with `feat(filters): embed and read an approximate in-tree watermark (#350, #351) TASK-ALLOWS-DOCS`, open the PR, and list the test plan's manual rows as a reviewer checklist.
